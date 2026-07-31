# Storage Phase 3 Implementation Guide

## Purpose

This document specifies the implementation work for Storage Phase 3.
Phase 3 extends the existing single-node in-memory storage engine with record metadata, optional TTL, an expiration worker, and in-process metrics.

Each section below is a self-contained feature block.
A feature is considered fully specified if and only if the information inside its block is present.
No section should be read in order or assumed from another section.

---

## Feature Block 1: Record Metadata

### Goal

Add creation time, last-update time, and optional expiry time to every stored value.

### Current state

`Record<V>` exists in `storage/src/rec/record.rs`.
It currently contains:

- `value: Arc<V>`
- `version: u64`

### Target state

`Record<V>` must contain:

- `value: Arc<V>`
- `version: u64`
- `created_at: Instant`
- `updated_at: Instant`
- `expires_at: Option<Instant>`

### Why use `Instant`

`Instant` is monotonic and not affected by system clock changes.
This makes local TTL behavior predictable.
Wall-clock time may be added later for cross-node replication.

### Constructors

#### `Record::new(value: V) -> Record<V>`

Creates a record with:

- `version` set to `1`
- `created_at` set to `Instant::now()`
- `updated_at` set to the same `Instant`
- `expires_at` set to `None`

#### `Record::with_ttl(value: V, ttl: Duration) -> Record<V>`

Creates a record with:

- `version` set to `1`
- `created_at` set to `Instant::now()`
- `updated_at` set to the same `Instant`
- `expires_at` set to `Some(Instant::now() + ttl)`

#### `Record::next(value: V, previous_version: u64, previous_created_at: Instant, ttl: Option<Duration>) -> Record<V>`

Creates a new record from an update with:

- `version` set to `previous_version + 1`
- `created_at` preserved from the previous record
- `updated_at` set to `Instant::now()`
- `expires_at` set to `Some(Instant::now() + ttl)` if `ttl` is `Some`, otherwise `None`

### Expiry helpers

Add an inherent method:

```rust
impl<V> Record<V> {
    /// Returns true if the record has expired relative to `now`.
    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.map_or(false, |expires| now >= expires)
    }
}
```

### `Clone` behavior

`Clone` must copy `value`, `version`, `created_at`, `updated_at`, and `expires_at` exactly.
Do not reset timestamps on clone.

### Files to change

- `storage/src/rec/record.rs`

---

## Feature Block 2: TTL in Storage Operations

### Goal

Allow `insert` and `update` to accept an optional TTL so records can expire automatically.

### Current state

`Storage<K, V>` in `storage/src/traits.rs` defines:

- `fn insert(&self, key: K, value: V);`
- `fn update(&self, key: K, value: V);`

`Collection<K, V>` in `storage/src/rec/collections.rs` implements these using `Record::new` and `Record::next`.

### Target state

Extend the trait with TTL-aware methods while keeping backward-compatible defaults.

#### Trait changes

```rust
use std::time::Duration;

pub trait Storage<K, V>
where
    K: Eq + Hash,
{
    fn get(&self, key: &K) -> Option<Arc<V>>;

    fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, None);
    }

    fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>);

    fn remove(&self, key: &K) -> Option<Arc<V>>;

    fn update(&self, key: K, value: V) {
        self.update_with_ttl(key, value, None);
    }

    fn update_with_ttl(&self, key: K, value: V, ttl: Option<Duration>);

    fn contains(&self, key: &K) -> bool;
}
```

Default trait methods call the `_with_ttl` variants with `None`.
Existing implementations and callers that do not care about TTL continue to compile.

#### `Collection` implementation

```rust
use std::time::{Duration, Instant};

impl<K, V> Storage<K, V> for Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: Send + Sync,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        let now = Instant::now();
        self.map
            .get(key)
            .filter(|entry| !entry.is_expired(now))
            .map(|entry| Arc::clone(&entry.value))
    }

    fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let record = match ttl {
            Some(duration) => Record::with_ttl(value, duration),
            None => Record::new(value),
        };
        self.map.insert(key, record);
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        self.map.remove(key).map(|(_, entry)| Arc::clone(&entry.value))
    }

    fn update_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        match self.map.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let old = occupied.get().clone();
                let next = Record::next(value, old.version, old.created_at, ttl);
                occupied.insert(next);
            }
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                let record = match ttl {
                    Some(duration) => Record::with_ttl(value, duration),
                    None => Record::new(value),
                };
                vacant.insert(record);
            }
        }
    }

    fn contains(&self, key: &K) -> bool {
        let now = Instant::now();
        self.map
            .get(key)
            .map(|entry| !entry.is_expired(now))
            .unwrap_or(false)
    }
}
```

Lazy expiry: `get` and `contains` return `None`/`false` for expired records even before the expiration worker removes them.

### Files to change

- `storage/src/traits.rs`
- `storage/src/rec/collections.rs`

---

## Feature Block 3: Expiration Worker

### Goal

Periodically remove records whose `expires_at` time has passed.

### Scope

This block covers a simple interval-scan worker.
A heap-based scheduler is a separate future optimization and is not part of this block.

### Requirements

- Run at a configurable interval.
- Scan every key in every table.
- Remove records where `record.is_expired(now)` is true.
- Do not block writers or readers for longer than a single `DashMap` shard lock.
- Operate on the `MemoryStore` registry, which holds all tables.

### Design

Introduce a new module `storage/src/workers/expiration.rs`.

The worker owns an `Arc<MemoryStore<String, Vec<u8>>>` and a `Duration` interval.

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

use crate::memory::store::MemoryStore;

pub struct ExpirationWorker;

impl ExpirationWorker {
    pub fn spawn(
        store: Arc<MemoryStore<String, Vec<u8>>>,
        interval_duration: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            loop {
                ticker.tick().await;
                let now = Instant::now();

                for table_entry in store.map.iter() {
                    let table = table_entry.value();
                    for mut item in table.map.iter_mut() {
                        if item.is_expired(now) {
                            let key = item.key().clone();
                            drop(item);
                            table.map.remove(&key);
                        }
                    }
                }
            }
        })
    }
}
```

Notes:

- `MemoryStore` exposes its internal `map` field to this worker through crate-private visibility.
- Each iteration locks one shard at a time because `iter_mut` yields shard-local iterators.
- The worker never panics. Errors are silently ignored because there is no recoverable failure mode here.

### Configuration

For Phase 3, the interval can be a constant or passed into `storage/src/main.rs`.
A default of 1 second is reasonable for development.

### Starting the worker

In `storage/src/main.rs`, after creating the `MemoryStore` and before accepting connections, spawn the worker:

```rust
let store = Arc::new(MemoryStore::<String, Vec<u8>>::new());
let _expiration_handle = ExpirationWorker::spawn(store.clone(), Duration::from_secs(1));
```

### Files to add or change

- Add `storage/src/workers/expiration.rs`
- Add `storage/src/workers/mod.rs`
- Export `workers` from `storage/src/lib.rs` if public access is desired.
- Change `storage/src/main.rs` to spawn the worker.
- Ensure `MemoryStore.map` is visible within the crate.

---

## Feature Block 4: In-Process Metrics

### Goal

Expose atomic counters for reads, writes, deletes, hits, and misses.

### Current state

No metrics exist in the storage engine.

### Metrics struct

Create `storage/src/metrics.rs`:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    reads: AtomicU64,
    writes: AtomicU64,
    deletes: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MetricsSnapshot {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub hits: u64,
    pub misses: u64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_read(&self, hit: bool) {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_write(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_delete(&self) {
        self.deletes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            deletes: self.deletes.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}
```

All counters use `Ordering::Relaxed` because they are statistical, not synchronization primitives.

### Integrating metrics into `Collection`

`Collection<K, V>` holds an `Arc<Metrics>`.

```rust
pub struct Collection<K, V>
where
    K: Eq + Hash,
{
    map: DashMap<K, Record<V>>,
    metrics: Arc<Metrics>,
}
```

`Collection::new(metrics: Arc<Metrics>)` takes the metrics instance.

`get` records a read:

```rust
fn get(&self, key: &K) -> Option<Arc<V>> {
    let now = Instant::now();
    let result = self.map.get(key).filter(|entry| !entry.is_expired(now));
    self.metrics.record_read(result.is_some());
    result.map(|entry| Arc::clone(&entry.value))
}
```

`insert_with_ttl` records a write:

```rust
fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
    let record = match ttl {
        Some(duration) => Record::with_ttl(value, duration),
        None => Record::new(value),
    };
    self.map.insert(key, record);
    self.metrics.record_write();
}
```

`update_with_ttl` records a write.
`remove` records a delete.
`contains` records a read.

### Integrating metrics into `MemoryStore`

`MemoryStore<K, V>` holds an `Arc<Metrics>`.
`create(table)` passes a clone of that `Arc<Metrics>` into every new `Collection`.

```rust
pub struct MemoryStore<K, V>
where
    K: Eq + Hash + Send + Sync,
    V: Send + Sync,
{
    map: DashMap<K, Arc<Collection<K, V>>>,
    metrics: Arc<Metrics>,
}

impl<K, V> MemoryStore<K, V> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            metrics: Arc::new(Metrics::new()),
        }
    }

    pub fn metrics(&self) -> Arc<Metrics> {
        Arc::clone(&self.metrics)
    }
}
```

### Metrics API on the wire

Add to `storage/src/api/requests.rs`:

```rust
#[derive(Debug, ByteSerializable)]
pub struct MetricsRequest {}

#[derive(Debug, ByteSerializable)]
pub struct MetricsResponse {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub hits: u64,
    pub misses: u64,
}
```

Add `MetricsHandler` in the `storage/src/api/` module (for example, alongside
the other handlers in `storage/src/api/table.rs` or in a dedicated
`storage/src/api/metrics.rs`):

```rust
pub struct MetricsHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for MetricsHandler {
    async fn call(&self, _payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let snapshot = self.store.metrics().snapshot();
        Ok(encode(&MetricsResponse {
            reads: snapshot.reads,
            writes: snapshot.writes,
            deletes: snapshot.deletes,
            hits: snapshot.hits,
            misses: snapshot.misses,
        }))
    }
}
```

Register the action in `storage/src/api/server.rs`:

```rust
actions.register_action("metrics", MetricsHandler { store: store.clone() });
```

`SharedStore` must expose `metrics()`.

### Files to add or change

- Add `storage/src/metrics.rs`
- Update `storage/src/lib.rs` to re-export `Metrics` and `MetricsSnapshot`.
- Update `storage/src/traits.rs` to add `_with_ttl` methods.
- Update `storage/src/rec/collections.rs` to hold metrics and record events.
- Update `storage/src/memory/store.rs` to own and distribute metrics.
- Update `storage/src/api/requests.rs` to add `MetricsRequest`/`MetricsResponse`.
- Update `storage/src/api/table.rs` (or add a dedicated `metrics.rs`) to add `MetricsHandler`.
- Update `storage/src/api/server.rs` to register `metrics` action.

---

## Feature Block 5: Wire Protocol Updates for TTL

### Goal

Allow clients to specify a TTL when calling `put` or `update`.

### Current state

`PutRequest` and `UpdateRequest` contain `table`, `key`, and `value`.

### Target state

Add an optional TTL field.

```rust
#[derive(Debug, ByteSerializable)]
pub struct PutRequest {
    pub table: String,
    pub key: String,
    pub value: Vec<u8>,
    pub ttl_seconds: Option<u32>,
}

#[derive(Debug, ByteSerializable)]
pub struct UpdateRequest {
    pub table: String,
    pub key: String,
    pub value: Vec<u8>,
    pub ttl_seconds: Option<u32>,
}
```

### `byteser` compatibility

If `byteser_derive` supports `Option<T>`, use `Option<u32>`.
If it does not, replace the field with:

```rust
pub ttl_seconds: u64,
```

and treat `0` as "no TTL".

### Handler changes

In `PutHandler::call`:

```rust
let ttl = request.ttl_seconds.map(|seconds| Duration::from_secs(seconds as u64));
let table = self.store.create(&request.table);
table.insert_with_ttl(request.key, request.value, ttl);
```

In `UpdateHandler::call`:

```rust
let ttl = request.ttl_seconds.map(|seconds| Duration::from_secs(seconds as u64));
let table = self.store.get(&request.table).ok_or_else(|| ...)?;
table.update_with_ttl(request.key, request.value, ttl);
```

### Validation

A `ttl_seconds` of zero must be treated as "no TTL" regardless of whether `Option<u32>` or `u64` is used.
If `Option<u32>` is used, `Some(0)` must be normalized to `None` before creating the record.

### Files to change

- `storage/src/api/requests.rs`
- `storage/src/api/table.rs` (update `PutHandler`/`UpdateHandler`) or a new handler file

---

## Feature Block 6: Server Bootstrap Updates

### Goal

Start the storage server with the expiration worker and the metrics action enabled.

### Current state

`storage/src/main.rs` creates a `MemoryStore`, registers handlers, and listens on `127.0.0.1:7878`.

### Target state

After creating the store, spawn the expiration worker.
Register the `metrics` action.

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store: Arc<MemoryStore<String, Vec<u8>>> = Arc::new(MemoryStore::new());

    let _expiration_handle =
        workers::expiration::ExpirationWorker::spawn(store.clone(), Duration::from_secs(1));

    let mut actions = Actions::new();
    actions.register_action("get", GetHandler { store: store.clone() });
    actions.register_action("put", PutHandler { store: store.clone() });
    actions.register_action("update", UpdateHandler { store: store.clone() });
    actions.register_action("delete", DeleteHandler { store: store.clone() });
    actions.register_action("contains", ContainsHandler { store: store.clone() });
    actions.register_action("add_table", AddTableHandler { store: store.clone() });
    actions.register_action("remove_table", RemoveTableHandler { store: store.clone() });
    actions.register_action("metrics", MetricsHandler { store: store.clone() });

    let actions = Arc::new(actions);
    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let actions = actions.clone();
        tokio::spawn(async move {
            ResilientServer::new(socket, peer_addr, actions).run().await;
        });
    }
}
```

### Files to change

- `storage/src/main.rs`
- `storage/src/api/server.rs` if action registration is encapsulated there.

---

## Feature Block 7: Test Coverage

### Goal

Verify Phase 3 behavior.

### Unit tests in `storage/src/rec/collections.rs`

Add a `#[cfg(test)]` module with the following tests:

1. `insert_with_ttl_expires`
   - Insert a key with a 50 ms TTL.
   - Assert `contains` returns true immediately.
   - Wait 100 ms.
   - Assert `contains` returns false.

2. `update_resets_expiry_and_bumps_version`
   - Insert a key with no TTL.
   - Update it with a TTL.
   - Wait longer than the original TTL but less than the new TTL.
   - Assert the key still exists and its version is `2`.

3. `get_returns_none_after_expiry`
   - Insert a key with a 50 ms TTL.
   - Wait 100 ms.
   - Assert `get` returns `None`.

4. `metrics_count_reads_and_writes`
   - Create a collection with metrics.
   - Call `get` on a missing key.
   - Insert a key.
   - Call `get` on the existing key.
   - Assert the metrics snapshot shows `reads: 2`, `misses: 1`, `hits: 1`, `writes: 1`.

5. `metrics_count_deletes`
   - Insert a key.
   - Remove it.
   - Assert `deletes: 1`.

### Integration test in `storage/tests/put_get.rs`

Add a test `put_with_ttl_expires`:

- Start the storage server.
- Send a `PutRequest` with `ttl_seconds: Some(1)`.
- Send a `GetRequest` immediately and assert the value is present.
- Wait 1.1 seconds.
- Send a `GetRequest` and assert the value is absent.

Add a test `metrics_are_queryable`:

- Start the storage server.
- Send a `MetricsRequest`.
- Assert the response decodes successfully and counters are all zero.
- Send a `GetRequest` for a missing key.
- Send a `MetricsRequest`.
- Assert `reads` and `misses` are non-zero.

### Files to add or change

- `storage/src/rec/collections.rs`
- `storage/tests/put_get.rs`

---

## Feature Block 8: Out of Scope for Phase 3

The following items are explicitly not part of Phase 3.
If a feature is not listed in Feature Blocks 1 through 7, it is not part of Phase 3.

- TLS/mTLS for storage traffic.
- Persistence, WAL, AOF, snapshots.
- Replication and distributed routing.
- Secondary indexes.
- Transactions, MVCC, pub/sub, streams.
- Compression or encryption of values.
- Bloom filters, LRU/LFU eviction, multi-version snapshots.
- Heap-based TTL scheduler.
- Memory-usage and average-latency metrics.
- `trench` node bootstrap integration.

These items belong to Phase 4 or later work.

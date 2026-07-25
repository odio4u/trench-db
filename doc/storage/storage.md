# In-Memory Storage Engine Design
**Version:** 1.2  
**Goal:** Build a generic, production-grade, Redis-inspired in-memory datastore optimized for read-heavy workloads.

Network communication is **not** reimplemented here. Storage nodes are exposed to clients over the existing
[`transport`](../transport/README.md) crate — its `StreamManager`, `ResilientClient`/`ResilientServer`, and
`Actions`/`Handler` routing. See [Communication Layer](#communication-layer) below.

---

# Current Scope & Status

This section reflects the actual state of the repository (2026-07-23), separating what already exists from what
this document still only designs. Everything after this section is the **target design** the phased plan below
works toward — treat it as the destination, not the current state.

## What already exists

| Crate | State | Notes |
|---|---|---|
| `transport` | Implemented, no TLS yet | `frame/` (TRNC header, encode/decode), `tcp/` (`Connection<T>`, `Stream`, `StreamManager<T>`, `receiver`), `client::resilient_client::ResilientClient`, `server::{ResilientServer, Dispatcher, Actions, Handler}`. Full gap list in [`architecture.md §13`](../transport/architecture.md#13-what-is-not-implemented-yet): TLS/mTLS, configurable timeouts, and back-pressure wake-up are all still **planned**, not implemented. |
| `interface` | Implemented, example only | `EchoHandler` + `UserMessage`/`ServerResponse` demo wired through `ResilientServer`/`ResilientClient`. This is the reference pattern `storage` copies, not production code. |
| `storage` | **Phases 1 & 2 done** | `traits::Storage<K, V>` + `Table<K, V>`, `rec::Record<V>`, `memory::MemoryStore`, and the full `api/` module (`requests`, `handlers`, `server`) are implemented. `storage/src/main.rs` is a real TCP server. Phase 3 (metadata, TTL, metrics) is next. |
| `trench` | **Skeleton** | `config::loader::Node` parses a flat `key=value` file (`config.trench`) into a `Node` struct. `auth::identity` is an empty file. `neighbors/` is an empty folder. Nothing in `trench` calls into `storage` or `transport` yet. |

## Explicitly out of scope for now

- TLS/mTLS for storage traffic — blocked on `transport`'s own TLS work landing first; storage will inherit it for
  free once available and should not build a parallel solution.
- Replication, snapshots, persistence (AOF/WAL), compaction, secondary indexes, MVCC, transactions, pub/sub — these
  remain **future features** (see that section below); none are scheduled in the phases below.
- The DHT / regional routing layer described in [`prd.md`](prd.md) — a separate, later effort layered on top of a
  working single-node store, not part of this plan.

## Phased plan

### Phase 1 — Minimal single-node store (no networking)
- Add `storage::traits::Storage<K, V>`, a `Record<V>` struct, and a `DashMap`-backed `MemoryStore` implementing
  `get`/`insert`/`remove`/`update`/`contains`.
- Unit tests only; `main.rs` stays a placeholder.
- New `storage/Cargo.toml` dependencies: `dashmap`, `byteser`, `byteser_derive`.
- **Exit criteria:** `cargo test -p storage` passes; no `unwrap`/`panic` on the hot (read) path.

### Phase 2 — Wire the store to `transport`
- Add a `storage::api` module: one request/response struct pair per operation (`GetRequest`/`GetResponse`, etc.)
  and one `Handler` impl per action (`get`, `put`, `update`, `delete`, `contains`, `add_table`, `remove_table`),
  registered on `transport::server::Actions` — see [Communication Layer](#communication-layer) for the exact mapping.
- `storage/Cargo.toml` gains `transport = { path = "../transport" }`, `tokio`, `async-trait` — matching
  `interface/Cargo.toml`.
- `storage/src/main.rs` becomes a real binary: binds a `TcpListener` and runs `ResilientServer` per accepted
  connection, mirroring `interface/src/server.rs` exactly.
- **Exit criteria:** a `ResilientClient` (in a test or the `interface`-style client binary) can `put` then `get` a
  key over a real TCP connection to the `storage` binary.

### Phase 3 — Metadata, TTL, metrics
- Flesh out `Record` with `version`/`created_at`/`updated_at`/`expires_at`.
- Add an expiration worker (a simple interval scan is acceptable before the min-heap optimization).
- Add atomic counters for read/write/hit/miss counts.
- **Exit criteria:** keys inserted with a TTL are observably gone after expiry; metrics are queryable in-process.

### Phase 4+ — Everything else in this document
- Secondary indexes, snapshot/replication workers, TTL min-heap, index rebuilder, statistics/event workers, and
  `trench` integration (config-driven node bootstrap, populating `neighbors`) all come after Phase 3 and are
  intentionally not scheduled yet — do not start them before Phases 1–3 land.

---

# Design Goals

This storage engine is intended for:

- Generic key-value storage
- Millions of reads/sec
- Thousands of writes/sec
- Low latency (<1ms read path)
- Horizontal scaling handled externally
- Single process per node
- Thread-safe
- Extensible architecture
- Future support for persistence and replication
- Network access exclusively via the `transport` crate — no bespoke socket/framing code in storage

This is **not** just a wrapper around `HashMap`. It is a complete storage engine.

---

# Target Workload

| Operation | Expected Load |
|------------|--------------|
| Reads | Millions/sec |
| Writes | ~1000/sec |
| Updates | Low |
| Deletes | Low |
| Memory | RAM Only |
| Latency | Very Low |

The workload is heavily read dominated.

---

# High-Level Architecture

```
                       Remote Client
                            │
        transport::client::ResilientClient
                            │
              TCP + TRNC Framing  (transport crate)
                            │
        transport::server::ResilientServer
           Dispatcher → Actions → Handler::call()
                            │
                 Storage API Handlers
                            │
      GET / PUT / DELETE / UPDATE / QUERY
                            │
                            ▼
                 Storage Engine Interface
                            │
     ┌──────────────────────┼──────────────────────┐
     │                      │                      │
Primary Index        Secondary Indexes        Metadata
     │                      │                      │
     └───────────────┬──────┴───────────────┬──────┘
                     ▼                      ▼
              Memory Store           Background Workers
```

Everything above the "Storage API Handlers" line already exists in the `transport` crate (see
[`doc/transport/README.md`](../transport/README.md)) and is reused as-is — the storage engine only adds the
Handler implementations that translate a `RequestEnvelope` into a `Storage` trait call.

---

# Project Structure

```
storage/src/
│
├── lib.rs                ← crate root, re-exports MemoryStore, Record, Storage
├── main.rs               ← TCP server binary
├── traits.rs             ← Storage<K, V> and Table<K, V> traits
│
├── rec/
│   ├── mod.rs
│   ├── record.rs         ← Record<V> { value, version }
│   └── collections.rs    ← Collection<K, V>: Storage over DashMap<K, Record<V>>
│
├── memory/
│   ├── mod.rs
│   └── store.rs          ← MemoryStore<K, V>: Table over DashMap<K, Arc<Collection<K, V>>>
│
├── api/
│   ├── mod.rs
│   ├── handlers.rs       ← transport::server::Handler impls (get/put/update/delete/contains/add_table/remove_table)
│   ├── requests.rs       ← byteser request/response structs per action
│   └── server.rs         ← wires Actions + ResilientServer, mirrors interface/src/server.rs
│
└── (workers/, persistence/, replication/ are Phase 4+ placeholders)
```

The `api/` module is the **only** place this crate touches networking, and it does so purely by depending on the
`transport` crate (added as a path dependency in `storage/Cargo.toml`, same as `interface` does today) — no raw
sockets, no custom framing.

---

# Core Components

## 1. Storage Engine

Responsible for:

- CRUD operations
- Thread safety
- Index management
- Metadata updates
- Worker coordination

The storage engine contains no business logic.

---

## 2. Generic Store

```text
Store<K, V>
│
├── Primary Index
├── Secondary Indexes
├── Metadata
├── Metrics
└── Workers
```

Requirements:

```
K: Eq + Hash + Clone
V: Send + Sync
```

---

## 3. Record Layout

Every stored object becomes a Record.

```
Record<V>

├── key
├── value
├── version
├── created_at
├── updated_at
├── expires_at
├── flags
└── metadata
```

Example:

```rust
Record<T> {
    value: Arc<T>,
    version: u64,
    created_at: Instant,
    updated_at: Instant,
    expires_at: Option<Instant>,
}
```

Benefits:

- Versioning
- TTL
- Replication
- Snapshots
- Optimistic locking

---

# Primary Index

Never use

```
Vec<Record>
```

Instead

```
HashMap<Key, Arc<Record>>
```

or

```
DashMap<Key, Arc<Record>>
```

Lookup:

```
Key
 │
 ▼
HashMap
 │
 ▼
Arc<Record>
```

Complexity

| Operation | Complexity |
|------------|------------|
| Insert | O(1) |
| Lookup | O(1) |
| Remove | O(1) |
| Update | O(1) |

---

# Secondary Indexes

For lookup by multiple fields.

Example

```
User

id
email
username
```

Indexes

```
Primary

id
 │
 ▼
Record

Secondary

email ─────► id

username ──► id
```

This avoids scanning the entire store.

---

# Why Arc?

Avoid cloning large objects.

Without Arc

```
Lookup

↓

Clone Value

↓

Return
```

With Arc

```
Lookup

↓

Clone Arc

↓

Return
```

Arc clone is extremely cheap.

---

# Concurrency Model

## Option 1

```
Arc<RwLock<HashMap>>
```

Pros

- Simple
- Easy to implement

Cons

- Readers still contend
- Doesn't scale as read volume grows

---

## Preferred

```
DashMap<Key, Arc<Record>>
```

Advantages

- Lock striping
- Concurrent reads
- Independent shard locking
- Very high throughput

---

# Read Path

```
Incoming GET

↓

Locate shard

↓

Lookup key

↓

Clone Arc

↓

Return
```

No allocations.

No object cloning.

No copies.

---

# Write Path

```
Incoming PUT

↓

Build Record

↓

Wrap in Arc

↓

Insert

↓

Replace existing entry

↓

Old Arc automatically freed
```

Readers never block each other.

---

# Update Strategy

Avoid mutable references.

Avoid

```
get_mut()
```

Prefer

```
Old Record

↓

Build New Record

↓

Atomic Replace

↓

Old Arc dropped
```

Benefits

- Readers always see consistent state.
- No partially updated objects.
- Simpler synchronization.

---

# Storage Trait

```rust
pub trait Storage<K, V>
where
    K: Eq + Hash,
{
    fn get(&self, key: &K) -> Option<Arc<V>>;
    fn insert(&self, key: K, value: V);
    fn remove(&self, key: &K) -> Option<Arc<V>>;
    fn update(&self, key: K, value: V);
    fn contains(&self, key: &K) -> bool;
}
```

# Table Trait

```rust
pub trait Table<K, V>
where
    K: Eq + Hash,
{
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn get(&self, table: &K) -> Option<Arc<dyn Storage<K, V> + Send + Sync>>;
    fn create(&self, table: &K) -> Arc<dyn Storage<K, V> + Send + Sync>;
    fn clear(&self, table: &K);
}
```

# Usage

The `storage` crate now exposes a table registry through `MemoryStore`, with each table providing a `Storage<K, V>` interface.
`Record<V>` still stores every value behind an `Arc` and tracks a version counter.

## Create a store

```rust
use storage::MemoryStore;

let store: MemoryStore<String, Vec<u8>> = MemoryStore::new();
let table = store.create(&"default".to_string());
```

## Create or open a table

```rust
let table = store.create(&"default".to_string());
```

`create()` returns the named table, creating it if necessary.

## Open an existing table

```rust
let table = store.get(&"default".to_string());
```

If the table does not exist, `get()` returns `None`. Reads and writes do not implicitly create tables.

## Insert data

```rust
table.insert("user:1".to_string(), b"Alice".to_vec());
```

This creates a new `Record` for the key and stores the value behind an `Arc`.

## Fetch data

```rust
if let Some(value) = table.get(&"user:1".to_string()) {
    let bytes: Vec<u8> = (*value).clone();
    println!("got {} bytes", bytes.len());
}
```

The store returns `Arc<V>`, so reads are cheap and do not clone the underlying value unless you explicitly clone it.

## Update data

```rust
table.update("user:1".to_string(), b"Alice v2".to_vec());
```

`update()` now uses entry-based mutation so concurrent updates do not read the old version twice before writing.
If the key already exists, it creates a new `Record` with an incremented version number.
If the key does not exist yet, it behaves like `insert` and starts at version 1.

## Delete data

```rust
table.remove(&"user:1".to_string());
```

This removes the key and drops the old `Arc<Record<V>>` when no other references remain.

## Check existence

```rust
let exists = table.contains(&"user:1".to_string());
```

## Delete a table

```rust
let existed = store.get(&"default".to_string()).is_some();
store.clear(&"default".to_string());
```

`clear()` removes the named table from the registry. Any existing handles to the old table remain alive until they are dropped.

## Validation and limits

The current implementation enforces input validation to reduce resource abuse:

- Table names: max 128 bytes, allowed characters `A-Z`, `a-z`, `0-9`, `_`, `-`, `.`
- Keys: max 256 bytes, same character restrictions
- Values: max 4 MB
- `get` / `update` / `delete` / `contains` do not create tables implicitly; the table must already exist

## Record versioning

You can construct records directly if needed, but the store normally handles version generation for you.

```rust
use storage::Record;

let first = Record::new(b"hello".to_vec());
let next = Record::next(b"hello v2".to_vec(), first.version);
```

`Record::next` returns a new versioned record with `previous_version + 1`.

## Example flow

```rust
let store: MemoryStore<String, Vec<u8>> = MemoryStore::new();
let table = store.create(&"default".to_string());

table.insert("k".to_string(), b"v1".to_vec());
assert_eq!(table.get(&"k".to_string()).as_deref(), Some(&b"v1".to_vec()));

table.update("k".to_string(), b"v2".to_vec());
assert_eq!(table.get(&"k".to_string()).as_deref(), Some(&b"v2".to_vec()));

table.remove(&"k".to_string());
assert_eq!(table.get(&"k".to_string()), None);
```

This is the current usage model for the in-memory table-backed store: create a table, insert or update values, read them back by key, and remove them when no longer needed.

Future implementations

```
MemoryStore

DiskStore

RedisStore

EtcdStore

RemoteStore
```

---

<a id="communication-layer"></a>
# Communication Layer

The storage engine is exposed to remote clients strictly through the existing `transport` crate. Storage never
opens a socket, encodes a frame, or manages a connection — it only implements `transport::server::Handler` for
each operation and registers those handlers with `transport::server::Actions`, exactly like `interface`'s
`EchoHandler` example (see [`doc/transport/resilient.md §9`](../transport/resilient.md)).

```
Client                                  Storage Node
──────                                  ────────────
ResilientClient::send_message()   ──►   ResilientServer::run()
  builds RequestEnvelope                   Dispatcher.dispatch(action)
  { action, payload }                         │
                                              ▼
                                        Actions.lookup(action) -> Handler
                                              │
                                              ▼
                                     Storage API Handler (this crate)
                                              │
                                              ▼
                                     Storage::get/insert/update/remove
                                              │
                                              ▼
                                     ResponseEnvelope { payload }   ──►  ResilientClient
```

## Action mapping

Each storage operation is registered under a stable action name; payloads are `byteser`-encoded structs nested
inside the transport's opaque `RequestEnvelope.payload` / `ResponseEnvelope.payload` bytes.

| Action name | Storage call         | Request payload      | Response payload        |
|-------------|-----------------------|-----------------------|--------------------------|
| `get`       | `Storage::get`        | `{ key }`             | `{ value: Option<V> }`   |
| `put`       | `Storage::insert`     | `{ key, value }`      | `{ ok: bool }`           |
| `update`    | `Storage::update`     | `{ key, value }`      | `{ ok: bool }`           |
| `delete`    | `Storage::remove`     | `{ key }`             | `{ ok: bool }`           |
| `contains`  | `Storage::contains`   | `{ key }`             | `{ exists: bool }`       |

## Handler sketch

```rust
struct GetHandler<K, V> {
    store: Arc<dyn Storage<K, V>>,
}

#[async_trait]
impl<K, V> transport::server::Handler for GetHandler<K, V>
where
    K: ByteSerializable + Eq + Hash + Send + Sync,
    V: ByteSerializable + Send + Sync,
{
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let mut slice: &[u8] = &payload;
        let req: GetRequest<K> = GetRequest::byte_deserialize(&mut slice)
            .map_err(|e| TransportError::InternalError(e.to_string()))?;

        let resp = GetResponse { value: self.store.get(&req.key) };
        let mut out = Vec::new();
        resp.byte_serialize(&mut out);
        Ok(out)
    }
}
```

## Server bootstrap

```rust
let mut actions = Actions::new();
actions.register_action("get", GetHandler { store: store.clone() });
actions.register_action("put", PutHandler { store: store.clone() });
actions.register_action("update", UpdateHandler { store: store.clone() });
actions.register_action("delete", DeleteHandler { store: store.clone() });
actions.register_action("contains", ContainsHandler { store: store.clone() });
let actions = Arc::new(actions);

let listener = TcpListener::bind(addr).await?;
loop {
    let (socket, peer_addr) = listener.accept().await?;
    let actions = actions.clone();
    tokio::spawn(async move {
        ResilientServer::new(socket, peer_addr, actions).run().await
    });
}
```

This is identical to `interface/src/server.rs`, with storage-specific handlers registered instead of `EchoHandler`.

## Client usage

```rust
let mut client = ResilientClient::new(host, port);
client.build_stream().await?;

let request = RequestEnvelope { action: "get".into(), payload: get_request_bytes };
let response: ResponseEnvelope = client.send_message(&request).await?;
```

## Why reuse `transport` instead of building a new protocol

- `transport` already provides TLS-ready (rustls), multiplexed, versioned, length-prefixed framing — reinventing
  this inside `storage` would duplicate work and risk a second, inconsistent wire format.
- Keeps the storage engine generic and network-agnostic, per the [Separation of Concerns](#separation-of-concerns)
  principle below — `storage` depends on `transport`, never the other way around.
- Reuses the `Actions`/`Handler`/`Dispatcher` routing already proven by the `interface` crate, so adding a new
  storage operation is just adding a new `Handler` + action name, with no transport-layer changes.

---

# Metadata

Each record stores

```
Version

Created Time

Updated Time

TTL

Flags

Checksums (optional)
```

Useful for

- replication
- snapshots
- optimistic locking
- expiration
- auditing

---

# Metrics

Store-level metrics

```
Read Count

Write Count

Delete Count

Cache Hit

Cache Miss

Memory Usage

Average Latency

Current Object Count
```

Should be atomics.

---

# Background Workers

The storage engine should be designed with independent workers.

---

## 1. Expiration Worker

Responsibilities

- Scan expired keys
- Remove stale records
- Update metrics

Runs every configurable interval.

---

## 2. Snapshot Worker

Responsibilities

- Serialize in-memory state
- Write snapshots
- Enable recovery

Future persistence support.

---

## 3. Replication Worker

Responsibilities

- Stream updates
- Publish WAL entries
- Replicate changes

Future distributed support.

---

## 4. Metrics Worker

Responsibilities

- Aggregate metrics
- Export Prometheus counters
- Compute averages
- Report memory usage

---

## 5. Compaction Worker

Responsibilities

- Clean obsolete metadata
- Release stale indexes
- Reclaim memory

---

## 6. Health Worker

Responsibilities

- Internal diagnostics
- Memory pressure monitoring
- Dead worker detection
- Store health reporting

---

## 7. TTL Scheduler

Instead of scanning every record:

```
Min Heap

expiration_time

↓

Expired Keys

↓

Delete
```

Much faster than full scans.

---

## 8. Index Rebuilder

Used after

- snapshot restore
- crash recovery
- replication catch-up

Rebuilds all secondary indexes.

---

## 9. Statistics Worker

Computes

- hottest keys
- cold keys
- object distribution
- key frequencies

Useful for optimization.

---

## 10. Event Worker

Responsible for

```
Insert Event

Delete Event

Update Event

Expiration Event
```

Allows subscribers to receive notifications.

---

# Future Features

- WAL (Write Ahead Log)
- Persistence
- AOF
- Snapshot Recovery
- Replication
- Cluster Mode
- Compression
- Encryption
- MVCC
- Transactions
- Pub/Sub
- Streams
- Bloom Filters
- LRU/LFU eviction
- Multi-version snapshots

---

# Design Principles

## Single Source of Truth

```
Key

↓

Primary Index

↓

Record
```

Everything references the primary index.

---

## Immutable Reads

Readers never modify objects.

---

## Atomic Updates

Replace entire records instead of mutating them.

---

## Generic Storage

Storage should know nothing about business objects.

It stores

```
Key

↓

Record<Value>
```

Nothing more.

---

<a id="separation-of-concerns"></a>
## Separation of Concerns

```
Remote Client

↓

transport (ResilientClient / ResilientServer / StreamManager)

↓

Application (Storage API Handlers)

↓

Repository

↓

Storage Engine

↓

Memory
```

Business logic never exists inside the storage layer. Networking code never exists inside the storage layer
either — it lives entirely in `transport`.

---

# Overall Architecture

```
                     transport crate
           (ResilientClient / ResilientServer)
                            │
                  Storage API Handlers
                            │
                        Storage Engine
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
  Primary Index         Secondary Indexes        Metrics
        │                      │                      │
        └──────────────┬───────┴──────────────┬───────┘
                       │                      │
                  Record Store          Worker System
                       │
      ┌────────────────┼────────────────┐
      │                │                │
  Expiration      Replication      Snapshot
      │                │                │
      └────────────────┼────────────────┘
                       │
                  Generic Records
                       │
               Arc<Record<Value>>
```

The `Replication` worker will eventually publish changes as opaque payloads over `transport` streams as well,
reusing the same framing rather than a separate replication protocol.

---

# Long-Term Vision

The storage engine should evolve similarly to Redis internally:

- Generic storage abstraction
- O(1) primary lookups
- Lock-efficient concurrent reads
- Copy-on-write style updates
- Metadata-rich records
- Worker-driven maintenance
- Pluggable persistence
- Pluggable replication
- Extensible indexing
- Production-ready observability
- Networking delegated entirely to `transport`, never reimplemented per service

The goal is to build a reusable storage engine that can back multiple services, not just a single application's data structures.
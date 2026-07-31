# Storage Layer — Implementation & Usage Guide

This document explains the `storage` crate as it exists today: what it
implements, how the pieces fit together, and how to run and interact with the
storage server.

For the long-term design vision, see [`storage.md`](storage.md). For the
distributed routing layer, see [`prd.md`](prd.md).

---

## 1. What the storage crate does

`storage` is an in-memory key-value engine for TrenchDB. Right now it is a
single-node store with no persistence, replication, or TTL. It exposes a
small set of actions (`get`, `put`, `update`, `delete`, `contains`,
`add_table`, `remove_table`) over TCP using the shared `transport` crate.

The design is intentionally layered:

```
Client (ResilientClient)
        │
        ▼
TCP + TRNC framing  ← provided by transport
        │
        ▼
ResilientServer → Actions → Handler::call()
        │
        ▼
storage::api::handlers
        │
        ▼
MemoryStore (Table<String, Vec<u8>>)
        │
        ▼
Collection<String, Vec<u8>> (Storage)
        │
        ▼
Record<Vec<u8>>
```

The storage crate does **not** open raw sockets or parse frames. All
networking is inherited from `transport`.

---

## 2. Core abstractions

### 2.1 `Storage<K, V>`

Defined in [`storage/src/traits.rs`](../../storage/src/traits.rs). This is the
per-table key-value interface.

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

The read path (`get`/`contains`) is required to never panic or unwrap.

### 2.2 `Table<K, V>`

Also in [`storage/src/traits.rs`](../../storage/src/traits.rs). This is the
table registry: a named collection of `Storage` instances.

```rust
pub trait Table<K, V>
where
    K: Eq + Hash,
{
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn get(&self, table: &K) -> Option<Arc<dyn Storage<K, V> + Send + Sync>>;
    fn create(&self, table: &K) -> Arc<dyn Storage<K, V> + Send + Sync>;
    fn remove(&self, table: &K);
}
```

- `create` is lazy: it returns an existing table or allocates a new one.
- `remove` drops the table and all of its entries (previously named `clear`,
  renamed to match the actual behavior).

### 2.3 `Record<V>`

Defined in [`storage/src/rec/record.rs`](../../storage/src/rec/record.rs).

```rust
pub struct Record<V> {
    pub value: Arc<V>,
    pub version: u64,
}
```

Every stored value is wrapped in a `Record`. New records start at version 1;
`update` increments the version. Versioning is used for optimistic-locking
style semantics in later phases.

### 2.4 `Collection<K, V>`

Defined in [`storage/src/rec/collections.rs`](../../storage/src/rec/collections.rs).

A `Collection` implements `Storage<K, V>` over a `DashMap<K, Record<V>>`.
This is the actual per-table data container. Reads clone an `Arc` to the
value instead of cloning the value itself.

### 2.5 `MemoryStore<K, V>`

Defined in [`storage/src/memory/store.rs`](../../storage/src/memory/store.rs).

A `MemoryStore` implements `Table<K, V>` over a
`DashMap<K, Arc<Collection<K, V>>>`. It is the top-level object passed to the
server and shared between all handlers.

---

## 3. Network API

The network-facing API lives in [`storage/src/api/`](../../storage/src/api/).

### 3.1 Request/response types

[`storage/src/api/requests.rs`](../../storage/src/api/requests.rs) defines the
wire structs. All use `String` for table and key names and `Vec<u8>` for
values. They derive `ByteSerializable` from `byteser_derive`.

| Action | Request | Response |
|---|---|---|
| `get` | `GetRequest { table, key }` | `GetResponse { value: Option<Vec<u8>> }` |
| `put` | `PutRequest { table, key, value }` | `PutResponse { ok: bool }` |
| `update` | `UpdateRequest { table, key, value }` | `UpdateResponse { ok: bool }` |
| `delete` | `DeleteRequest { table, key }` | `DeleteResponse { ok: bool }` |
| `contains` | `ContainsRequest { table, key }` | `ContainsResponse { exists: bool }` |
| `add_table` | `AddTableRequest { table }` | `AddTableResponse { ok: bool }` |
| `remove_table` | `RemoveTableRequest { table }` | `RemoveTableResponse { ok: bool }` |

### 3.2 Handlers

The handlers are split into two files by scope:

- [`storage/src/api/collection.rs`](../../storage/src/api/collection.rs) —
  `AddTableHandler` and `RemoveTableHandler`, which operate on the table
  registry itself.
- [`storage/src/api/table.rs`](../../storage/src/api/table.rs) —
  `GetHandler`, `PutHandler`, `UpdateHandler`, `DeleteHandler`, and
  `ContainsHandler`, which operate on records inside a table.

Each handler:

1. Decodes the request.
2. Validates table name and key (alphanumeric, `_`, `-`, `.`, length limits).
3. Validates value size (`<= 4 MiB`).
4. Calls the store.
5. Encodes the response.

Validation failures currently return `TransportError::InternalError`. This
will be replaced with a dedicated client error variant in a future cleanup.

### 3.3 Server wiring

[`storage/src/api/server.rs`](../../storage/src/api/server.rs) registers the
seven handlers on a `transport::server::Actions` object and runs a
`TcpListener` loop. For each accepted connection it spawns a
`ResilientServer` task.

---

## 4. Running the storage server

From the workspace root:

```sh
cargo run -p storage
```

The binary binds `127.0.0.1:7878` and prints:

```
[storage] listening on 127.0.0.1:7878
```

There is no configuration file support yet; the address is hard-coded in
[`storage/src/main.rs`](../../storage/src/main.rs).

---

## 5. Interacting with storage programmatically

There are two ways to use the storage crate: as an in-process library, or over
the network via the `transport` client.

### 5.1 In-process library usage

```rust
use std::sync::Arc;
use storage::{MemoryStore, Record, Storage};

#[tokio::main]
async fn main() {
    let store: Arc<MemoryStore<String, Vec<u8>>> = Arc::new(MemoryStore::new());

    // Put a key into the "users" table.
    let table = store.create(&"users".to_string());
    table.insert("alice".to_string(), b"{\"age\":30}".to_vec());

    // Read it back.
    if let Some(value) = table.get(&"alice".to_string()) {
        println!("value = {:?}", &*value);
    }

    // Drop the whole table.
    store.remove(&"users".to_string());
}
```

### 5.2 Network client usage

The easiest way to talk to the server is the bundled
[`trench-cli`](../../trench-cli/README.md). It provides both one-shot
subcommands and an interactive REPL.

For programmatic access, use `transport::client::resilient_client::ResilientClient`
and the request structs from `storage::api::requests`. Each call opens a stream,
performs a handshake, sends one request, waits for one response, and returns
the underlying connection so it can be reused.

```rust
use std::net::SocketAddr;
use byteser::ByteSerializable;
use storage::api::requests::{GetRequest, GetResponse, PutRequest, PutResponse};
use transport::client::resilient_client::ResilientClient;
use transport::server::RequestEnvelope;

async fn put_and_get(addr: SocketAddr) {
    let mut client = ResilientClient::new(addr.ip().to_string(), addr.port());
    client.build_stream().await.expect("connect failed");

    // Build a put request envelope.
    let mut payload = Vec::new();
    PutRequest {
        table: "users".to_string(),
        key: "alice".to_string(),
        value: b"{\"age\":30}".to_vec(),
    }
    .byte_serialize(&mut payload);

    let request = RequestEnvelope {
        action: "put".to_string(),
        payload,
    };

    let response: transport::server::ResponseEnvelope =
        client.send_message(&request).await.expect("send failed");

    let mut slice: &[u8] = &response.payload;
    let put_response: PutResponse =
        PutResponse::byte_deserialize(&mut slice).expect("decode failed");
    assert!(put_response.ok);

    // Get the same key back.
    let mut payload = Vec::new();
    GetRequest {
        table: "users".to_string(),
        key: "alice".to_string(),
    }
    .byte_serialize(&mut payload);

    let request = RequestEnvelope {
        action: "get".to_string(),
        payload,
    };

    let response: transport::server::ResponseEnvelope =
        client.send_message(&request).await.expect("send failed");

    let mut slice: &[u8] = &response.payload;
    let get_response: GetResponse =
        GetResponse::byte_deserialize(&mut slice).expect("decode failed");

    assert_eq!(get_response.value, Some(b"{\"age\":30}".to_vec()));

    client.close().await.expect("close failed");
}
```

### 5.3 Action reference

| Action | Effect |
|---|---|
| `put` | Inserts or replaces a key in a table. Creates the table if it does not exist. |
| `get` | Returns the raw value for a key, or `None`. Returns an error if the table does not exist. |
| `update` | Replaces an existing key and bumps its version. Returns an error if the table does not exist. |
| `delete` | Removes a key from a table. Returns an error if the table does not exist. |
| `contains` | Returns whether the key exists in the table. Returns `false` if the table does not exist. |
| `add_table` | Creates an empty table. Idempotent. |
| `remove_table` | Drops a table and all its keys. Returns `ok: true` if the table existed. |

### 5.4 Validation rules

- Table names and keys must be non-empty, `<= 128` and `<= 256` bytes
  respectively, and may only contain `a-z A-Z 0-9 _ - .`.
- Values must be `<= 4 MiB`.

---

## 6. Test coverage

The only integration test today is
[`storage/tests/put_get.rs`](../../storage/tests/put_get.rs). It starts the
storage server on a random local port and performs a real TCP `put`/`get`
roundtrip using `ResilientClient`.

Run it with:

```sh
cargo test -p storage
```

---

## 7. Current limitations & what is next

| Area | Status | Notes |
|---|---|---|
| Single-node in-memory store | ✅ Done | `MemoryStore` + `Collection`. |
| Network API | ✅ Done | All actions wired through `transport`. |
| TTL / expiration | ❌ Not started | Next Phase 3 item. |
| Record timestamps | ❌ Not started | `created_at`, `updated_at`, `expires_at`. |
| Metrics | ❌ Not started | Read/write/hit/miss counters. |
| Persistence / WAL / snapshots | ❌ Out of scope | Planned for Phase 4+. |
| Replication | ❌ Out of scope | Planned for Phase 4+. |
| Secondary indexes | ❌ Out of scope | Planned for Phase 4+. |
| Configurable bind address | ❌ Not implemented | Hard-coded to `127.0.0.1:7878`. |

The next concrete milestone is **Phase 3**: extend `Record<V>` with metadata,
add optional TTL to `put`/`update`, implement an expiration worker, and expose
in-process metrics.

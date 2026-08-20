# trench-db — Project Structure, Libraries & Build System

---

## Table of Contents

1. [Full Folder Tree](#1-full-folder-tree)
2. [What Each Folder Is For](#2-what-each-folder-is-for)
3. [What Each File Is For](#3-what-each-file-is-for)
4. [External Libraries — What They Are & Why We Use Them](#4-external-libraries)
5. [How Rust Crates Depend on Each Other](#5-how-rust-crates-depend-on-each-other)
6. [How Binaries Are Generated — The Full Compilation Pipeline](#6-how-binaries-are-generated)
7. [The Workspace `Cargo.toml` Explained](#7-the-workspace-cargotoml-explained)
8. [Build Commands Reference](#8-build-commands-reference)
9. [Dependency Diagram](#9-dependency-diagram)

---

## 1. Full Folder Tree

```
trench-db/
│
├── Cargo.toml                  ← workspace manifest
├── config.trench               ← example node configuration
├── README.md
│
├── doc/                        ← written documentation
│   ├── PROJECT_STRUCTURE.md    ← this file
│   ├── protocol.md
│   ├── storage/
│   │   ├── phase3.md
│   │   ├── prd.md
│   │   ├── storage.md
│   │   └── storage_layer_and_usage.md
│   ├── transport/
│   │   ├── architecture.md
│   │   ├── connection.md
│   │   ├── frame.md
│   │   ├── manager.md
│   │   ├── README.md
│   │   ├── receiver.md
│   │   ├── resilient.md
│   │   └── stream.md
│   └── trench-cli/
│       └── README.md
│
├── interface/                  ← example/demo crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── client.rs
│       ├── server.rs
│       └── bin/
│           ├── client.rs
│           └── server.rs
│
├── storage/                    ← in-memory key-value store
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── traits.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── collection.rs
│   │   │   ├── table.rs
│   │   │   ├── requests.rs
│   │   │   └── server.rs
│   │   ├── memory/
│   │   │   ├── mod.rs
│   │   │   └── store.rs
│   │   └── rec/
│   │       ├── mod.rs
│   │       ├── collections.rs
│   │       └── record.rs
│   └── tests/
│       └── put_get.rs
│
├── transport/                  ← networking + framing crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── errors.rs
│       ├── client/
│       ├── frame/
│       ├── server/
│       └── tcp/
│
├── trench/                     ← top-level node binary (skeleton)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── mod.rs
│       ├── auth/
│       ├── config/
│       ├── neighbors/
│       └── store/
│
└── trench-cli/                 ← command-line client for storage
    ├── Cargo.toml
    ├── README.md
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── client.rs
        ├── parser.rs
        ├── registry.rs
        ├── repl.rs
        └── commands/
            ├── add_table.rs
            ├── command_handler.rs
            ├── contains.rs
            ├── delete.rs
            ├── get.rs
            ├── mod.rs
            ├── put.rs
            ├── remove_table.rs
            └── update.rs
```

---

## 2. What Each Folder Is For

### `doc/`
Plain documentation that explains the project to humans — not to the compiler.
Nothing in here affects the build. Put design notes, plans, and diagrams here.

---

### `interface/`
An example crate demonstrating the `transport` patterns. It is **not**
production code; it shows how `ResilientClient`/`ResilientServer` and
`Actions`/`Handler` fit together. The `storage` crate copied this pattern.

---

### `storage/`
The in-memory storage engine. Implements:
- `Storage<K, V>` and `Table<K, V>` traits.
- `MemoryStore`, a `DashMap`-backed concurrent table registry.
- `Record<V>` with versioning.
- Network API handlers wired to `transport::server::Actions`.

---

### `transport/`
The networking layer. Owns TRNC framing, TCP connection/stream management,
resilient client/server logic, and the `Actions`/`Handler` routing harness.
All storage networking is reused from here — storage does not implement its
own sockets or framing.

---

### `trench/`
The top-level node binary (currently a skeleton). It will eventually
bootstrap a node from `config.trench`, manage identity/auth, and wire
`storage` + `transport` together.

---

## 3. What Each File Is For

### `Cargo.toml` (workspace root)
Defines the Cargo workspace and its members:
```toml
[workspace]
members = ["interface", "storage", "transport", "trench", "trench-cli"]
resolver = "3"
```

---

### `interface/Cargo.toml` + `interface/src/*.rs`
Example/demo crate dependencies and code. Mirrors the pattern `storage`
uses: a server binary that binds a `TcpListener`, registers `Handler`s on
`Actions`, and spawns `ResilientServer` per connection.

---

### `storage/Cargo.toml`
Crate manifest. Dependencies include:
- `async-trait` — for `#[async_trait]` on `Handler` impls.
- `byteser`, `byteser_derive` — wire request/response serialization.
- `dashmap` — lock-striped concurrent hash map.
- `tokio` — async runtime and TCP networking.
- `transport` — local path dependency on the networking crate.

---

### `storage/src/lib.rs`
Crate root. Re-exports the public API:
```rust
pub mod config;
pub mod metadata;
pub mod memory;
pub mod rec;
pub mod traits;

pub use memory::MemoryStore;
pub use rec::record::Record;
pub use traits::{Storage, Table};

pub type SharedStore = Arc<dyn Table<String, Vec<u8>> + Send + Sync>;
```

---

### `trench/src/main.rs`
The node runtime binary. Creates an `Arc<MemoryStore<String, Vec<u8>>>`,
binds `127.0.0.1:7878`, and runs the storage API server through
`trench::api::run_server`.

---

### `trench/src/api/requests.rs`
Wire-facing request/response structs for every storage action, derived with
`ByteSerializable`. Kept concrete (`String` table/key, `Vec<u8>` value) so the
network protocol has a single encoding.

---

### `trench/src/api/collection.rs`
Collection-level `transport::server::Handler` implementations:
`AddTableHandler` and `RemoveTableHandler`. Each decodes a request, calls the
store's table registry, and encodes the response.

---

### `trench/src/api/table.rs`
Record-level `transport::server::Handler` implementations: `GetHandler`,
`PutHandler`, `UpdateHandler`, `DeleteHandler`, and `ContainsHandler`. Each
decodes a request, calls the table, and encodes the response.

---

### `trench/src/api/server.rs`
Wires the handlers into a `transport::server::Actions` registry and runs a
`TcpListener`/`ResilientServer` loop, mirroring `interface/src/server.rs`.

---

### `storage/src/traits.rs`
Defines the two core abstractions:
- `Storage<K, V>` — hot-path key-value operations (`get`, `insert`, `remove`,
  `update`, `contains`).
- `Table<K, V>` — named table registry (`get`, `create`, `remove`,
  `len`, `is_empty`).

---

### `storage/src/rec/record.rs`
A single stored value wrapper:
```rust
pub struct Record<V> {
    pub value: Arc<V>,
    pub version: u64,
}
```
Versioning is used by `Collection::update`.

---

### `storage/src/rec/collections.rs`
`Collection<K, V>` implements `Storage<K, V>` over a
`DashMap<K, Record<V>>`. This is the per-table data container.

---

### `storage/src/memory/store.rs`
`MemoryStore<K, V>` implements `Table<K, V>` over a
`DashMap<K, Arc<Collection<K, V>>>`. Creating a table lazily allocates a
new `Collection`; removing a table drops it and all its entries.

---

### `transport/src/*.rs`
The networking crate. Key modules:
- `frame/` — TRNC framing.
- `tcp/` — `Connection`, `Stream`, `StreamManager`, `Receiver`.
- `client/` — `ResilientClient`.
- `server/` — `ResilientServer`, `Dispatcher`, `Actions`, `Handler`.
- `errors.rs` — `TransportError`.

---

### `trench/src/*.rs`
Skeleton for the final node binary. `config::loader` parses `config.trench`;
`auth/`, `neighbors/`, and `store/` are placeholders for future features.

---

### `trench-cli/src/main.rs`
Entry point for the command-line storage client. Parses `host`/`port` flags
and an optional subcommand; if no subcommand is given it starts the REPL.

---

### `trench-cli/src/client.rs`
`PersistentClient` — a TCP client that maintains a single `transport`
`StreamManager`, completes the TRNC handshake, and automatically reconnects
with exponential backoff on failure.

---

### `trench-cli/src/registry.rs`
`CommandRegistry` — maps command names (`get`, `put`, `update`, `delete`,
`contains`, `add_table`, `remove_table`) to their `CommandHandler`
implementations.

---

### `trench-cli/src/commands/*.rs`
One file per CLI command. Each command builds the appropriate
`storage::api::requests` struct, sends it through `PersistentClient`, and
prints the response.

---

### `trench-cli/src/repl.rs`
Interactive read-eval-print loop. Reads lines from stdin, dispatches commands
through the registry, and supports `help` and `quit`/`exit`.

---

### `trench-cli/src/parser.rs`
Small argument-parsing helpers used by command handlers to validate argument
counts and join multi-word values.

---

## 4. External Libraries

### Tokio
**What it is:** The dominant async runtime for Rust.

**Why we use it:**
- `tokio::net::TcpListener` / `TcpStream` for async networking.
- `tokio::spawn` for per-connection tasks.
- `macros` and `rt-multi-thread` for the multi-threaded runtime.

**How it is installed:** Listed in each crate's `Cargo.toml`; Cargo downloads
and builds it automatically. No manual install step.

---

### DashMap
**What it is:** A high-performance concurrent hash map for Rust using lock
striping.

**Why we use it:** It allows many concurrent readers without a global lock,
which matches our read-heavy workload. Used in both `MemoryStore` (table
registry) and `Collection` (per-table data).

**How it is installed:** Listed in `storage/Cargo.toml`; Cargo handles it.

---

### async-trait
**What it is:** A proc-macro that makes async methods in traits ergonomic.

**Why we use it:** `transport::server::Handler::call` is async; `async-trait`
lets us implement it directly.

**How it is installed:** Listed in `storage/Cargo.toml` and `transport/Cargo.toml`.

---

### byteser / byteser_derive
**What it is:** A custom (workspace-local) serialization library and its
`ByteSerializable` derive macro.

**Why we use it:** Request/response structs use `#[derive(ByteSerializable)]`
to encode/decode payloads sent over `transport`.

**How it is installed:** Listed in `storage/Cargo.toml` and `interface/Cargo.toml`;
likely a local path dependency.

---

## 5. How Rust Crates Depend on Each Other

In Rust, dependencies are declared per crate in `Cargo.toml`:

```toml
[dependencies]
transport = { version = "0.1.0", path = "../transport" }
```

This is roughly equivalent to C's "compile + link against a sibling library",
but Cargo handles both steps: it compiles `transport` first, then makes its
public items available to `storage` at compile time and links the rlib during
the final binary build.

### Crate dependency graph

```
                    ┌─────────────┐
                    │   trench    │
                    │  (skeleton) │
                    └──────┬──────┘
                           │ (planned)
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
      ┌─────────┐    ┌──────────┐    ┌──────────┐
      │ storage │◄───│ transport │───►│ interface │
      └────┬────┘    └──────────┘    └───────────┘
           │                ▲
           ▼                │
      ┌─────────┐     ┌──────────┐
      │ byteser │     │trench-cli│
      └─────────┘     └──────────┘
```

- `storage` depends on `transport` and `byteser`.
- `interface` depends on `transport` and `byteser`.
- `trench-cli` depends on `storage`, `transport`, `byteser`, and `clap`.
- `trench` is a skeleton and currently does not depend on any other crate.

---

## 6. How Binaries Are Generated — The Full Compilation Pipeline

Rust compilation with Cargo happens in roughly three stages:

### Stage 1 — Dependency resolution
```sh
cargo check
```
Cargo reads every `Cargo.toml`, resolves the dependency graph, downloads
missing crates from crates.io or local paths, and locks versions in
`Cargo.lock`.

### Stage 2 — Crate compilation (source → rlib)
```
storage/src/*.rs  ──┐
transport/src/*.rs ─┤   rustc      ┌── libstorage.rlib ──┐
byteser/src/*.rs  ──┤  ────────►   │   libtransport.rlib  │   rustc (link)
interface/src/*.rs ─┘              │   libbyteser.rlib   ─┤  ──────────► storage.exe
                                   └─────────────────────┘      interface.exe
                                                                  transport tests...
```

Each crate compiles independently into an `rlib` (Rust static library). Cargo
reuses already-built rlibs when their source has not changed.

### Stage 3 — Linking (rlibs + deps → executable)
Cargo invokes the linker with the relevant rlibs, native libraries, and
runtime objects to produce the final binary.

---

## 7. The Workspace `Cargo.toml` Explained

The root `Cargo.toml` only declares the workspace:

```toml
[workspace]
members = ["interface", "storage", "transport", "trench", "trench-cli"]
resolver = "3"
```

- `members` — the subdirectories that are part of this workspace.
- `resolver = "3"` — which version of Cargo's dependency resolver to use.

Each member has its own `Cargo.toml` with its own `[package]` and
`[dependencies]` sections.

---

## 8. Build Commands Reference

### First-time setup

Install Rust if you haven't already:
```sh
# Follow https://rustup.rs, or on most systems:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

No vendored C sources or system crypto libraries are required for the current
Rust workspace.

### Common Cargo commands

| Command | What it does |
|---|---|
| `cargo build` | Compile the entire workspace |
| `cargo build -p storage` | Compile only the `storage` crate |
| `cargo check` | Fast syntax/type check without producing binaries |
| `cargo test` | Build and run all tests in the workspace |
| `cargo test -p storage` | Build and run only `storage` tests |
| `cargo run -p storage` | Build and run the `storage` server binary |
| `cargo run -p interface --bin server` | Run the interface example server |
| `cargo run -p trench-cli` | Start the trench-cli REPL |
| `cargo clippy -p storage` | Run the linter on the `storage` crate |
| `cargo clean` | Delete all build artifacts |

### Run the storage server

```sh
cargo run -p storage
# listens on 127.0.0.1:7878
```

---

## 9. Dependency Diagram

This shows which Rust modules/crates depend on which others. An arrow means
"depends on / uses".

```
storage/src/traits.rs
      │
      ├──► storage/src/rec/record.rs
      │
      ├──► storage/src/rec/collections.rs
      │
      ├──► storage/src/memory/store.rs
      │
      └──► storage/src/api/*.rs ──► transport::server::Handler
              │
              ├──► storage/src/api/requests.rs ──► byteser_derive
              │
              └──► storage/src/api/server.rs ──► transport::server::{Actions, ResilientServer}
```

`transport` and `byteser` are independent workspace crates that `storage`,
`interface`, and `trench-cli` consume. `trench` is currently disconnected.

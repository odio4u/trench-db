# trench-db

TrenchDB is an experimental distributed leaderless key-value system built in Rust.
It is organized as a workspace of reusable crates for transport, storage,
client tooling, and a node runtime skeleton.

## Full scope
Build a decentralized data storage, with a leaderless architecture, regional data sovereignty, and a self-healing routing layer.

## What this repository contains

- `transport/` — TRNC binary framing, TCP connection management, logical stream multiplexing, and a resilient request/response layer.
- `storage/` — in-memory key-value engine with table registry, request handlers, and a storage server binary.
- `trench-cli/` — command-line client and REPL for talking to a running storage node.
- `trench/` — top-level node binary skeleton with configuration parsing and node bootstrap structure.
- `interface/` — example/demo crate showing how to wire `transport` into a simple client/server application.
- `doc/` — detailed design documentation for project structure, transport protocol, storage design, and CLI behavior.

## Current system status

- `transport` is the core networking layer and provides the wire protocol, frame encoding/decoding, logical streams, and connection multiplexing.
- `storage` is a working in-memory single-node store exposing `get`, `put`, `update`, `delete`, `contains`, `add_table`, and `remove_table` over the `transport` layer.
- `trench-cli` is a working CLI client for the storage server, including one-shot commands and a REPL.
- `trench` is currently a skeleton runtime with config parsing and module placeholders, not yet wired to the storage or transport stacks.
- `interface` is an example implementation demonstrating how to use `transport` in a client/server pattern.

## Quick start

From the workspace root:

```sh
cargo build
```

Run the storage server:

```sh
cargo run -p storage
```

Use the CLI against the running server:

```sh
cargo run -p trench-cli -- put users alice '{"age":30}'
cargo run -p trench-cli -- get users alice
```

## Repository layout

- `Cargo.toml` — workspace manifest.
- `config.trench` — example node configuration file.
- `doc/` — nested system documentation.
- `interface/` — demo client/server crate.
- `storage/` — in-memory data store implementation.
- `transport/` — networking and wire protocol crate.
- `trench/` — node runtime skeleton.
- `trench-cli/` — command-line client crate.

## Documentation

This README is a top-level summary only. For full design and implementation details, see the nested docs:

- `doc/PROJECT_STRUCTURE.md` — repo structure, crate responsibilities, and build model.
- `doc/protocol.md` — transport protocol goals and framing design.
- `doc/storage/storage.md` — current storage crate implementation and usage.
- `doc/transport/README.md` — transport layer overview and internal architecture.
- `doc/trench-cli/README.md` — CLI behavior, commands, and REPL details.
- `doc/storage/phase3.md` — planned storage Phase 3 metadata and TTL work.
- `doc/storage/prd.md` — long-term distributed routing and cluster design.

## Development notes

- Build the whole workspace with `cargo build`.
- Run all tests with `cargo test`.
- The storage server currently listens on `127.0.0.1:7878` by default.
- TLS, persistence, replication, and full distributed routing are planned but not implemented yet.

## Want more detail?

This repository keeps detailed design notes in `doc/` so the top-level README stays focused. If you need architecture, protocol, or storage internals, follow the links above rather than expanding this file further.

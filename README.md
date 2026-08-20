# TrenchDB

TrenchDB is an experimental distributed key-value database written in Rust.

The project is building toward a leaderless, decentralized storage system with regional data sovereignty, self-healing routing, and reusable networking primitives. The current implementation focuses on the core foundations: transport, storage, and node runtime infrastructure.

## Vision

TrenchDB aims to provide:

- Leaderless distributed architecture
- Decentralized data storage
- Regional data sovereignty
- Self-healing routing and node discovery
- High-performance networking and storage primitives
- Modular components that can be reused independently

## Repository Structure

The workspace is organized into independent crates.

| Crate | Description |
|-------|-------------|
| `transport/` | TRNC binary protocol, TCP connection management, stream multiplexing, and request/response transport. |
| `storage/` | In-memory key-value engine with table management. |
| `trench-cli/` | Command-line client and interactive REPL. |
| `trench/` | Node runtime skeleton and storage API server for future distributed nodes. |
| `interface/` | Example crate demonstrating client/server integration using the transport layer. |
| `doc/` | Architecture, protocol, storage, and implementation documentation. |

## Current Status

Implemented today:

- ✅ Custom binary transport protocol
- ✅ TCP networking
- ✅ Logical stream multiplexing
- ✅ In-memory key-value storage
- ✅ Table management
- ✅ CLI client and REPL
- ✅ Modular workspace architecture

In progress:

- 🚧 Distributed node runtime
- 🚧 Cluster routing
- 🚧 Replication
- 🚧 Persistence
- 🚧 Node discovery

Planned:

- Leaderless distributed storage
- Regional data placement
- Self-healing routing
- Data replication
- Fault tolerance
- TLS and authentication

## Quick Start

Build the workspace:

```sh
cargo build
```

Start the storage server:

```sh
cargo run -p trench
```

Store a value:

```sh
cargo run -p trench-cli -- put users alice '{"age":30}'
```

Retrieve it:

```sh
cargo run -p trench-cli -- get users alice
```

## Repository Layout

```
.
├── transport/
├── storage/
├── trench/
├── trench-cli/
├── interface/
├── doc/
├── Cargo.toml
└── config.trench
```

## Documentation

Detailed documentation lives under `doc/`.

- `PROJECT_STRUCTURE.md` — Workspace organization
- `protocol.md` — TRNC transport protocol
- `storage/storage.md` — Storage engine design
- `transport/README.md` — Transport architecture
- `trench-cli/README.md` — CLI usage
- `storage/phase3.md` — Metadata and TTL roadmap
- `storage/prd.md` — Distributed storage design

## Development

Build:

```sh
cargo build
```

Run tests:

```sh
cargo test
```

The storage server listens on `127.0.0.1:7878` by default.

## Project Status

TrenchDB is under active development. The networking stack and single-node storage engine are functional, while the distributed runtime, replication, routing, and persistence are being developed incrementally.
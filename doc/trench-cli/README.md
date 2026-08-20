# trench-cli

Command-line client for the TrenchDB storage server.

`trench-cli` connects to a running [`storage`](../../storage) node over TCP,
exposes every storage action as a subcommand, and provides an interactive
REPL for ad-hoc queries.

---

## Table of Contents

1. [What trench-cli does](#what-trench-cli-does)
2. [Project layout](#project-layout)
3. [Building](#building)
4. [Usage](#usage)
5. [Command reference](#command-reference)
6. [REPL](#repl)
7. [Connection handling](#connection-handling)
8. [Validation rules](#validation-rules)
9. [Current limitations](#current-limitations)

---

## What trench-cli does

`trench-cli` is a thin client over the [`transport`](../../transport) crate.
It does not open raw sockets or parse TRNC frames directly; instead it uses
`transport::tcp::{Connection, StreamManager}` to establish a single logical
stream per request and decode responses through `byteser`.

High-level flow:

```
User input  →  CommandRegistry  →  CommandHandler
                                      │
                                      ▼
                         storage::api::requests struct
                                      │
                                      ▼
                         PersistentClient::send(action, payload)
                                      │
                                      ▼
                         transport StreamManager → TCP → storage server
```

---

## Project layout

```
trench-cli/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs          ← CLI entry point and argument parsing
    ├── lib.rs           ← module declarations
    ├── client.rs        ← PersistentClient (TCP + reconnect)
    ├── parser.rs        ← argument-count helpers
    ├── registry.rs      ← CommandRegistry mapping names to handlers
    ├── repl.rs          ← interactive read-eval-print loop
    └── commands/
        ├── command_handler.rs  ← CommandHandler trait
        ├── get.rs
        ├── put.rs
        ├── update.rs
        ├── delete.rs
        ├── contains.rs
        ├── add_table.rs
        ├── remove_table.rs
        └── mod.rs
```

### Key components

| File | Purpose |
|---|---|
| `main.rs` | Parses `host`/`port` flags and an optional subcommand with `clap`. Starts the REPL when no subcommand is given. |
| `client.rs` | `PersistentClient` keeps one `StreamManager<TcpStream>` alive, completes the TRNC handshake, and reconnects with exponential backoff on failure. |
| `registry.rs` | `CommandRegistry` holds all command handlers and is used by both the one-shot CLI path and the REPL. |
| `commands/*.rs` | One `CommandHandler` implementation per storage action. Each builds the corresponding `trench::api::requests` struct and prints the response. |
| `repl.rs` | Reads lines from stdin, splits on whitespace, dispatches through the registry, and supports `help` / `quit` / `exit`. |
| `parser.rs` | Validates argument counts and joins multi-word values into a single `Vec<u8>`. |

---

## Building

From the workspace root:

```sh
cargo build -p trench-cli
```

Or to build the entire workspace:

```sh
cargo build
```

`trench-cli` depends on:

- `clap` — command-line argument parsing.
- `tokio` — async runtime and TCP networking.
- `async-trait` — async trait methods for `CommandHandler`.
- `byteser` / `byteser_derive` — request/response serialization.
- `transport` — TRNC framing, stream management, and handshake.
- `trench` — re-uses the public `trench::api::requests` types and `encode` helper.

---

## Usage

Start the storage server first:

```sh
cargo run -p storage
# [storage] listening on 127.0.0.1:7878
```

Then run `trench-cli`. The default host and port are `127.0.0.1:7878`.

### One-shot command

```sh
cargo run -p trench-cli -- put users alice '{"age":30}'
cargo run -p trench-cli -- get users alice
cargo run -p trench-cli -- contains users alice
```

### Connect to a remote host

```sh
cargo run -p trench-cli -H 192.168.1.100 -p 7878 -- get users alice
```

### Start the REPL

```sh
cargo run -p trench-cli
```

---

## Command reference

All commands are also available inside the REPL.

| Command | Arguments | Description |
|---|---|---|
| `get` | `<table> <key>` | Retrieve the value for a key. Prints `(not found)` if missing. |
| `put` | `<table> <key> <value...>` | Insert or overwrite a key-value pair. Creates the table if it does not exist. |
| `update` | `<table> <key> <value...>` | Replace the value of an existing key. Fails if the table does not exist. |
| `delete` | `<table> <key>` | Remove a key from a table. Fails if the table does not exist. |
| `contains` | `<table> <key>` | Print `true` or `false` depending on whether the key exists. |
| `add_table` | `<table>` | Create an empty table. Idempotent. |
| `remove_table` | `<table>` | Drop a table and all of its records. |
| `help` | | Show available commands (REPL only). |
| `quit` / `exit` | | Exit the REPL. |

Values with spaces are joined into a single value:

```
trench> put users alice {"name":"Alice","age":30}
```

This sends the bytes `{"name":"Alice","age":30}` as the value.

---

## REPL

The REPL reads one line at a time, tokenizes it on whitespace, and dispatches
the first token as a command name.

```
$ cargo run -p trench-cli
trench-db CLI connected to 127.0.0.1:7878
Type 'help' for available commands, 'quit' to exit.
trench> put users alice hello world
ok: true
trench> get users alice
hello world
trench> contains users alice
true
trench> delete users alice
ok: true
trench> quit
```

Errors from the server or malformed input are printed to stderr and the loop
continues.

---

## Connection handling

`PersistentClient` maintains a single TCP connection for the lifetime of the
CLI process:

1. On first request it connects to `host:port`.
2. It completes the TRNC handshake via `StreamManager::start_handshake`.
3. Each command opens a fresh logical stream, sends one request, waits for the
   matching response, and closes the stream.
4. If a send fails, the client discards the broken manager and retries the
   whole exchange up to three times with exponential backoff (500 ms, 1 s, 1.5 s).

Because each request uses a new stream ID, multiple commands cannot be
interleaved on the same stream; this keeps the CLI simple and matches the
request/response semantics of the storage API.

---

## Validation rules

Validation happens on the server, but the CLI sends the raw strings/bytes
exactly as received:

- Table names and keys must be non-empty, use only `a-z A-Z 0-9 _ - .`, and
  are limited to 128 and 256 bytes respectively.
- Values must be `<= 4 MiB`.

If the server rejects a request, `PersistentClient` prints the error and the
command returns a non-zero exit code (for one-shot usage) or prints the error
in the REPL.

---

## Current limitations

- No support for TTL; `put` and `update` always create records without expiry.
- No authentication or TLS.
- The CLI parses values as raw bytes and prints them as lossy UTF-8; binary
  values may not display cleanly.
- Only the seven core storage actions are exposed; metrics, snapshots, and
  replication commands are not implemented yet.

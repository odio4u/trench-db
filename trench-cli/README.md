# trench-cli

Command-line client for the TrenchDB storage server.

## Quick start

Start the storage server:

```sh
cargo run -p trench
```

In another terminal, run a one-shot command:

```sh
cargo run -p trench-cli -- put users alice '{"age":30}'
cargo run -p trench-cli -- get users alice
```

Or start the interactive REPL:

```sh
cargo run -p trench-cli
```

## Full documentation

See [doc/trench-cli/README.md](../doc/trench-cli/README.md) for the complete
architecture, command reference, and connection-handling details.

# Write-Ahead Log (WAL)

This document describes the current embedded WAL ("ewal") implementation in the
`storage` crate, including its file layout, public API, initialization
behavior, integration with the storage node, and the roadmap to a production
persistence layer.

For the broader storage design, see [`storage.md`](storage.md). For usage
details, see [`storage_layer_and_usage.md`](storage_layer_and_usage.md).

---

## 1. Purpose

The WAL provides a process-wide append-only log for storage mutations. It is
intended to support:

- Durability of writes before they are reflected in the in-memory store.
- Future replication (a replication worker can tail the WAL).
- Future recovery / replay on startup.

It is **not** a full persistence layer yet: recovery, compaction, and
encryption are still future work.

---

## 2. Implementation layout

```
storage/src/
├── ewal/
│   ├── mod.rs        ← WalEntry, WAL_HEADER_SIZE, public re-exports
│   ├── error.rs      ← WalError
│   └── pipe.rs       ← WALWriter
└── walmanager/
    ├── mod.rs        ← Public re-exports
    └── writers.rs    ← WalManager + singleton API
```

---

## 3. `WALWriter`

Defined in [`storage/src/ewal/pipe.rs`](../../storage/src/ewal/pipe.rs).

```rust
pub struct WALWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
    position: u64,
    pathbuf: String,
}
```

- `writer`: buffered file handle protected by a mutex so the same writer can be
  shared across threads.
- `position`: logical byte offset of the next write (starts after the 8-byte
  WAL header).
- `pathbuf`: path of the WAL file.

### 3.1 Initialization

```rust
pub fn init(path: &str) -> Result<Self, WalError>
```

Steps:

1. Validate that `path` has a parent directory (`PathMissing` otherwise).
2. Create the parent directory tree if it does not exist.
3. Open the file with `create | append | write`.
4. Wrap it in a `BufWriter<File>`.
5. Set `position = WAL_HEADER_SIZE` (8 bytes).

Important behaviors:

- The WAL file is created empty; the header is **not** written yet by `init`.
  Writers must call `flush` after the first append, or an explicit header write
  can be added later.
- The path is taken as-is. Config values must be stripped of surrounding quotes
  before being passed here.
- All I/O is synchronous (`std::fs`). Callers running inside an async runtime
  should invoke WAL initialization through `tokio::task::spawn_blocking`.

### 3.2 Public operations

| Method | Behavior |
|--------|----------|
| `append(entry)` | Append one entry to the WAL. |
| `append_batch(entries)` | Append many entries. |
| `flush()` | Flush the `BufWriter` to the OS. |
| `sync()` | Call `sync_data` on the underlying file without flushing first. |
| `get_position()` | Return the logical write offset. |
| `path()` | Return the WAL file path. |

### 3.3 Entry serialization (temporary)

A `WalEntry` is currently an in-memory struct:

```rust
pub struct WalEntry {
    pub payload: Vec<u8>,
    pub key: Option<Vec<u8>>,
    pub operation: Option<u8>,
    pub checksum: Option<u32>,
}
```

| Field | Current use |
|-------|-------------|
| `payload` | Raw event bytes, e.g. `insert:users:alice`. |
| `key` | Composite key extracted from the payload, e.g. `users:alice`. |
| `operation` | Operation code: `1` = insert, `2` = delete, `3` = update. |
| `checksum` | Reserved for future CRC32; currently `None`. |

When appended, the writer currently serializes the entry as human-readable
debug text. This is intentionally temporary until a binary frame format is
defined.

Current constraints:

- Max payload size is hard-coded to 2000 bytes in `WALWriter::append`.
- No checksum validation yet.
- No encryption yet.
- No binary framing yet.

---

## 4. WAL file structure

```text
+--------------------------------------------------+
| WAL Header | Entry 1 | Entry 2 | ... | Entry N   |
+--------------------------------------------------+
```

### 4.1 Header (8 bytes)

```rust
pub const WAL_HEADER_SIZE: usize = 8;
```

The exact header bytes are not yet finalized. The current code reserves 8 bytes
for a future magic number + version field.

### 4.2 Entries (current text format)

Each appended entry is currently written as plain text:

```text
WAL Entry:
  Payload Length: 18
  Payload: insert:users:alice
  Checksum: None
  Key: Some(users:alice)
  Operation: Some(1)
```

The dispatcher in [`storage/src/events/dispatcher.rs`](../../storage/src/events/dispatcher.rs)
parses the payload `op:table:key` and populates:

- `Operation`: `1` for `insert`, `2` for `delete`, `3` for `update`.
- `Key`: the composite `table:key` bytes.
- `Payload`: the original event payload unchanged.

This text format exists only for visibility during early development. The
planned binary format will be:

```text
+--------------------------------------------------+
| entry_len: u32 | checksum: u32 | payload: [u8]   |
+--------------------------------------------------+
```

Where `entry_len` is the byte length of `payload`, and `checksum` is a CRC32
over the payload.

---

## 5. WAL trigger flow

Storage mutations flow through the event loop into the WAL:

```
Client (trench-cli)
        │
        ▼
PutHandler / UpdateHandler / DeleteHandler  (trench/src/api/table.rs)
        │
        ▼
publish_storage_event("op:table:key")
        │
        ▼
SharedQueue → EventLoopSupervisor
        │
        ▼
Dispatcher::dispatch  (storage/src/events/dispatcher.rs)
        │
        ▼
WalManager::append + flush  (storage/src/walmanager/writers.rs)
        │
        ▼
data/<id>/wal.log
```

- [`GetHandler`](../../trench/src/api/table.rs) and [`ContainsHandler`](../../trench/src/api/table.rs)
  do **not** publish events; only mutating operations are logged.
- The storage methods in [`Collection`](../../storage/src/rec/collections.rs)
  no longer publish events themselves, so each client mutation produces exactly
  one WAL entry.

---

## 6. Singleton manager (`walmanager`)

Defined in [`storage/src/walmanager/writers.rs`](../../storage/src/walmanager/writers.rs).

```rust
pub struct WalManager {
    writer: Option<WALWriter>,
}
```

`create_writer(path)` opens the WAL file and stores the writer. Subsequent
operations delegate to the inner `WALWriter`.

### 5.1 Singleton API

```rust
static WAL_MANAGER: OnceLock<Mutex<WalManager>> = OnceLock::new();

pub fn init_wal_manager<P: AsRef<Path>>(path: P) -> Result<(), WalError>;
pub fn wal_manager() -> &'static Mutex<WalManager>;
pub fn append(entry: &WalEntry) -> Result<(), WalError>;
pub fn append_batch(entries: &[WalEntry]) -> Result<(), WalError>;
pub fn flush() -> Result<(), WalError>;
pub fn sync() -> Result<(), WalError>;
pub fn position() -> Result<u64, WalError>;
pub fn path() -> Result<PathBuf, WalError>;
```

- `init_wal_manager` must be called exactly once per process. A second call
  returns `WAL manager already initialized`.
- `wal_manager()` panics if called before initialization.
- Convenience functions (`append`, `flush`, etc.) lock the mutex, perform the
  operation, and return.

### 5.2 Async integration

Because `init_wal_manager` performs synchronous file I/O, it is called from
[`trench/src/api/server.rs`](../../trench/src/api/server.rs) inside
`tokio::task::spawn_blocking`:

```rust
let wal_path = config.wal_path.clone();
tokio::task::spawn_blocking(move || init_wal_manager(&wal_path))
    .await
    .map_err(|err| format!("WAL manager init panicked: {err}"))?
    .map_err(|err| format!("failed to initialize WAL manager: {err}"))?;
```

This prevents the async runtime worker thread from blocking on directory
creation and file open.

---

## 7. Configuration

The WAL path is read from `config.trench`:

```toml
WalPath="data/xyz/wal.log"
```

The config parser strips surrounding quotes, so the value passed to
`init_wal_manager` is the clean path `data/xyz/wal.log`.

If `WalPath` is omitted, the parser falls back to `data/{id}/wal.log`.

---

## 8. Error handling

`WalError` covers:

| Variant | Meaning |
|---------|---------|
| `Io(io::Error)` | OS-level file I/O failure. |
| `InvalidPayload(String)` | Payload too large or misuse of the API. |
| `UnexpectedEof` | Read reached end-of-file unexpectedly. |
| `CorruptEntry { position, reason }` | Entry checksum / length mismatch. |
| `InvalidHeader { reason }` | WAL header missing or malformed. |
| `UnsupportedVersion { found, expected }` | WAL file version mismatch. |
| `PathMissing` | WAL path has no parent directory. |

All `init_wal_manager` failures are propagated as `Box<dyn Error>` from
`trench::api::run_server`, causing the server to exit with a non-zero status.

---

## 9. Known limitations & next steps

| Area | Status | Notes |
|------|--------|-------|
| Binary entry format | ❌ Not started | Currently debug text; define length-prefixed + checksum frame. |
| Header write on create | ❌ Not started | `init` reserves 8 bytes but does not write a header. |
| Recovery / replay | ❌ Not started | No reader or replay logic yet. |
| Encryption | ❌ Not started | `encrypt_data` is a no-op placeholder. |
| Compression | ❌ Not started | Future optimization. |
| Compaction / rotation | ❌ Not started | WAL will grow unbounded until added. |
| Batched flush | ❌ Not started | Every dispatch flushes; batch for write throughput. |
| Integration with handlers | ✅ Done | Put/update/delete handlers publish events; dispatcher appends to WAL. |

### Roadmap to production WAL

1. **Binary frame format** — replace text serialization with length-prefixed
   entries: `| entry_len: u32 | checksum: u32 | op: u8 | key_len: u16 | key |
   payload_len: u32 | payload |`.
2. **Header on create** — write a magic number + version into the reserved
   8-byte header during `WALWriter::init`.
3. **WAL reader / replay** — add `WALReader` that validates checksums and
   replays entries into the in-memory store on startup.
4. **Encryption** — replace the `encrypt_data` placeholder with a real cipher
   (e.g. AES-GCM) using a node key.
5. **Compaction / rotation** — periodically rewrite the WAL to drop obsolete
   entries (e.g. a later delete for the same key) and cap file size.
6. **Batching + group commit** — flush on a timer or after N entries instead of
   every dispatch to reduce fsync pressure.
7. **Snapshot integration** — combine WAL truncation with periodic memory
   snapshots for fast recovery.

---

## 10. Basic usage

```rust
use storage::walmanager::{init_wal_manager, append, flush, WalEntry};

init_wal_manager("data/xyz/wal.log")?;

let entry = WalEntry {
    payload: b"insert:users:alice".to_vec(),
    key: Some(b"users:alice".to_vec()),
    operation: Some(1),
    checksum: None,
};

append(&entry)?;
flush()?;
```

# TrenchDB Transport Protocol

Build a lightweight, secure, multiplexed transport protocol in Rust.

The transport is intended for node-to-node communication in a distributed system and will later carry
replication traffic, status updates, snapshots, and cluster messages.

The transport itself must remain application-agnostic.

Do NOT implement replication logic.

Do NOT implement business messages.

The transport only moves opaque byte payloads.

---

## Design Goals

- Lightweight
- Secure (TLS/mTLS using rustls)
- Long-lived connections
- Multiplexed streams
- Binary protocol
- Magic-prefixed, length-prefixed framing
- Versioned protocol
- Minimal allocations
- High throughput
- Easy to extend

### Target

- 1000+ requests/sec per node
- Persistent connections
- Multiple logical streams per connection

---

## Architecture

```
Application Layer
        ↑
Transport Streams
        ↑
Transport Connection
        ↑
Frame Encoder/Decoder
        ↑
TLS (rustls)
        ↑
TCP
```

The transport must not know about application message types.

Applications send and receive raw bytes.

---

## Project Structure

```
src/
├── lib.rs
│
├── frame/
│   ├── mod.rs
│   ├── header.rs
│   ├── frame.rs
│   ├── encoder.rs
│   └── decoder.rs
│
├── transport/
│   ├── mod.rs
│   ├── connection.rs
│   ├── stream.rs
│   └── manager.rs
│
├── tls/
│   ├── mod.rs
│   └── config.rs
│
├── protocol/
│   ├── mod.rs
│   ├── handshake.rs
│   └── capabilities.rs
│
└── error.rs
```

---

## Protocol Header

Every frame begins with a fixed-size binary header.

```rust
/// Magic prefix to detect frame boundaries and reject garbage connections early.
/// Bytes: [0x54, 0x52, 0x4E, 0x43] = "TRNC"
pub const FRAME_MAGIC: [u8; 4] = [0x54, 0x52, 0x4E, 0x43];

#[repr(C)]
pub struct Header {
    pub magic: [u8; 4],   // Must equal FRAME_MAGIC
    pub length: u32,       // Payload length in bytes, NOT including header
    pub stream_id: u32,    // Stream this frame belongs to
    pub flags: u16,        // See Flag Bits section
    pub frame_type: u8,    // See FrameType enum
    pub version: u8,       // Protocol version; current = 1
}
```

### Requirements

- Big-endian encoding for all multi-byte fields
- Exactly 16 bytes (magic[4] + length[4] + stream_id[4] + flags[2] + frame_type[1] + version[1])
- Reject frames with invalid magic
- Validate version against supported range
- Validate frame length; reject if `length > MAX_FRAME_SIZE`
- TLS provides transport-level integrity; magic prefix provides fast frame boundary detection

```rust
pub const CURRENT_VERSION: u8 = 1;
pub const MIN_SUPPORTED_VERSION: u8 = 1;
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MiB
```

---

## Frame Types

```rust
#[repr(u8)]
pub enum FrameType {
    Open     = 1,  // Open a new logical stream
    Data     = 2,  // Carry opaque payload bytes
    Close    = 3,  // Graceful half-close of a stream
    Reset    = 4,  // Abrupt stream termination (no waiting for ACK)
    Ping     = 5,  // Heartbeat request
    Pong     = 6,  // Heartbeat response
    Window   = 7,  // Flow control window update
    Error    = 8,  // Stream or connection error
    Settings = 9,  // Dynamic configuration exchange (post-handshake)
    Hello    = 10, // Handshake: client → server
    Welcome  = 11, // Handshake: server → client
}
```

### Frame Type Notes

- `Reset` provides abrupt stream termination without waiting for a `Close` acknowledgement.
  Use this when a stream is abandoned (e.g., request cancelled, timeout).
- `Settings` allows dynamic reconfiguration of per-connection parameters after handshake.
  Sent on stream_id=0 only.
- `Hello` and `Welcome` are sent on stream_id=0 before any user streams are opened.

---

## Flag Bits

Flags are a `u16` field. Bits are numbered 0 (LSB) to 15 (MSB).

| Bit | Name       | Applicable Frame Types    | Meaning                                                             |
|-----|------------|---------------------------|---------------------------------------------------------------------|
| 0   | FIN        | Close, Data               | Final frame for this stream; no more data will follow              |
| 1   | ACK        | Close, Reset              | Acknowledgement of a Close or Reset received from the remote side  |
| 2   | CONTROL    | Any                       | Frame is a control frame; must be processed before user data       |
| 3   | RESERVED   | —                         | Must be 0; reject frames with this bit set                         |
| 4–15| RESERVED   | —                         | Must be 0; reserved for future use; reject if non-zero             |

---

## Frame Structure

```rust
pub struct Frame {
    pub header: Header,
    pub payload: Bytes, // zero-copy via `bytes::Bytes`
}
```

### Requirements

- Payload may be empty (`length == 0` in header)
- `Data` frame carries arbitrary bytes; transport never interprets payload
- Use `bytes::Bytes` for payload to avoid copies when routing frames to streams
- `Reset`, `Close` with `ACK` bit, `Ping`, `Pong` frames MUST have empty payloads

---

## Encoder

```rust
pub fn encode(frame: &Frame) -> Bytes
```

### Requirements

- Write magic prefix
- Encode header fields in big-endian
- Append payload bytes
- No unsafe code
- Return `Bytes` to allow zero-copy chaining with writev-style sends

---

## Decoder

```rust
pub fn decode(buf: &[u8]) -> Result<(Frame, usize), TransportError>
```

Returns the decoded frame and the number of bytes consumed from `buf`.

### Requirements

- Verify magic prefix first; return `InvalidMagic` if mismatch
- Validate version is within `[MIN_SUPPORTED_VERSION, CURRENT_VERSION]`
- Validate `length <= MAX_FRAME_SIZE`
- Validate reserved flag bits are zero
- Return `NeedMoreData` if buffer is shorter than `16 + length`
- Reject unknown `frame_type` values with `InvalidFrame`

---

## TLS

Use:

- `rustls`
- `tokio-rustls`

### Requirements

- TLS 1.3 minimum; reject TLS 1.2 and below
- TLS server config
- TLS client config
- mTLS support (client certificate required for cluster peers)
- Certificate validation against a pinned CA bundle
- Extract peer identity from the TLS peer certificate

```rust
/// Node identity extracted from the peer's TLS certificate.
/// node_id is the Common Name (CN) from the peer certificate's Subject field.
/// Format: UUID v4 string, e.g. "550e8400-e29b-41d4-a716-446655440000"
/// Reject connections where CN is not a valid UUID v4.
pub struct PeerIdentity {
    pub node_id: Uuid,         // Parsed from certificate CN
    pub cert_fingerprint: [u8; 32], // SHA-256 of the DER-encoded certificate
}
```

---

## Connection Layer

```rust
pub struct Connection<S> {
    stream: S,
    read_buf: BytesMut,   // Reused across recv_frame calls
    peer_identity: Option<PeerIdentity>,
}
```

Where `S: AsyncRead + AsyncWrite + Unpin + Send`.

```rust
impl<S: AsyncRead + AsyncWrite + Unpin + Send> Connection<S> {
    pub async fn send_frame(&mut self, frame: &Frame) -> Result<(), TransportError>;
    pub async fn recv_frame(&mut self) -> Result<Frame, TransportError>;
}
```

### Requirements

- Read exact header size (16 bytes) first, then exact payload size
- Handle partial reads with `read_buf` accumulation
- Handle partial writes with retry loop
- Reuse `read_buf` across calls; never shrink below 64 KiB
- Control frames (`CONTROL` flag set) must be delivered before queued data frames

---

## Multiplexed Streams

Implement logical streams over a single TLS connection.

### Stream ID Allocation

```
stream_id = 0       → reserved for control (handshake, settings, connection-level errors)
stream_id = 1..u32::MAX → user streams
```

Stream IDs are allocated monotonically and NEVER reused within a connection lifetime.
When IDs are exhausted (`next_id` would overflow), close the connection gracefully and reconnect.
The peer that opened the connection allocates odd IDs; the accepting peer allocates even IDs.
This prevents collision without coordination (same convention as HTTP/2).

```rust
pub struct Stream {
    pub id: u32,
    pub state: StreamState,
    pub send_window: i64,   // Remaining bytes we are allowed to send
    pub recv_window: i64,   // Remaining bytes remote is allowed to send us
}

pub enum StreamState {
    Open,
    HalfClosedLocal,   // We sent Close/FIN; waiting for remote Close
    HalfClosedRemote,  // Remote sent Close/FIN; we can still send
    Closed,
    Reset,
}

pub struct StreamManager {
    streams: HashMap<u32, Stream>,
    next_local_id: u32,
}
```

### StreamManager Responsibilities

- Allocate stream IDs (odd or even depending on connection role)
- Track active streams and their state machine
- Route incoming frames to the correct stream receiver
- Enforce stream state transitions; reject frames for streams in invalid states
- Clean up closed/reset streams and release their resources
- Prioritize routing of frames on stream_id=0 (control stream) over user streams

---

## Stream Lifecycle

```
Initiator                        Responder
    │                                │
    │──── OPEN(stream_id) ──────────▶│
    │                                │
    │──── DATA(stream_id) ──────────▶│
    │◀─── DATA(stream_id) ───────────│
    │                                │
    │  Graceful close:               │
    │──── CLOSE(FIN) ───────────────▶│  HalfClosedLocal
    │◀─── CLOSE(FIN, ACK) ───────────│  Closed (both sides)
    │                                │
    │  Abrupt termination:           │
    │──── RESET(stream_id) ─────────▶│  Both sides immediately free the stream
```

- A stream is not usable until after `OPEN` is acknowledged implicitly by the first `DATA` or `CLOSE`
- A `RESET` on either side immediately transitions both ends to `Closed`; no ACK needed
- Sending `DATA` on a `HalfClosedLocal` stream is a protocol error

---

## Handshake

Immediately after TLS handshake completes, before any user streams are opened, both sides must
complete the transport handshake on stream_id=0.

### Client sends HELLO

```rust
pub struct Hello {
    pub version: u8,          // Highest protocol version the client supports
    pub min_version: u8,      // Lowest protocol version the client accepts
    pub capabilities: u32,    // Bitfield of requested capabilities (see Capabilities)
    pub max_frame_size: u32,  // Max frame size the client can receive
    pub initial_window: u32,  // Per-stream initial receive window size
}
```

Hello is encoded as the payload of a `Hello` frame on stream_id=0.

### Server responds WELCOME

```rust
pub struct Welcome {
    pub version: u8,           // Negotiated version (must be within client's [min,max] range)
    pub capabilities: u32,     // Accepted capabilities (subset of Hello.capabilities)
    pub max_frame_size: u32,   // Max frame size the server can receive
    pub initial_window: u32,   // Per-stream initial receive window size
}
```

### Rejection: server sends ERROR then closes

If the server cannot negotiate a compatible version or capabilities, it MUST:

1. Send an `Error` frame on stream_id=0 with error code `HANDSHAKE_REJECTED`
2. Close the TLS connection

The client MUST NOT open any user streams until `Welcome` is received.

### Version Negotiation Rules

- If `Hello.min_version > CURRENT_VERSION`: server rejects with `InvalidVersion`
- If `Hello.version < MIN_SUPPORTED_VERSION`: server rejects with `InvalidVersion`
- Negotiated version = `min(Hello.version, CURRENT_VERSION)`

---

## Capabilities

```rust
bitflags::bitflags! {
    pub struct Capabilities: u32 {
        const MULTIPLEXING  = 0b0000_0001; // Multiple logical streams per connection
        const FLOW_CONTROL  = 0b0000_0010; // WINDOW frame based flow control
        const MTLS          = 0b0000_0100; // Mutual TLS peer authentication
        const COMPRESSION   = 0b0000_1000; // Reserved; do not implement yet
    }
}
```

- `MULTIPLEXING` and `FLOW_CONTROL` are mandatory; reject connections that do not advertise both
- `MTLS` is required for cluster peers; optional for read-only observers
- `COMPRESSION` is reserved; must not be activated in this implementation
- Negotiated capabilities = `Hello.capabilities & Welcome.capabilities`
- Store negotiated capabilities on the `Connection` struct

---

## Flow Control

Flow control is bidirectional and per-stream.

```rust
WINDOW(stream_id, increment: u32)
```

### Rules

- Initial window size is exchanged in `Hello`/`Welcome` (default: 65536 bytes)
- A sender MUST NOT send more bytes than its current `send_window` for that stream
- When a receiver has consumed data and is ready to accept more, it sends a `WINDOW` frame
  with the number of additional bytes it is willing to accept (increment, not absolute)
- The sender adds the increment to its `send_window`
- A `send_window` of 0 means the sender MUST block until a `WINDOW` frame arrives
- `send_window` is a `i64` internally; if it goes negative due to a bug, treat as 0
- Connection-level flow control is not implemented; per-stream only

Do not implement advanced congestion control.

---

## Settings

After handshake, either peer may send a `Settings` frame on stream_id=0 to adjust connection
parameters dynamically.

```rust
pub struct SettingsEntry {
    pub key: u16,
    pub value: u64,
}

pub struct Settings {
    pub entries: Vec<SettingsEntry>,
}
```

### Defined Settings Keys

| Key  | Name                    | Default  | Description                              |
|------|-------------------------|----------|------------------------------------------|
| 0x01 | INITIAL_WINDOW_SIZE     | 65536    | New default window for future streams    |
| 0x02 | MAX_FRAME_SIZE          | 16777216 | Maximum frame payload the sender accepts |
| 0x03 | PING_INTERVAL_MS        | 5000     | Heartbeat interval in milliseconds       |
| 0x04 | PING_TIMEOUT_MS         | 15000    | Heartbeat timeout in milliseconds        |

- Unknown keys MUST be ignored (forward compatibility)
- Settings take effect immediately upon receipt
- Settings do not affect already-open streams

---

## Heartbeats

```rust
PING(stream_id=0, payload=[u8; 8])  // 8-byte opaque echo token
PONG(stream_id=0, payload=[u8; 8])  // Must echo the PING payload exactly
```

### Requirements

- Sent on stream_id=0 only
- Configurable interval (default: 5000 ms, settable via `Settings`)
- Configurable timeout (default: 15000 ms, settable via `Settings`)
- If a PONG is not received within the timeout after sending PING, close the connection with `Timeout`
- Unanswered PINGs must not accumulate; send at most one outstanding PING at a time

---

## Error Frame Payload

```rust
pub struct ErrorPayload {
    pub error_code: u16,  // See ErrorCode enum
    pub stream_id: u32,   // 0 = connection-level error; non-zero = stream-level error
    pub message_len: u16, // Length of the UTF-8 message that follows
    // followed by `message_len` bytes of UTF-8 text
}

#[repr(u16)]
pub enum ErrorCode {
    Unknown            = 0,
    ProtocolError      = 1,  // Generic unrecoverable protocol violation
    InvalidVersion     = 2,  // Version negotiation failed
    InvalidFrame       = 3,  // Malformed frame received
    FrameTooLarge      = 4,  // Frame exceeded MAX_FRAME_SIZE
    StreamClosed       = 5,  // Frame received for an already-closed stream
    StreamReset        = 6,  // Stream was reset by the remote
    FlowControlViolation = 7, // Sender exceeded its send window
    HandshakeRejected  = 8,  // Handshake could not be completed
    Timeout            = 9,  // Peer failed to respond within timeout
    InternalError      = 10, // Implementation error (should not occur)
}
```

- A connection-level error (stream_id=0 in payload) MUST be followed by closing the TLS connection
- A stream-level error closes only the named stream; the connection remains open

---

## Error Handling

```rust
pub enum TransportError {
    InvalidMagic,
    InvalidVersion { got: u8, min: u8, max: u8 },
    InvalidFrame(String),
    FrameTooLarge { size: usize, max: usize },
    ConnectionClosed,
    TlsError(rustls::Error),
    StreamClosed(u32),
    StreamReset(u32),
    FlowControlViolation { stream_id: u32 },
    HandshakeRejected { code: ErrorCode, message: String },
    NeedMoreData,
    Timeout,
    Io(std::io::Error),
}
```

Avoid panics. All error paths must return a `TransportError`.

---

## Testing

### Unit Tests

- Header encoding (all fields, big-endian)
- Header decoding (valid and invalid magic, invalid version, oversized length)
- Flag bit validation (reserved bits rejected)
- Frame encoding round-trip
- Frame decoding with partial buffer (NeedMoreData)
- Invalid frame type rejection
- Stream state machine transitions
- Stream ID allocation (odd/even, exhaustion)
- Flow control window arithmetic (increment, block at zero)
- Error payload encoding/decoding

### Integration Tests

- TLS connection (server cert only)
- mTLS connection (client + server cert)
- PeerIdentity extraction and UUID validation
- Handshake success
- Handshake rejection (version mismatch)
- Handshake rejection (missing required capabilities)
- Multiplexed streams (open N streams, interleave sends, verify delivery)
- Ping/Pong round-trip
- Ping timeout detection
- Large payload transfer (> 1 MiB)
- Flow control: sender blocks when window exhausted, resumes after WINDOW frame
- Reset: abrupt stream termination
- Settings: dynamic window resize takes effect

---

## Non Goals

Do NOT implement:

- Replication
- Consensus
- Raft
- Application messages
- Serialization frameworks
- Compression (reserved bit only)
- QUIC
- HTTP/2
- gRPC
- Connection-level flow control (per-stream only)
- Advanced congestion control

The transport layer must only provide:

```rust
stream.send(bytes: Bytes) -> Result<(), TransportError>
stream.recv()             -> Result<Bytes, TransportError>
```

over a secure, multiplexed connection.

---

## Deliverables

Implement in phases:

**Phase 1** — Framing
- Frame header (16 bytes, magic prefix, flag bits defined)
- Encoder
- Decoder (with `NeedMoreData` support)

**Phase 2** — TLS Connection
- TLS server and client config
- mTLS
- `PeerIdentity` extraction (UUID from CN)
- `Connection<S>` send/recv

**Phase 3** — Stream Multiplexing
- Stream state machine
- `StreamManager` with odd/even ID allocation
- OPEN / DATA / CLOSE / RESET lifecycle

**Phase 4** — Handshake
- HELLO / WELCOME exchange
- Version and capability negotiation
- Rejection path with ERROR frame

**Phase 5** — Flow Control
- Bidirectional per-stream WINDOW frames
- Send blocking when window exhausted

**Phase 6** — Settings & Heartbeats
- SETTINGS frame with defined keys
- PING / PONG with configurable interval and timeout

**Phase 7** — Integration Tests
- Full test suite as specified above

---

Favor simplicity, correctness, and maintainability over feature richness.
The resulting transport should feel like a minimal HTTP/2-style stream multiplexer
built specifically for cluster communication in TrenchDB.
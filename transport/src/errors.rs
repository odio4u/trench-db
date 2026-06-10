

#[derive(Debug)]
pub struct ErrorPayload {
    pub error_code: u16,
    pub stream_id: u32,
    pub message_len: u16,
}


#[derive(Debug, Clone, Copy)]
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

#[derive(Debug)]
pub enum TransportError {
    InvalidMagic,
    InvalidVersion { got: u8, min: u8, max: u8 },
    InvalidFrame(String),
    FrameTooLarge { size: usize, max: usize },
    ConnectionClosed,
    // TlsError(rustls::Error),
    StreamClosed(u32),
    StreamReset(u32),
    FlowControlViolation { stream_id: u32 },
    HandshakeRejected { code: ErrorCode, message: String },
    NeedMoreData,
    Timeout,
    Io(std::io::Error),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::InvalidMagic => write!(f, "Invalid magic number in header"),
            TransportError::InvalidVersion { got, min, max } => write!(f, "Invalid version: got {}, expected between {} and {}", got, min, max),
            TransportError::InvalidFrame(msg) => write!(f, "Invalid frame: {}", msg),
            TransportError::FrameTooLarge { size, max } => write!(f, "Frame too large: size {}, max {}", size, max),
            TransportError::ConnectionClosed => write!(f, "Connection closed"),
            // TransportError::TlsError(e) => write!(f, "TLS error: {}", e),
            TransportError::StreamClosed(id) => write!(f, "Stream {} is closed", id),
            TransportError::StreamReset(id) => write!(f, "Stream {} was reset by remote", id),
            TransportError::FlowControlViolation { stream_id } => write!(f, "Flow control violation on stream {}", stream_id),
            TransportError::HandshakeRejected { code, message } => write!(f, "Handshake rejected: {:?} - {}", code, message),
            TransportError::NeedMoreData => write!(f, "Need more data to parse frame"),
            TransportError::Timeout => write!(f, "Operation timed out"),
            TransportError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}
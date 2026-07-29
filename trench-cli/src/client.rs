use std::error::Error;
use std::{fmt};
use std::time::Duration;

use byteser::ByteSerializable;
use tokio::net::TcpStream;
use tokio::time::sleep;
use transport::errors::{ErrorPayload, TransportError};
use transport::frame::frame::Frametype;
use transport::server::{RequestEnvelope, ResponseEnvelope};
use transport::tcp::{connection::Connection, manager::Role, manager::StreamManager};

/// Thread-safe boxed error used throughout the CLI.
pub type CliError = Box<dyn Error + Send + Sync>;
pub type CliResult<T> = Result<T, CliError>;

/// Convert a transport-layer error into a thread-safe CLI error.
pub fn boxed_err<E: fmt::Display>(err: E) -> CliError {
    err.to_string().into()
}

/// Persistent TCP client to the storage server with automatic reconnect.
///
/// Keeps a single `StreamManager` (and its underlying TCP stream) alive for the
/// lifetime of the CLI session.  If the connection drops, the manager is
/// discarded and a fresh one is built with exponential backoff.
pub struct PersistentClient {
    pub host: String,
    pub port: u16,
    inner: Option<StreamManager<TcpStream>>,
    max_retries: u32,
}

impl PersistentClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            inner: None,
            max_retries: 3,
        }
    }

    /// Create a new TCP connection and complete the transport handshake.
    async fn connect(&mut self) -> CliResult<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr).await.map_err(boxed_err)?;
        let mut manager = StreamManager::new(Connection::new(stream), Role::Initiator);
        manager.start_handshake().await.map_err(boxed_err)?;
        self.inner = Some(manager);
        Ok(())
    }

    /// Ensure a transport manager exists, reconnecting on failure with backoff.
    async fn ensure_connected(&mut self) -> CliResult<()> {
        if self.inner.is_some() {
            return Ok(());
        }

        let mut last_err: Option<CliError> = None;
        for attempt in 0..=self.max_retries {
            match self.connect().await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    eprintln!("[client] connection failed (attempt {}): {}", attempt + 1, err);
                    last_err = Some(err);
                }
            }
            if attempt < self.max_retries {
                sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
        }
        Err(last_err.unwrap_or_else(|| boxed_err("failed to connect")))
    }

    /// Send one request over a fresh logical stream and return the decoded response.
    /// Reconnects and retries the whole exchange if the connection drops.
    pub async fn send<R: ByteSerializable>(
        &mut self,
        action: &str,
        payload: Vec<u8>,
    ) -> CliResult<R> {
        let request = RequestEnvelope {
            action: action.to_string(),
            payload,
        };
        let mut last_err: Option<CliError> = None;

        for attempt in 0..=self.max_retries {
            self.ensure_connected().await?;

            let manager = self.inner.as_mut().expect("manager must be connected");
            match send_request(manager, &request).await {
                Ok(response) => {
                    let mut slice: &[u8] = &response.payload;
                    return R::byte_deserialize(&mut slice)
                        .map_err(|e| boxed_err(format!("failed to decode response: {e}")));
                }
                Err(err) => {
                    eprintln!("[client] send failed (attempt {}): {}", attempt + 1, err);
                    self.inner = None; // discard broken manager; reconnect on next attempt
                    last_err = Some(boxed_err(err));
                }
            }
            if attempt < self.max_retries {
                sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
        }

        Err(last_err.unwrap_or_else(|| boxed_err("send failed after retries")))
    }

    pub async fn close(&mut self) -> CliResult<()> {
        self.inner.take();
        Ok(())
    }
}

/// Send a single `RequestEnvelope` over an already-established manager and wait
/// for the matching response on the same logical stream.
async fn send_request(
    manager: &mut StreamManager<TcpStream>,
    request: &RequestEnvelope,
) -> Result<ResponseEnvelope, TransportError> {
    let mut request_bytes = Vec::new();
    request.byte_serialize(&mut request_bytes);

    let stream_id = manager.open_stream().await?;
    manager.send_data(stream_id, request_bytes).await?;
    manager.close_stream(stream_id).await?;
    manager.flush().await?;

    let response = loop {
        let frame = manager.recv_frame().await?;
        if frame.header.stream_id != stream_id {
            continue;
        }

        match frame.header.frame_type {
            Frametype::Data => {
                let mut slice: &[u8] = &frame.payload;
                let response: ResponseEnvelope = ResponseEnvelope::byte_deserialize(&mut slice)
                    .map_err(|e| TransportError::InternalError(format!("Failed to deserialize response: {}", e)))?;
                break response;
            }
            Frametype::Close => return Err(TransportError::ConnectionClosed),
            Frametype::Reset => return Err(TransportError::StreamReset(stream_id)),
            Frametype::Error => {
                let error_payload = ErrorPayload::decode(&frame.payload)?;
                return Err(TransportError::RemoteError(
                    error_payload.error_code,
                    error_payload.stream_id,
                    error_payload.message,
                ));
            }
            _ => continue,
        }
    };

    Ok(response)
}

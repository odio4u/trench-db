use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::{errors::TransportError, frame::frame::Frame, frame::encoder::encode, frame::decoder::decode};

const MAX_BUFFER_SIZE: usize = 256 * 1024;
const MIN_BUFFER_SIZE: usize = 64 * 1024;
const CHUNK_SIZE: usize = 8 * 1024;


pub struct Connection<T> {
    stream: T,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
}

impl <T: AsyncRead + AsyncWrite + Unpin> Connection<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            read_buffer: BytesMut::with_capacity(MIN_BUFFER_SIZE),
            write_buffer: BytesMut::with_capacity(MIN_BUFFER_SIZE),
        }
    }

    pub fn buffer_frame(&mut self, frame: &Frame) -> Result<(), TransportError>  {
        if self.write_buffer.len() > MAX_BUFFER_SIZE {
            return Err(TransportError::BufferOverflow);
        }

        let encoder = encode(frame)?;
        self.write_buffer.extend_from_slice(&encoder);
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), TransportError> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        self.stream.write_all(&self.write_buffer).await?;
        self.write_buffer.clear();
        Ok(())
    }

    pub async fn send_frame(&mut self, frame: &Frame) -> Result<(), TransportError> {
        self.buffer_frame(frame)?;
        self.flush().await
    }
 
    pub fn write_buf_len(&self) -> usize {
        self.write_buffer.len()
    }

    pub async fn recv_frame(&mut self) -> Result<Frame, TransportError> {
        loop {

            match decode(&self.read_buffer[..]) {
                Ok((frame, consumed)) => {
                    self.read_buffer.advance(consumed);
                    return Ok(frame);
                }
 
                Err(TransportError::NeedMoreData) => {
                    // Not enough bytes yet for a complete frame.
                    // Fall through to the read below.
                }
 
                Err(e) => {
                    return Err(e);
                }
            }
 
            self.read_buffer.reserve(CHUNK_SIZE);
 
            let n = self.stream.read(&mut self.read_buffer).await?;
            if n == 0 {
                return Err(TransportError::ConnectionClosed);
            }
        }
    }
 
    pub fn read_buf_len(&self) -> usize {
        self.read_buffer.len()
    }


}
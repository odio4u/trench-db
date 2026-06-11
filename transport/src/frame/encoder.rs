use bytes::{BufMut, Bytes, BytesMut};
use crate::frame;


pub fn encode(frame: &frame::frame::Frame) -> Result<Bytes, crate::errors::TransportError> {
    if frame.payload.len() != frame.header.payload_length as usize {
        return Err(crate::errors::TransportError::InvalidFrame(format!(
            "Payload length mismatch: header says {}, but actual payload is {}",
            frame.header.payload_length,
            frame.payload.len()
        )));
    }

    let total_size = frame::header::HEADER_SIZE + frame.payload.len();
    let mut buffer = BytesMut::with_capacity(total_size);

    buffer.extend_from_slice(&frame.header.magic);
    buffer.put_u8(frame.header.version);
    buffer.extend_from_slice(&frame.header.flags.to_be_bytes());
    buffer.extend_from_slice(&frame.header.stream_id.to_be_bytes());
    buffer.extend_from_slice(&frame.header.payload_length.to_be_bytes());
    buffer.put_u8(frame.header.frame_type as u8);
    buffer.extend_from_slice(&frame.payload);

    Ok(buffer.freeze())
}
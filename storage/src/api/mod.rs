pub mod collection;
pub mod requests;
pub mod server;
pub mod table;

pub use server::{build_actions, run_server};

use std::sync::Arc;

use byteser::ByteSerializable;
use transport::errors::TransportError;
use crate::traits::Table;


pub const MAX_TABLE_NAME_LEN: usize = 128;
pub const MAX_KEY_NAME_LEN: usize = 256;
pub const MAX_VALUE_LEN: usize = 4 * 1024 * 1024; // 4MB

/// Shared table registry type handed to every handler.
pub type SharedStore = Arc<dyn Table<String, Vec<u8>> + Send + Sync>;

pub fn validate_name(name: &str, field: &str) -> Result<(), TransportError> {
    if name.is_empty() {
        return Err(TransportError::InternalError(format!("{field} cannot be empty")));
    }
    if name.len() > MAX_TABLE_NAME_LEN {
        return Err(TransportError::InternalError(format!("{field} is too long: max {} bytes", MAX_TABLE_NAME_LEN)));
    }
    if !name.chars().all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.')) {
        return Err(TransportError::InternalError(format!("{field} contains invalid characters")));
    }
    Ok(())
}

pub fn validate_key(key: &str) -> Result<(), TransportError> {
    if key.is_empty() {
        return Err(TransportError::InternalError("key cannot be empty".into()));
    }
    if key.len() > MAX_KEY_NAME_LEN {
        return Err(TransportError::InternalError(format!("key is too long: max {} bytes", MAX_KEY_NAME_LEN)));
    }
    if !key.chars().all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.')) {
        return Err(TransportError::InternalError("key contains invalid characters".into()));
    }
    Ok(())
}

pub fn validate_value(value: &[u8]) -> Result<(), TransportError> {
    if value.len() > MAX_VALUE_LEN {
        return Err(TransportError::InternalError(format!("value is too large: max {} bytes", MAX_VALUE_LEN)));
    }
    Ok(())
}
pub fn is_metadata_table(name: &str) -> bool {
    name == crate::metadata::metadata::METADATA_TABLE
}

pub fn validate_not_metadata_table(name: &str, action: &str) -> Result<(), TransportError> {
    if is_metadata_table(name) {
        return Err(TransportError::InternalError(format!("{action} is not allowed on the reserved metadata table")));
    }
    Ok(())
}
pub fn decode<T: ByteSerializable>(payload: Vec<u8>) -> Result<T, TransportError> {
    let mut slice: &[u8] = &payload;
    T::byte_deserialize(&mut slice).map_err(|e| TransportError::InternalError(format!("decode failed: {e}")))
}

pub fn encode<T: ByteSerializable>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.byte_serialize(&mut bytes);
    bytes
}

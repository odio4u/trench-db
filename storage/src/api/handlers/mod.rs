//! `transport::server::Handler` implementations, one per storage action.
//!
//! Each handler decodes its request, selects the requested table, calls the
//! appropriate storage operation, and encodes the response.

pub mod metrics;
pub mod records;
pub mod tables;

use byteser::ByteSerializable;
use transport::errors::TransportError;

use crate::traits::Table;

pub use metrics::MetricsHandler;
pub use records::{ContainsHandler, DeleteHandler, GetHandler, PutHandler, UpdateHandler};
pub use tables::{AddTableHandler, RemoveTableHandler};

const MAX_TABLE_NAME_LEN: usize = 128;
const MAX_KEY_NAME_LEN: usize = 256;
const MAX_VALUE_LEN: usize = 4 * 1024 * 1024; // 4MB

/// Shared table registry type handed to every handler.
pub type SharedStore = std::sync::Arc<dyn Table<String, Vec<u8>> + Send + Sync>;

fn validate_name(name: &str, field: &str) -> Result<(), TransportError> {
    if name.is_empty() {
        return Err(TransportError::InternalError(format!(
            "{field} cannot be empty"
        )));
    }
    if name.len() > MAX_TABLE_NAME_LEN {
        return Err(TransportError::InternalError(format!(
            "{field} is too long: max {} bytes",
            MAX_TABLE_NAME_LEN
        )));
    }
    if !name
        .chars()
        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.'))
    {
        return Err(TransportError::InternalError(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), TransportError> {
    if key.is_empty() {
        return Err(TransportError::InternalError("key cannot be empty".into()));
    }
    if key.len() > MAX_KEY_NAME_LEN {
        return Err(TransportError::InternalError(format!(
            "key is too long: max {} bytes",
            MAX_KEY_NAME_LEN
        )));
    }
    if !key
        .chars()
        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.'))
    {
        return Err(TransportError::InternalError(
            "key contains invalid characters".into(),
        ));
    }
    Ok(())
}

fn validate_value(value: &[u8]) -> Result<(), TransportError> {
    if value.len() > MAX_VALUE_LEN {
        return Err(TransportError::InternalError(format!(
            "value is too large: max {} bytes",
            MAX_VALUE_LEN
        )));
    }
    Ok(())
}

fn decode<T: ByteSerializable>(payload: Vec<u8>) -> Result<T, TransportError> {
    let mut slice: &[u8] = &payload;
    T::byte_deserialize(&mut slice)
        .map_err(|e| TransportError::InternalError(format!("decode failed: {e}")))
}

fn encode<T: ByteSerializable>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.byte_serialize(&mut bytes);
    bytes
}

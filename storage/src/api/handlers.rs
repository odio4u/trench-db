//! `transport::server::Handler` implementations, one per storage action.
//!
//! Each handler decodes its request, selects the requested table, calls the
//! appropriate storage operation, and encodes the response.

use std::sync::Arc;

use async_trait::async_trait;
use byteser::ByteSerializable;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::requests::{
    AddTableRequest, AddTableResponse, ContainsRequest, ContainsResponse, DeleteRequest, DeleteResponse,
    GetRequest, GetResponse, PutRequest, PutResponse, RemoveTableRequest, RemoveTableResponse, UpdateRequest,
    UpdateResponse,
};
use crate::traits::Table;

const MAX_TABLE_NAME_LEN: usize = 128;
const MAX_KEY_NAME_LEN: usize = 256;
const MAX_VALUE_LEN: usize = 4 * 1024 * 1024; // 4MB

fn validate_name(name: &str, field: &str) -> Result<(), TransportError> {
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

fn validate_key(key: &str) -> Result<(), TransportError> {
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

fn validate_value(value: &[u8]) -> Result<(), TransportError> {
    if value.len() > MAX_VALUE_LEN {
        return Err(TransportError::InternalError(format!("value is too large: max {} bytes", MAX_VALUE_LEN)));
    }
    Ok(())
}

/// Shared table registry type handed to every handler.
pub type SharedStore = Arc<dyn Table<String, Vec<u8>> + Send + Sync>;

fn decode<T: ByteSerializable>(payload: Vec<u8>) -> Result<T, TransportError> {
    let mut slice: &[u8] = &payload;
    T::byte_deserialize(&mut slice).map_err(|e| TransportError::InternalError(format!("decode failed: {e}")))
}

fn encode<T: ByteSerializable>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.byte_serialize(&mut bytes);
    bytes
}

pub struct GetHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for GetHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: GetRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_key(&request.key)?;

        let table = self
            .store
            .get(&request.table)
            .ok_or_else(|| TransportError::InternalError("table not found".into()))?;

        let value = table.get(&request.key).map(|value| (*value).clone());
        Ok(encode(&GetResponse { value }))
    }
}

pub struct PutHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for PutHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: PutRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_key(&request.key)?;
        validate_value(&request.value)?;

        let table = self.store.create(&request.table);
        table.insert(request.key, request.value);
        Ok(encode(&PutResponse { ok: true }))
    }
}

pub struct UpdateHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for UpdateHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: UpdateRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_key(&request.key)?;
        validate_value(&request.value)?;

        let table = self
            .store
            .get(&request.table)
            .ok_or_else(|| TransportError::InternalError("table not found".into()))?;
        table.update(request.key, request.value);
        Ok(encode(&UpdateResponse { ok: true }))
    }
}

pub struct DeleteHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for DeleteHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: DeleteRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_key(&request.key)?;

        let table = self
            .store
            .get(&request.table)
            .ok_or_else(|| TransportError::InternalError("table not found".into()))?;
        table.remove(&request.key);
        Ok(encode(&DeleteResponse { ok: true }))
    }
}

pub struct ContainsHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for ContainsHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: ContainsRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_key(&request.key)?;

        let exists = self
            .store
            .get(&request.table)
            .map(|table| table.contains(&request.key))
            .unwrap_or(false);
        Ok(encode(&ContainsResponse { exists }))
    }
}

pub struct AddTableHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for AddTableHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: AddTableRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        self.store.create(&request.table);
        Ok(encode(&AddTableResponse { ok: true }))
    }
}

pub struct RemoveTableHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for RemoveTableHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: RemoveTableRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        let existed = self.store.get(&request.table).is_some();
        self.store.clear(&request.table);
        Ok(encode(&RemoveTableResponse { ok: existed }))
    }
}

//! `transport::server::Handler` implementations, one per storage action.
//!
//! Each handler just decodes its request, calls into `Storage`, and encodes
//! the response — no business logic lives here (see
//! `doc/storage/storage.md#separation-of-concerns`).

use std::sync::Arc;

use async_trait::async_trait;
use byteser::ByteSerializable;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::requests::{
    ContainsRequest, ContainsResponse, DeleteRequest, DeleteResponse, GetRequest, GetResponse, PutRequest,
    PutResponse, UpdateRequest, UpdateResponse,
};
use crate::traits::Storage;

/// Shared store type handed to every handler.
pub type SharedStore = Arc<dyn Storage<String, Vec<u8>> + Send + Sync>;

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
        let value = self.store.get(&request.key).map(|value| (*value).clone());
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
        self.store.insert(request.key, request.value);
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
        self.store.update(request.key, request.value);
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
        self.store.remove(&request.key);
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
        let exists = self.store.contains(&request.key);
        Ok(encode(&ContainsResponse { exists }))
    }
}

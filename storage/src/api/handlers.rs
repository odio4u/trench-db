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
        let table = self.store.new(&request.table);
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
        let table = self.store.new(&request.table);
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
        let table = self.store.new(&request.table);
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
        let table = self.store.new(&request.table);
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
        let table = self.store.new(&request.table);
        let exists = table.contains(&request.key);
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
        self.store.new(&request.table);
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
        self.store.clear(&request.table);
        Ok(encode(&RemoveTableResponse { ok: true }))
    }
}

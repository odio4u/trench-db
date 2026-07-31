//! `transport::server::Handler` implementations for table-level actions.
//!
//! These handlers operate on records inside a table: get, put, update, delete,
//! and contains.

use async_trait::async_trait;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::{decode, encode, validate_key, validate_name, validate_not_metadata_table, validate_value, SharedStore};
use crate::api::requests::{
    ContainsRequest, ContainsResponse, DeleteRequest, DeleteResponse, GetRequest, GetResponse, PutRequest, PutResponse,
    UpdateRequest, UpdateResponse,
};

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
        validate_not_metadata_table(&request.table, "write")?;
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
        validate_not_metadata_table(&request.table, "update")?;
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
        validate_not_metadata_table(&request.table, "delete")?;
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

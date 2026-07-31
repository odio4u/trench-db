//! `transport::server::Handler` implementations for collection-level actions.
//!
//! These handlers operate on the table registry itself: creating and removing
//! tables.

use async_trait::async_trait;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::{decode, encode, validate_name, validate_not_metadata_table, SharedStore};
use crate::api::requests::{AddTableRequest, AddTableResponse, RemoveTableRequest, RemoveTableResponse};

pub struct AddTableHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for AddTableHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let request: AddTableRequest = decode(payload)?;
        validate_name(&request.table, "table")?;
        validate_not_metadata_table(&request.table, "create")?;
        if self.store.get(&request.table).is_some() {
            return Err(TransportError::InternalError("table already exists".into()));
        }
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
        validate_not_metadata_table(&request.table, "remove")?;
        let existed = self.store.get(&request.table).is_some();
        self.store.remove(&request.table);
        Ok(encode(&RemoveTableResponse { ok: existed }))
    }
}

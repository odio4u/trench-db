//! Handlers for table-level storage operations: add_table, remove_table.

use async_trait::async_trait;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::handlers::{SharedStore, decode, encode, validate_name};
use crate::api::requests::{
    AddTableRequest, AddTableResponse, RemoveTableRequest, RemoveTableResponse,
};

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
        self.store.remove(&request.table);
        Ok(encode(&RemoveTableResponse { ok: existed }))
    }
}

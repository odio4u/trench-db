use std::sync::Arc;

use byteser::ByteSerializable;
use transport::errors::TransportError;
use transport::server::Handler;

use storage::api::collection::AddTableHandler;
use storage::api::requests::{AddTableRequest, PutRequest};
use storage::api::table::PutHandler;
use storage::MemoryStore;

#[tokio::test]
async fn metadata_table_is_reserved_and_duplicate_create_is_rejected() {
    let store: Arc<MemoryStore<String, Vec<u8>>> = Arc::new(MemoryStore::new());

    let add_handler = AddTableHandler { store: store.clone() };
    let mut payload = Vec::new();
    AddTableRequest {
        table: "__metadata".to_string(),
    }
    .byte_serialize(&mut payload);

    let result = add_handler.call(payload).await;
    assert!(matches!(result, Err(TransportError::InternalError(_))), "reserved metadata table create should fail");

    let mut payload = Vec::new();
    AddTableRequest {
        table: "users".to_string(),
    }
    .byte_serialize(&mut payload);

    let result = add_handler.call(payload).await;
    assert!(result.is_ok(), "first table creation should succeed");

    let mut payload = Vec::new();
    AddTableRequest {
        table: "users".to_string(),
    }
    .byte_serialize(&mut payload);

    let result = add_handler.call(payload).await;
    assert!(matches!(result, Err(TransportError::InternalError(_))), "duplicate table creation should fail");

    let put_handler = PutHandler { store: store.clone() };
    let mut payload = Vec::new();
    PutRequest {
        table: "__metadata".to_string(),
        key: "node_id".to_string(),
        value: b"xyz".to_vec(),
    }
    .byte_serialize(&mut payload);

    let result = put_handler.call(payload).await;
    assert!(matches!(result, Err(TransportError::InternalError(_))), "writes to reserved metadata table should fail");
}

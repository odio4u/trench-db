//! Exit-criteria smoke test for Phase 2: a `ResilientClient` can `put` then
//! `get` a key over a real TCP connection to the storage handlers.
//!
//! `ResilientClient::send_message` performs a fresh handshake per call, so
//! (matching how `ResilientServer::run` expects one handshake per accepted
//! connection) each request here opens its own short-lived connection to the
//! server, exactly like a real client would use the `storage` binary.

use std::sync::Arc;

use byteser::ByteSerializable;
use storage::api::requests::{GetRequest, GetResponse, PutRequest, PutResponse};
use storage::MemoryStore;
use tokio::net::TcpListener;
use transport::client::resilient_client::ResilientClient;
use transport::server::{RequestEnvelope, ResponseEnvelope};

async fn send<Req: ByteSerializable, Resp: ByteSerializable>(addr: std::net::SocketAddr, action: &str, request: &Req) -> Resp {
    let mut payload = Vec::new();
    request.byte_serialize(&mut payload);

    let envelope = RequestEnvelope {
        action: action.to_string(),
        payload,
    };

    let mut client = ResilientClient::new(addr.ip().to_string(), addr.port());
    client.build_stream().await.expect("connect failed");

    let response: ResponseEnvelope = client.send_message(&envelope).await.expect("send_message failed");
    client.close().await.expect("close failed");

    let mut slice: &[u8] = &response.payload;
    Resp::byte_deserialize(&mut slice).expect("decode response failed")
}

#[tokio::test]
async fn put_then_get_roundtrip_over_tcp() {
    let store: Arc<MemoryStore<String, Vec<u8>>> = Arc::new(MemoryStore::new());

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind failed");
    let addr = listener.local_addr().expect("local_addr failed");

    tokio::spawn(async move {
        let actions = Arc::new(storage::api::build_actions(store));
        loop {
            let (socket, peer) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(_) => return,
            };
            let actions = actions.clone();
            tokio::spawn(async move {
                let server = transport::server::ResilientServer::new(socket, peer, actions);
                if let Err(err) = server.run().await {
                    eprintln!("[test server] connection error: {err}");
                }
            });
        }
    });

    let put_response: PutResponse = send(
        addr,
        "put",
        &PutRequest {
            table: "default".to_string(),
            key: "hello".to_string(),
            value: b"world".to_vec(),
        },
    )
    .await;
    assert!(put_response.ok);

    let get_response: GetResponse = send(
        addr,
        "get",
        &GetRequest {
            table: "default".to_string(),
            key: "hello".to_string(),
        },
    )
    .await;
    assert_eq!(get_response.value, Some(b"world".to_vec()));
}


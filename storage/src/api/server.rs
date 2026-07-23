//! Wires the storage `Handler`s to `transport`'s `Actions`/`ResilientServer`,
//! mirroring `interface/src/server.rs` exactly (see
//! `doc/storage/storage.md#communication-layer`).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use transport::server::{Actions, ResilientServer};

use crate::api::handlers::{ContainsHandler, DeleteHandler, GetHandler, PutHandler, SharedStore, UpdateHandler};

/// Registers the `get`/`put`/`update`/`delete`/`contains` actions against `store`.
pub fn build_actions(store: SharedStore) -> Actions {
    let mut actions = Actions::new();
    actions.register_action(
        "get",
        GetHandler {
            store: store.clone(),
        },
    );
    actions.register_action(
        "put",
        PutHandler {
            store: store.clone(),
        },
    );
    actions.register_action(
        "update",
        UpdateHandler {
            store: store.clone(),
        },
    );
    actions.register_action(
        "delete",
        DeleteHandler {
            store: store.clone(),
        },
    );
    actions.register_action("contains", ContainsHandler { store });
    actions
}

/// Binds `addr` and serves storage requests until an accept error occurs.
pub async fn run_server(addr: SocketAddr, store: SharedStore) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr).await?;
    let actions = Arc::new(build_actions(store));

    println!("[storage] listening on {addr}");

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("[storage] accepted connection from {peer_addr}");

        let actions = actions.clone();
        tokio::spawn(async move {
            let server = ResilientServer::new(socket, peer_addr, actions);
            if let Err(err) = server.run().await {
                eprintln!("[storage {peer_addr}] connection error: {err}");
            }
        });
    }
}

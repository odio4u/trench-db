//! Wires the storage `Handler`s to `transport`'s `Actions`/`ResilientServer`,
//! mirroring `interface/src/server.rs` exactly (see
//! `doc/storage/storage.md#communication-layer`).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use transport::server::{Actions, ResilientServer};

use crate::api::collection::{AddTableHandler, RemoveTableHandler};
use crate::api::SharedStore;
use crate::api::table::{ContainsHandler, DeleteHandler, GetHandler, PutHandler, UpdateHandler};
use crate::config::NodeConfig;
use crate::metadata::seed_metadata;

/// Registers the `get`/`put`/`update`/`delete`/`contains`/`add_table`/`remove_table` actions against `store`.
pub fn build_actions(store: SharedStore) -> Actions {
    let mut actions = Actions::new();
    actions.register_action("get",GetHandler {store: store.clone()});
    actions.register_action("put",PutHandler {store: store.clone()});
    actions.register_action("update",UpdateHandler {store: store.clone()});
    actions.register_action("delete",DeleteHandler {store: store.clone() });
    actions.register_action("contains", ContainsHandler { store: store.clone() });
    actions.register_action("add_table", AddTableHandler { store: store.clone() });
    actions.register_action("remove_table", RemoveTableHandler { store });
    actions
}

/// Binds `addr` and serves storage requests until an accept error occurs.
pub async fn run_server(addr: SocketAddr, store: SharedStore) -> Result<(), Box<dyn Error>> {
    let config = NodeConfig::from_file("config.trench")?;
    seed_metadata(&store, &config)?;

    println!("[storage] node started: {}", config.id);
    println!("[storage] node address: {}", config.node_address);
    println!("[storage] anchor address: {}", config.anchor_address);
    println!("[storage] region: {}", config.region);

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

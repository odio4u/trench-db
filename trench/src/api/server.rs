//! Wires the engine `Handler`s to `transport`'s `Actions`/`ResilientServer`,
//! mirroring `interface/src/server.rs` exactly (see
//! `doc/storage/storage.md#communication-layer`).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use transport::server::{Actions, ResilientServer};
use engine::walmanager::init_wal_manager;
use crate::api::collection::{AddTableHandler, RemoveTableHandler};
use crate::api::SharedStore;
use crate::api::table::{ContainsHandler, DeleteHandler, GetHandler, PutHandler, UpdateHandler};
use engine::config::NodeConfig;
use crate::identity::seed::seed_identity;
use crate::identity::node_identity;

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

/// Binds `addr` and serves engine requests until an accept error occurs.
pub async fn run_server(addr: SocketAddr, store: SharedStore) -> Result<(), Box<dyn Error>> {
    let config = NodeConfig::from_file("config.trench")?;

    println!("[engine] node started: {}", config.id);
    println!("[engine] node address: {}", config.node_address);
    println!("[engine] anchor address: {}", config.anchor_address);
    println!("[engine] region: {}", config.region);

    // Initialize the process-wide WAL writer before any operation can publish
    // a storage event. This performs synchronous file I/O, so run it off the
    // async runtime.
    let wal_path = config.wal_path.clone();
    tokio::task::spawn_blocking(move || init_wal_manager(&wal_path))
        .await
        .map_err(|err| format!("WAL manager init panicked: {err}"))?
        .map_err(|err| format!("failed to initialize WAL manager: {err}"))?;
    println!("[engine] WAL initialized at {}", config.wal_path);

    // Initialize node identity after the WAL is ready so the resulting storage
    // events can be appended to the log by the dispatcher.
    let node_identity = node_identity::NodeIdentity::new(true)?;
    println!("[engine] node identity created:\n{node_identity}");
    seed_identity(&store, &node_identity)?;

    let listener = TcpListener::bind(addr).await?;
    let actions = Arc::new(build_actions(store));

    println!("[engine] listening on {addr}");

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("[engine] accepted connection from {peer_addr}");

        let actions = actions.clone();
        tokio::spawn(async move {
            let server = ResilientServer::new(socket, peer_addr, actions);
            if let Err(err) = server.run().await {
                eprintln!("[engine {peer_addr}] connection error: {err}");
            }
        });
    }
}

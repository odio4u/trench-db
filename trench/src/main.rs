use std::{env, println};
use std::sync::Arc;

use storage::MemoryStore;
use trench::api::{run_server, SharedStore};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let bootstraped = args.iter().any(|arg| arg == "--bootstrap");

    let address = if bootstraped {
        args.iter()
            .position(|arg| arg == "--address")
            .and_then(|index| args.get(index + 1))
            .map(|s| s.as_str())
            .unwrap_or("127.0.0.1:7878")
    } else {
        "127.0.0.1:7878"
    };

    let addr: std::net::SocketAddr = address.parse().expect("invalid address");

    let store: SharedStore = Arc::new(MemoryStore::new());
    println!("[trench] starting storage server on {addr}");
    println!("[trench] bootstrap mode: {bootstraped}");

    if let Err(err) = run_server(addr, store).await {
        eprintln!("[trench] storage server failed: {err}");
        std::process::exit(1);
    }
}

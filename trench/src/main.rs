use std::env;
use std::sync::Arc;

use storage::MemoryStore;
use trench::api::{run_server, SharedStore};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let addr = if args.len() > 1 {
        match args[1].parse() {
            Ok(addr) => addr,
            Err(_) => {
                eprintln!("Usage: {} [ip:port]", args[0]);
                std::process::exit(1);
            }
        }
    } else {
        "127.0.0.1:7878".parse().expect("default address parse failed")
    };

    let store: SharedStore = Arc::new(MemoryStore::new());
    println!("[trench] starting storage server on {addr}");

    if let Err(err) = run_server(addr, store).await {
        eprintln!("[trench] storage server failed: {err}");
        std::process::exit(1);
    }
}

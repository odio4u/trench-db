use std::sync::Arc;

use storage::api::{run_server, SharedStore};
use storage::config::NodeConfig;
use storage::metadata::seed_metadata;
use storage::MemoryStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.trench".to_string());
    let config = NodeConfig::from_file(&config_path)?;
    let store: SharedStore = Arc::new(MemoryStore::new());

    seed_metadata(&store, &config)?;

    println!("[storage] node started: {}", config.id);

    let addr = "127.0.0.1:7878".parse()?;
    run_server(addr, store).await
}

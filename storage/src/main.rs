use std::sync::Arc;

use storage::api::run_server;
use storage::MemoryStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store: Arc<MemoryStore<String, Vec<u8>>> = Arc::new(MemoryStore::new());
    let addr = "127.0.0.1:7878".parse()?;
    run_server(addr, store).await
}

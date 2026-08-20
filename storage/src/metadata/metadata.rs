use std::error::Error;

use crate::config::NodeConfig;
use crate::SharedStore;

pub const METADATA_TABLE: &str = "__metadata";

pub fn seed_metadata(store: &SharedStore, config: &NodeConfig) -> Result<(), Box<dyn Error>> {
    let table = store.create(&METADATA_TABLE.to_string());

    table.insert("node_id".to_string(), config.id.clone().into_bytes());
    table.insert("node_address".to_string(), config.node_address.clone().into_bytes());
    table.insert("anchor_address".to_string(), config.anchor_address.clone().into_bytes());
    table.insert("region".to_string(), config.region.clone().into_bytes());
    table.insert("status".to_string(), config.status.clone().into_bytes());

    Ok(())
}

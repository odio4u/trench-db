use crate::api::SharedStore;
use crate::identity::node_identity::NodeIdentity;
use byteser::ByteSerializable;

pub fn seed_identity(store: &SharedStore, node: &NodeIdentity) -> Result<(), Box<dyn std::error::Error>> {
    let table_name = String::from("metadata");
    let key = String::from("node_identity");

    let table = store.create(&table_name);
    let mut payload = Vec::new();
    node.byte_serialize(&mut payload);

    table.insert(key, payload);
    Ok(())
}
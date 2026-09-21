
use std::format;

use uuid::Uuid;

pub fn bootstrap_signature(node_id: Uuid, fingerprint: String) -> String {
    // sign the incoming signature request to issuer
    // TODO: Implement actual bootstrap signature generation
    let signature = format!("{}:{}", node_id, fingerprint);
    signature
}


pub fn peer_signature(node_id: Uuid, fingerprint: String) -> String {

    // connect with issuer and get signed peer certificate
    // TODO: Implement actual peer signature retrieval from issuer



    let signature = format!("{}:{}", node_id, fingerprint);
    signature
}
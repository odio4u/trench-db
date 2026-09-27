
use std::{format, fs};

use uuid::Uuid;
use p256::ecdsa::{
    signature::Signer,
    Signature,
    SigningKey,
};
use p256::pkcs8::DecodePrivateKey;


pub fn bootstrap_signature(node_id: Uuid, fingerprint: String) -> Result<String, Box<dyn std::error::Error>> {
    let cert_path = "node-key.pem";
    let pem = fs::read_to_string(cert_path)?;
    // let fingerprint = 
    let signing_key = SigningKey::from_pkcs8_pem(&pem)?;

    let mut message = Vec::with_capacity(16 + 4 + fingerprint.len());
    message.extend_from_slice(node_id.as_bytes());
    message.extend_from_slice(&(fingerprint.len() as u32).to_be_bytes());
    message.extend_from_slice(fingerprint.as_bytes());


    let sig: Signature  = signing_key.sign(&message);
    let sigs = sig.to_string();
    Ok(sigs)
}


pub fn peer_signature(node_id: Uuid, fingerprint: String) -> Result<String, Box<dyn std::error::Error>> {

    // connect with issuer and get signed peer certificate
    // TODO: Implement actual peer signature retrieval from issuer



    let signature = format!("{}:{}", node_id, fingerprint);
    Ok(signature)
}
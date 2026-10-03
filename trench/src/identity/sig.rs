
use std::{fs};

use byteser::ByteSerializable;
use byteser_derive::ByteSerializable;
use uuid::Uuid;
use p256::ecdsa::{
    signature::Signer,
    Signature,
    SigningKey,
};
use p256::pkcs8::DecodePrivateKey;
use transport::client::resilient_client::ResilientClient;
use transport::server::{RequestEnvelope};

const ACTION_FINGERPRINT_SIGNATURE: &str = "fingerprint_signature";

#[derive(Debug, ByteSerializable)]
struct FingerprintSignatureRequest {
    pub node_id: Uuid,
    pub fingerprint: String,
}
#[derive(Debug, ByteSerializable)]
struct FingerprintSignatureResponse {
    pub signature: String,
}

pub fn fingerprint_signature(node_id: Uuid, fingerprint: String) -> Result<String, Box<dyn std::error::Error>> {
    let cert_path = "node-key.pem";
    let pem = fs::read_to_string(cert_path)?;
    let signing_key = SigningKey::from_pkcs8_pem(&pem)?;

    let mut message = Vec::with_capacity(16 + 4 + fingerprint.len());
    message.extend_from_slice(node_id.as_bytes());
    message.extend_from_slice(&(fingerprint.len() as u32).to_be_bytes());
    message.extend_from_slice(fingerprint.as_bytes());


    let sig: Signature  = signing_key.sign(&message);
    let sigs = sig.to_string();
    Ok(sigs)
}

pub async fn call_issuer(node_id: Uuid, fingerprint: String, issuer: super::identity::IssuerIdentity, bootstraped: bool) -> Result<String, Box<dyn std::error::Error>> {
    let (_id, _pubkey, _address, _region, _status, _issuer_bootstraped) = (
        issuer.id,
        issuer.pubkey,
        issuer.address,
        issuer.region,
        issuer.status,
        issuer.issuer_bootstraped,
    );
    if bootstraped {
        return fingerprint_signature(node_id, fingerprint);
    }

    let (host, port) = _address.rsplit_once(":").ok_or("Invalid issuer address format")?;
    let mut client = ResilientClient::new(host.to_string(), port.parse::<u16>()?);

    client.build_stream().await?;
    println!("[trench] connected issuer - {host}:{port}");

    let payload = FingerprintSignatureRequest {
        node_id,
        fingerprint,
    };
    let mut request_payload: Vec<u8> = Vec::new();
    payload.byte_serialize(&mut request_payload);

    let request = RequestEnvelope {
        action: ACTION_FINGERPRINT_SIGNATURE.to_string(),
        payload: request_payload,
    };

    let response: transport::server::ResponseEnvelope = client.send_message(&request).await?;
    if response.payload.is_empty() {
        return Err("Empty response from issuer".into());
    }

    let mut response_slice: &[u8] = &response.payload;
    let response_message: FingerprintSignatureResponse = FingerprintSignatureResponse::byte_deserialize(&mut response_slice)?;
    client.close().await?;
    println!("[trench] closed connection to issuer - {host}:{port}");
    println!("[trench] received fingerprint signature from issuer - {host}:{port}");
    Ok(response_message.signature)

}
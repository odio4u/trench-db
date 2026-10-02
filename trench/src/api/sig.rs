use async_trait::async_trait;
use std::error::Error;
use transport::errors::TransportError;
use transport::server::Handler;
use crate::api::SharedStore;
use crate::api::requests::{FingerprintRequest, FingerprintResponse};
use crate::api::{decode, encode};
use crate::identity::sig::fingerprint_signature;

pub struct FingerprintHandler{
    pub store: SharedStore,
}

#[async_trait]
impl Handler for FingerprintHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        // Implement the logic to generate the fingerprint signature here
        // For now, just return an error indicating it's not implemented
        let request: FingerprintRequest = decode(payload)?;
        let response: Result<String, Box<dyn Error>> = fingerprint_signature(request.node_id, request.fingerprint);
        match response {
            Ok(signature) => {
                let response = FingerprintResponse { signature };
                let encoded_response = encode(&response);
                Ok(encoded_response)
            }
            Err(e) => Err(TransportError::InternalError(format!("Failed to generate fingerprint signature: {}", e))),
        }
    }
}
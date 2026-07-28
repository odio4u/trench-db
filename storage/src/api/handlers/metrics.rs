//! Handler for querying storage metrics.

use async_trait::async_trait;
use transport::errors::TransportError;
use transport::server::Handler;

use crate::api::handlers::{SharedStore, decode, encode};
use crate::api::requests::{MetricsRequest, MetricsResponse};

pub struct MetricsHandler {
    pub store: SharedStore,
}

#[async_trait]
impl Handler for MetricsHandler {
    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        let _request: MetricsRequest = decode(payload)?;
        let snapshot = self.store.metrics().snapshot();
        Ok(encode(&MetricsResponse {
            reads: snapshot.reads,
            writes: snapshot.writes,
            deletes: snapshot.deletes,
            hits: snapshot.hits,
            misses: snapshot.misses,
            average_latency_ns: snapshot.average_latency_ns,
            memory_usage_bytes: snapshot.memory_usage_bytes,
        }))
    }
}

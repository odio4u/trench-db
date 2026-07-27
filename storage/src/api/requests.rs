//! Wire-facing request/response structs for each storage action.
//!
//! Kept deliberately concrete (`String` keys, raw `Vec<u8>` values) instead
//! of generic over `K`/`V`: the network protocol only ever needs one
//! encoding, and concrete types avoid pulling in generic (de)serialization
//! code paths for every instantiation, keeping the binary small.

use byteser_derive::ByteSerializable;

#[derive(Debug, ByteSerializable)]
pub struct GetRequest {
    pub table: String,
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct GetResponse {
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, ByteSerializable)]
pub struct PutRequest {
    pub table: String,
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, ByteSerializable)]
pub struct PutResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct UpdateRequest {
    pub table: String,
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, ByteSerializable)]
pub struct UpdateResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct DeleteRequest {
    pub table: String,
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct DeleteResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct ContainsRequest {
    pub table: String,
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct ContainsResponse {
    pub exists: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct AddTableRequest {
    pub table: String,
}

#[derive(Debug, ByteSerializable)]
pub struct AddTableResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct RemoveTableRequest {
    pub table: String,
}

#[derive(Debug, ByteSerializable)]
pub struct RemoveTableResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct MetricsRequest {}

#[derive(Debug, ByteSerializable)]
pub struct MetricsResponse {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub hits: u64,
    pub misses: u64,
    pub average_latency_ns: u64,
    pub memory_usage_bytes: u64,
}

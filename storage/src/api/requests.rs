//! Wire-facing request/response structs for each storage action.
//!
//! Kept deliberately concrete (`String` keys, raw `Vec<u8>` values) instead
//! of generic over `K`/`V`: the network protocol only ever needs one
//! encoding, and concrete types avoid pulling in generic (de)serialization
//! code paths for every instantiation, keeping the binary small.

use byteser_derive::ByteSerializable;

#[derive(Debug, ByteSerializable)]
pub struct GetRequest {
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct GetResponse {
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, ByteSerializable)]
pub struct PutRequest {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, ByteSerializable)]
pub struct PutResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct UpdateRequest {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, ByteSerializable)]
pub struct UpdateResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct DeleteRequest {
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct DeleteResponse {
    pub ok: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct ContainsRequest {
    pub key: String,
}

#[derive(Debug, ByteSerializable)]
pub struct ContainsResponse {
    pub exists: bool,
}

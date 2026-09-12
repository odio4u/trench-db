pub mod pipe;
pub mod error;

pub use error::WalError;
pub use pipe::WALWriter;

pub const WAL_HEADER_SIZE: usize = 8;

pub struct WalEntry {
    pub payload: Vec<u8>,
    pub key: Option<Vec<u8>>,
    pub operation: Option<u8>,
    pub checksum: Option<u32>,
}
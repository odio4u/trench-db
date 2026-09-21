use std::io::{Read, Write};

use crate::wal::error::WalError;

/// Magic bytes at the start of every WAL file.
pub const WAL_MAGIC: &[u8; 4] = b"TRDB";
/// Current WAL format version.
pub const WAL_VERSION: u8 = 1;
/// Total size of the WAL file header, in bytes.
pub const WAL_HEADER_SIZE: usize = 8;

/// Size of the length prefix for each entry payload.
pub const ENTRY_LEN_SIZE: usize = 4;
/// Size of the CRC-32 checksum stored with each entry.
pub const ENTRY_CKSUM_SIZE: usize = 4;
/// Total per-entry overhead (length + checksum).
pub const ENTRY_OVERHEAD: usize = ENTRY_LEN_SIZE + ENTRY_CKSUM_SIZE;

/// Writes a WAL file header to `writer`.
pub fn write_header<W: Write>(writer: &mut W) -> Result<(), WalError> {
    writer.write_all(WAL_MAGIC)?;
    writer.write_all(&[WAL_VERSION, 0, 0, 0])?;
    Ok(())
}

/// Reads a WAL file header from `reader` and validates its magic and version.
pub fn validate_header<R: Read>(reader: &mut R) -> Result<(), WalError> {
    use std::io;

    let mut header = [0u8; WAL_HEADER_SIZE];
    match reader.read_exact(&mut header) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(WalError::InvalidHeader {
                reason: "file too short to contain WAL header".to_string(),
            });
        }
        Err(err) => return Err(err.into()),
    }

    if &header[0..4] != WAL_MAGIC {
        return Err(WalError::InvalidHeader {
            reason: format!(
                "bad magic: expected {:?}, found {:?}",
                WAL_MAGIC,
                &header[0..4]
            ),
        });
    }

    let version = header[4];
    if version != WAL_VERSION {
        return Err(WalError::UnsupportedVersion {
            found: version,
            expected: WAL_VERSION,
        });
    }

    Ok(())
}

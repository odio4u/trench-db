use std::path::Path;

use crate::traits::Table;
use crate::wal::entry::WalEntry;
use crate::wal::error::WalError;
use crate::wal::format::WAL_HEADER_SIZE;
use crate::wal::options::{ReplayMode, ReplaySummary};
use crate::wal::reader::WalReader;

/// Replays a WAL file into `store` using the default lenient mode. Missing
/// files are treated as empty WALs.
pub fn replay(
    store: &dyn Table<String, Vec<u8>>,
    path: impl AsRef<Path>,
) -> Result<ReplaySummary, WalError> {
    replay_with_mode(store, path, ReplayMode::Lenient)
}

/// Replays a WAL file into `store` using the requested mode. Missing files are
/// treated as empty WALs.
pub fn replay_with_mode(
    store: &dyn Table<String, Vec<u8>>,
    path: impl AsRef<Path>,
    mode: ReplayMode,
) -> Result<ReplaySummary, WalError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(ReplaySummary::default());
    }

    let mut reader = WalReader::open(path)?;
    let mut summary = ReplaySummary {
        final_position: WAL_HEADER_SIZE as u64,
        ..Default::default()
    };

    loop {
        let start_position = reader.position();
        match reader.next_entry() {
            Ok(Some(entry)) => {
                apply_entry(store, entry);
                summary.entries_applied += 1;
                summary.final_position = reader.position();
            }
            Ok(None) => break,
            Err(err) => match mode {
                ReplayMode::Strict => return Err(err),
                ReplayMode::Lenient => {
                    if matches!(err, WalError::UnexpectedEof) {
                        summary.truncated_at = Some(start_position);
                    }
                    if let WalError::CorruptEntry { position, .. } = err {
                        summary.corruption_detected_at = Some(position);
                    }
                    break;
                }
            },
        }
    }

    Ok(summary)
}

fn apply_entry(store: &dyn Table<String, Vec<u8>>, entry: WalEntry) {
    match entry {
        WalEntry::CreateTable { table } => {
            store.create(&table);
        }
        WalEntry::RemoveTable { table } => {
            store.remove(&table);
        }
        WalEntry::Insert { table, key, value } => {
            store.create(&table).insert(key, value);
        }
        WalEntry::Update { table, key, value } => {
            store.create(&table).update(key, value);
        }
        WalEntry::Delete { table, key } => {
            if let Some(table_store) = store.get(&table) {
                table_store.remove(&key);
            }
        }
    }
}

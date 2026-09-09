//! Full local backup/restore using SQLite's online backup API (brief
//! section 11) — never a raw copy of the live .db file, which would miss
//! in-flight WAL state.

use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use rusqlite::{backup::Backup, Connection};
use std::path::Path;
use std::time::Duration;

pub fn backup_to(source: &Connection, dest_path: &Path) -> AnchorResult<()> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not create backup directory."))?;
    }
    let mut dest = Connection::open(dest_path)?;
    let backup = Backup::new(source, &mut dest).map_err(AnchorError::from)?;
    backup
        .run_to_completion(5, Duration::from_millis(50), None)
        .map_err(AnchorError::from)?;
    Ok(())
}

/// Validate a candidate restore file before touching the live vault:
/// openable, has the expected core tables, and is not obviously a
/// different kind of SQLite file. Extension loading stays disabled (the
/// default for a plain `Connection::open`) so an untrusted backup can't run
/// arbitrary extension code.
pub fn validate_restore_candidate(path: &Path) -> AnchorResult<()> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| AnchorError::new(SafeErrorCode::MalformedImport, "That file could not be opened as a database."))?;
    let required = ["journal_entries", "worries", "worry_outcomes", "small_steps", "app_settings"];
    for table in required {
        let ok: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !ok {
            return Err(AnchorError::new(SafeErrorCode::MalformedImport, "That file doesn't look like an Anchor vault backup."));
        }
    }
    Ok(())
}

/// Atomically switch the live vault to a validated snapshot. Caller is
/// responsible for having already backed up the current vault if the user
/// wants that safety net (the Settings screen offers this).
pub fn restore_from(dest_pool_path: &Path, validated_source: &Path) -> AnchorResult<()> {
    validate_restore_candidate(validated_source)?;
    std::fs::copy(validated_source, dest_pool_path)
        .map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not write the restored vault file."))?;
    for suffix in ["-wal", "-shm"] {
        let stale = dest_pool_path.with_extension(format!("sqlite{suffix}"));
        let _ = std::fs::remove_file(stale);
    }
    Ok(())
}

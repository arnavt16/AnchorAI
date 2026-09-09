pub mod repo;

use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub type Pool = r2d2::Pool<SqliteConnectionManager>;

/// Which vault is currently open. Demo and personal data must never mix:
/// they live in physically separate SQLite files under separate
/// directories, and switching bumps `vault_generation` so any in-flight
/// job or reflection from the other vault is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultMode {
    Personal,
    Demo,
}

pub struct VaultState {
    pub pool: Pool,
    pub mode: VaultMode,
    pub path: PathBuf,
}

/// Shared, swappable vault handle. A `Mutex<Arc<VaultState>>` rather than a
/// bare pool so that switching vault mode (or restoring a backup) can
/// atomically replace the whole handle without racing in-flight readers —
/// they hold an `Arc` snapshot for the duration of their own request.
pub struct VaultManager(pub Mutex<Arc<VaultState>>);

impl VaultManager {
    pub fn current(&self) -> Arc<VaultState> {
        self.0.lock().expect("vault mutex poisoned").clone()
    }

    pub fn replace(&self, state: VaultState) {
        let mut guard = self.0.lock().expect("vault mutex poisoned");
        *guard = Arc::new(state);
    }
}

pub fn app_data_dir(app_handle: &tauri::AppHandle) -> AnchorResult<PathBuf> {
    use tauri::Manager;
    app_handle
        .path()
        .app_data_dir()
        .map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not resolve app data directory."))
}

pub fn vault_path(app_data: &Path, mode: VaultMode) -> PathBuf {
    match mode {
        VaultMode::Personal => app_data.join("vault").join("anchor.sqlite"),
        VaultMode::Demo => app_data.join("demo-vault").join("anchor-demo.sqlite"),
    }
}

const MIGRATIONS: &[(&str, &str)] = &[("0001_init", include_str!("../../migrations/0001_init.sql"))];

pub fn open_vault(path: &Path) -> AnchorResult<VaultState> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not create vault directory."))?;
    }

    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        c.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )
    });
    let pool = r2d2::Pool::builder()
        .max_size(4)
        .build(manager)
        .map_err(|_| AnchorError::new(SafeErrorCode::DatabaseError, "Could not open the local vault."))?;

    let init_conn = pool.get()?;
    run_migrations(&init_conn)?;
    drop(init_conn);

    Ok(VaultState {
        pool,
        mode: VaultMode::Personal, // caller overwrites as needed
        path: path.to_path_buf(),
    })
}

fn run_migrations(conn: &Connection) -> AnchorResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);",
    )?;
    for (name, sql) in MIGRATIONS {
        let already: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?1)",
                [name],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if already {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO _migrations (name, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [name],
        )?;
        tracing::info!(migration = *name, "applied migration");
    }
    Ok(())
}

pub fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

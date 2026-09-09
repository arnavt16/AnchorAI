use crate::db::{repo, now_iso, VaultManager};
use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use crate::indexing::worker::IndexingControl;
use crate::models::AppSettings;
use crate::ollama::{self, OllamaClient};
use rusqlite::params;
use std::sync::atomic::Ordering;
use tauri::State;

pub const CONSENT_VERSION: &str = "2026-09-01";

#[tauri::command]
pub fn get_settings(vault: State<VaultManager>) -> AnchorResult<AppSettings> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::get_settings(&conn)
}

#[tauri::command]
pub fn grant_local_ai_consent(vault: State<VaultManager>) -> AnchorResult<AppSettings> {
    let v = vault.current();
    let conn = v.pool.get()?;
    conn.execute(
        "UPDATE app_settings SET local_ai_enabled = 1, consent_version = ?1, consent_at = ?2, consent_withdrawn_at = NULL, updated_at = ?2 WHERE id = 1",
        params![CONSENT_VERSION, now_iso()],
    )?;
    repo::get_settings(&conn)
}

/// Withdrawing consent cancels future indexing/generation and offers
/// removal of existing derived memories (the caller decides whether to
/// also purge chunks; this command only flips the flag + cancels queued
/// work, matching "withdrawal cancels future work", not an implicit purge).
#[tauri::command]
pub fn withdraw_local_ai_consent(vault: State<VaultManager>) -> AnchorResult<AppSettings> {
    let v = vault.current();
    let conn = v.pool.get()?;
    conn.execute(
        "UPDATE app_settings SET local_ai_enabled = 0, consent_withdrawn_at = ?1, updated_at = ?1 WHERE id = 1",
        params![now_iso()],
    )?;
    conn.execute("DELETE FROM indexing_jobs WHERE status IN ('pending','processing')", [])?;
    repo::get_settings(&conn)
}

#[tauri::command]
pub fn set_support_country(vault: State<VaultManager>, country: Option<String>) -> AnchorResult<AppSettings> {
    let v = vault.current();
    let conn = v.pool.get()?;
    conn.execute("UPDATE app_settings SET support_country = ?1, updated_at = ?2 WHERE id = 1", params![country, now_iso()])?;
    repo::get_settings(&conn)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub reachable: bool,
    pub local_models: Vec<String>,
}

#[tauri::command]
pub async fn check_ollama_runtime() -> AnchorResult<RuntimeStatus> {
    let client = OllamaClient::new(None).map_err(|_| AnchorError::new(SafeErrorCode::LocalOnlyUnverified, "Could not construct a loopback Ollama client."))?;
    let reachable = client.is_reachable().await;
    let local_models = if reachable {
        client.list_local_models().await.map(|m| m.into_iter().map(|x| x.name).collect()).unwrap_or_default()
    } else {
        vec![]
    };
    Ok(RuntimeStatus { reachable, local_models })
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessResult {
    pub embedding_ok: bool,
    pub embedding_dimension: Option<usize>,
    pub embedding_error: Option<String>,
    pub chat_ok: bool,
    pub chat_error: Option<String>,
}

/// Runs synthetic readiness tests: one embedding (checked for dimension +
/// finite values) and one short structured chat response — synthetic text
/// only, never journal content. On success, persists the chosen models +
/// embedding dimension into app_settings and bumps the embedding space
/// version if the embedding model changed.
#[tauri::command]
pub async fn run_readiness_check_and_select(
    vault: State<'_, VaultManager>,
    chat_model: String,
    embedding_model: String,
) -> AnchorResult<ReadinessResult> {
    let client = OllamaClient::new(None).map_err(|_| AnchorError::new(SafeErrorCode::LocalOnlyUnverified, "Could not construct a loopback Ollama client."))?;

    let mut result = ReadinessResult { embedding_ok: false, embedding_dimension: None, embedding_error: None, chat_ok: false, chat_error: None };

    match ollama::readiness_check_embedding(&client, &embedding_model).await {
        Ok(dim) => {
            result.embedding_ok = true;
            result.embedding_dimension = Some(dim);
        }
        Err(e) => result.embedding_error = Some(e.to_string()),
    }

    match ollama::readiness_check_chat(&client, &chat_model).await {
        Ok(_) => result.chat_ok = true,
        Err(e) => result.chat_error = Some(e.to_string()),
    }

    if result.embedding_ok && result.chat_ok {
        let v = vault.current();
        let conn = v.pool.get()?;
        let prev_model: Option<String> = conn.query_row("SELECT embedding_model FROM app_settings WHERE id = 1", [], |r| r.get(0)).ok();
        let models_local = client.list_local_models().await.unwrap_or_default();
        let chat_digest = models_local.iter().find(|m| m.name == chat_model).map(|m| m.digest.clone());
        let embed_digest = models_local.iter().find(|m| m.name == embedding_model).map(|m| m.digest.clone());
        let space_bump = if prev_model.as_deref() != Some(embedding_model.as_str()) { 1 } else { 0 };
        conn.execute(
            "UPDATE app_settings
             SET chat_model = ?1, chat_model_digest = ?2, embedding_model = ?3, embedding_model_digest = ?4,
                 embedding_dimension = ?5, embedding_space_version = embedding_space_version + ?6, updated_at = ?7
             WHERE id = 1",
            params![
                chat_model, chat_digest, embedding_model, embed_digest,
                result.embedding_dimension.unwrap_or(0) as i64, space_bump, now_iso()
            ],
        )?;
        if space_bump == 1 {
            // Embedding space changed: every existing chunk is now stale.
            // Re-queue all memory-eligible entries; old-space vectors are
            // never compared against new-space queries (retrieval filters
            // on embedding_space_version).
            conn.execute("DELETE FROM retrieval_chunks", [])?;
            conn.execute(
                "UPDATE journal_entries SET indexing_status = 'pending' WHERE memory_enabled = 1",
                [],
            )?;
            let ids: Vec<String> = {
                let mut stmt = conn.prepare("SELECT id FROM journal_entries WHERE memory_enabled = 1")?;
                let rows = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
                rows
            };
            for id in ids {
                let _ = crate::indexing::worker::enqueue_if_eligible(&conn, &id);
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn pause_indexing(control: State<IndexingControl>) {
    control.paused.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn resume_indexing(control: State<IndexingControl>) {
    control.paused.store(false, Ordering::Relaxed);
}

/// Manually re-enqueue every memory-eligible entry (e.g. after fixing a
/// runtime problem, or a user-triggered "rebuild index").
#[tauri::command]
pub fn rebuild_index(vault: State<VaultManager>) -> AnchorResult<usize> {
    let v = vault.current();
    let conn = v.pool.get()?;
    conn.execute("UPDATE journal_entries SET indexing_status = 'pending' WHERE memory_enabled = 1", [])?;
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM journal_entries WHERE memory_enabled = 1")?;
        let rows = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
        rows
    };
    let n = ids.len();
    for id in ids {
        let _ = crate::indexing::worker::enqueue_if_eligible(&conn, &id);
    }
    Ok(n)
}

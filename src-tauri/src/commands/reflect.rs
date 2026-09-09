use crate::db::VaultManager;
use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use crate::indexing::worker::IndexingControl;
use crate::models::{JournalEntry, ValidatedReflection};
use crate::ollama::OllamaClient;
use crate::rag::{self, ReflectRequest};
use rusqlite::params;
use std::sync::atomic::Ordering;
use tauri::State;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectInput {
    pub message: String,
    pub intention: Option<String>,
    pub use_memory: bool,
    pub exclude_entry_id: Option<String>,
}

/// Runs one reflection turn. Indexing is paused for the duration so the
/// model gets full attention while the user is actively waiting. Chat
/// history itself is never persisted here: the frontend keeps the
/// conversation in memory only and this command is stateless per call.
#[tauri::command]
pub async fn reflect(vault: State<'_, VaultManager>, control: State<'_, IndexingControl>, input: ReflectInput) -> AnchorResult<ValidatedReflection> {
    if input.message.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Message is required."));
    }
    if input.message.len() > 8000 {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Message is too long."));
    }

    control.paused.store(true, Ordering::Relaxed);
    let result = run_reflect(&vault, input).await;
    control.paused.store(false, Ordering::Relaxed);
    result
}

async fn run_reflect(vault: &State<'_, VaultManager>, input: ReflectInput) -> AnchorResult<ValidatedReflection> {
    let v = vault.current();
    let client = OllamaClient::new(None).map_err(|_| AnchorError::new(SafeErrorCode::LocalOnlyUnverified, "Could not reach the local model runtime."))?;

    let reflection = rag::reflect(
        &v.pool,
        &client,
        ReflectRequest {
            message: &input.message,
            intention: input.intention.as_deref(),
            use_memory: input.use_memory,
            exclude_entry_id: input.exclude_entry_id.as_deref(),
        },
    )
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "reflection pipeline error");
        AnchorError::new(SafeErrorCode::RuntimeStopped, "The local model runtime didn't respond. Your journal is unaffected — you can try again once it's running.")
    })?;
    Ok(reflection)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReflectionInput {
    pub text: String,
    pub tags: Vec<String>,
}

/// Explicit "Save my reflection" action: the user reviews and edits the
/// wording themselves before it becomes a journal entry. AI wording is
/// never silently turned into an autobiographical fact.
#[tauri::command]
pub fn save_reflection_as_entry(vault: State<VaultManager>, input: SaveReflectionInput) -> AnchorResult<JournalEntry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let entry = crate::db::repo::create_entry(
        &conn,
        crate::db::repo::NewEntry {
            title: Some("Saved reflection".into()),
            body: input.text,
            mood: None,
            tags: input.tags,
            memory_enabled: false,
            origin: "saved_reflection".into(),
        },
    )?;
    let _ = conn.execute("UPDATE journal_entries SET origin = 'saved_reflection' WHERE id = ?1", params![entry.id]);
    Ok(entry)
}

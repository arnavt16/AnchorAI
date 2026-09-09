use crate::db::{repo, VaultManager};
use crate::error::AnchorResult;
use crate::indexing::worker;
use crate::models::JournalEntry;
use tauri::State;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntryInput {
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    pub tags: Vec<String>,
    pub memory_enabled: bool,
}

#[tauri::command]
pub fn save_entry(vault: State<VaultManager>, input: CreateEntryInput) -> AnchorResult<JournalEntry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let entry = repo::create_entry(
        &conn,
        repo::NewEntry {
            title: input.title,
            body: input.body,
            mood: input.mood,
            tags: input.tags,
            memory_enabled: input.memory_enabled,
            origin: "user".into(),
        },
    )?;
    let _ = worker::enqueue_if_eligible(&conn, &entry.id);
    Ok(entry)
}

#[tauri::command]
pub fn get_entry(vault: State<VaultManager>, id: String) -> AnchorResult<Option<JournalEntry>> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::get_entry(&conn, &id)
}

#[tauri::command]
pub fn list_entries(vault: State<VaultManager>, search: Option<String>) -> AnchorResult<Vec<JournalEntry>> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::list_entries(&conn, search.as_deref(), 200)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntryInput {
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    pub tags: Vec<String>,
}

#[tauri::command]
pub fn update_entry(vault: State<VaultManager>, id: String, input: UpdateEntryInput) -> AnchorResult<JournalEntry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let entry = repo::update_entry(
        &conn,
        &id,
        repo::EntryEdit { title: input.title, body: input.body, mood: input.mood, tags: input.tags },
    )?;
    let _ = worker::enqueue_if_eligible(&conn, &entry.id);
    Ok(entry)
}

#[tauri::command]
pub fn delete_entry(vault: State<VaultManager>, id: String) -> AnchorResult<()> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::delete_entry(&conn, &id)
}

#[tauri::command]
pub fn set_memory_eligibility(vault: State<VaultManager>, id: String, enabled: bool) -> AnchorResult<JournalEntry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let entry = repo::set_memory_eligibility(&conn, &id, enabled)?;
    if enabled {
        let _ = worker::enqueue_if_eligible(&conn, &entry.id);
    }
    Ok(entry)
}

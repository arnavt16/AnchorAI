use crate::db::{repo, VaultManager};
use crate::error::AnchorResult;
use crate::indexing::worker;
use crate::models::{SmallStep, Worry, WorryOutcome, WorryWithHistory};
use tauri::State;

#[tauri::command]
pub fn create_worry(
    vault: State<VaultManager>,
    entry_id: String,
    worry_text: String,
    expected_outcome: Option<String>,
) -> AnchorResult<Worry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let w = repo::create_worry(&conn, &entry_id, &worry_text, expected_outcome.as_deref())?;
    let _ = worker::enqueue_if_eligible(&conn, &entry_id);
    Ok(w)
}

#[tauri::command]
pub fn update_worry(
    vault: State<VaultManager>,
    id: String,
    worry_text: String,
    expected_outcome: Option<String>,
) -> AnchorResult<Worry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let w = repo::update_worry(&conn, &id, &worry_text, expected_outcome.as_deref())?;
    let _ = worker::enqueue_if_eligible(&conn, &w.entry_id);
    Ok(w)
}

#[tauri::command]
pub fn list_worries(vault: State<VaultManager>, status: Option<String>) -> AnchorResult<Vec<WorryWithHistory>> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::list_worries(&conn, status.as_deref())
}

#[tauri::command]
pub fn set_worry_status(vault: State<VaultManager>, id: String, status: String) -> AnchorResult<Worry> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let w = repo::set_worry_status(&conn, &id, &status)?;
    let _ = worker::enqueue_if_eligible(&conn, &w.entry_id);
    Ok(w)
}

#[tauri::command]
pub fn create_outcome(
    vault: State<VaultManager>,
    worry_id: String,
    outcome_text: String,
    result_category: Option<String>,
    reflection: Option<String>,
) -> AnchorResult<WorryOutcome> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let o = repo::create_outcome(&conn, &worry_id, &outcome_text, result_category.as_deref(), reflection.as_deref())?;
    if let Some(w) = repo::get_worry(&conn, &worry_id)? {
        let _ = worker::enqueue_if_eligible(&conn, &w.entry_id);
    }
    Ok(o)
}

#[tauri::command]
pub fn update_outcome(
    vault: State<VaultManager>,
    id: String,
    outcome_text: String,
    result_category: Option<String>,
    reflection: Option<String>,
) -> AnchorResult<WorryOutcome> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let o = repo::update_outcome(&conn, &id, &outcome_text, result_category.as_deref(), reflection.as_deref())?;
    if let Some(w) = repo::get_worry(&conn, &o.worry_id)? {
        let _ = worker::enqueue_if_eligible(&conn, &w.entry_id);
    }
    Ok(o)
}

#[tauri::command]
pub fn delete_outcome(vault: State<VaultManager>, id: String) -> AnchorResult<()> {
    let v = vault.current();
    let conn = v.pool.get()?;
    repo::delete_outcome(&conn, &id)
}

#[tauri::command]
pub fn create_step(
    vault: State<VaultManager>,
    entry_id: String,
    worry_id: Option<String>,
    action_text: String,
) -> AnchorResult<SmallStep> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let s = repo::create_step(&conn, &entry_id, worry_id.as_deref(), &action_text)?;
    let _ = worker::enqueue_if_eligible(&conn, &entry_id);
    Ok(s)
}

#[tauri::command]
pub fn set_step_feedback(
    vault: State<VaultManager>,
    id: String,
    feedback: Option<String>,
    note: Option<String>,
) -> AnchorResult<SmallStep> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let s = repo::set_step_feedback(&conn, &id, feedback.as_deref(), note.as_deref())?;
    let _ = worker::enqueue_if_eligible(&conn, &s.entry_id);
    Ok(s)
}

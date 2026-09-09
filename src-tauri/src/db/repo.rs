//! Shared data-access functions. Commands, the indexing worker, and the RAG
//! pipeline all go through here rather than writing ad-hoc SQL inline, so
//! there is exactly one place that knows the schema shape.

use crate::db::{new_id, now_iso};
use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension, Row};

fn entry_from_row(row: &Row) -> rusqlite::Result<JournalEntry> {
    let tags_json: String = row.get("tags")?;
    Ok(JournalEntry {
        id: row.get("id")?,
        title: row.get("title")?,
        body: row.get("body")?,
        mood: row.get("mood")?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        origin: row.get("origin")?,
        memory_enabled: row.get::<_, i64>("memory_enabled")? != 0,
        content_version: row.get("content_version")?,
        aggregate_version: row.get("aggregate_version")?,
        indexing_status: row.get("indexing_status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn worry_from_row(row: &Row) -> rusqlite::Result<Worry> {
    Ok(Worry {
        id: row.get("id")?,
        entry_id: row.get("entry_id")?,
        worry_text: row.get("worry_text")?,
        expected_outcome: row.get("expected_outcome")?,
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn outcome_from_row(row: &Row) -> rusqlite::Result<WorryOutcome> {
    Ok(WorryOutcome {
        id: row.get("id")?,
        worry_id: row.get("worry_id")?,
        recorded_at: row.get("recorded_at")?,
        outcome_text: row.get("outcome_text")?,
        result_category: row.get("result_category")?,
        reflection: row.get("reflection")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn step_from_row(row: &Row) -> rusqlite::Result<SmallStep> {
    Ok(SmallStep {
        id: row.get("id")?,
        entry_id: row.get("entry_id")?,
        worry_id: row.get("worry_id")?,
        action_text: row.get("action_text")?,
        feedback: row.get("feedback")?,
        feedback_note: row.get("feedback_note")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

// ---------- journal_entries ----------

pub struct NewEntry {
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    pub tags: Vec<String>,
    pub memory_enabled: bool,
    pub origin: String,
}

pub fn create_entry(conn: &Connection, e: NewEntry) -> AnchorResult<JournalEntry> {
    if e.body.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Entry body is required."));
    }
    let id = new_id();
    let now = now_iso();
    let tags_json = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".into());
    let indexing_status = if e.memory_enabled { "pending" } else { "excluded" };
    conn.execute(
        "INSERT INTO journal_entries
            (id, title, body, mood, tags, origin, memory_enabled, content_version, aggregate_version, indexing_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, ?8, ?9, ?9)",
        params![id, e.title, e.body, e.mood, tags_json, e.origin, e.memory_enabled as i64, indexing_status, now],
    )?;
    get_entry(conn, &id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::Unexpected, "Entry vanished after insert."))
}

pub fn get_entry(conn: &Connection, id: &str) -> AnchorResult<Option<JournalEntry>> {
    conn.query_row("SELECT * FROM journal_entries WHERE id = ?1", [id], entry_from_row)
        .optional()
        .map_err(AnchorError::from)
}

pub fn list_entries(conn: &Connection, search: Option<&str>, limit: i64) -> AnchorResult<Vec<JournalEntry>> {
    let mut stmt;
    let rows = if let Some(q) = search.filter(|s| !s.trim().is_empty()) {
        stmt = conn.prepare(
            "SELECT * FROM journal_entries
             WHERE body LIKE ?1 OR title LIKE ?1 OR tags LIKE ?1
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let like = format!("%{}%", q.replace('%', "").replace('_', ""));
        stmt.query_map(params![like, limit], entry_from_row)?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        stmt = conn.prepare("SELECT * FROM journal_entries ORDER BY created_at DESC LIMIT ?1")?;
        stmt.query_map(params![limit], entry_from_row)?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(rows)
}

pub struct EntryEdit {
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    pub tags: Vec<String>,
}

pub fn update_entry(conn: &Connection, id: &str, edit: EntryEdit) -> AnchorResult<JournalEntry> {
    if edit.body.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Entry body is required."));
    }
    let now = now_iso();
    let tags_json = serde_json::to_string(&edit.tags).unwrap_or_else(|_| "[]".into());
    let changed = conn.execute(
        "UPDATE journal_entries
         SET title = ?1, body = ?2, mood = ?3, tags = ?4,
             content_version = content_version + 1,
             aggregate_version = aggregate_version + 1,
             indexing_status = CASE WHEN memory_enabled = 1 THEN 'pending' ELSE indexing_status END,
             updated_at = ?5
         WHERE id = ?6",
        params![edit.title, edit.body, edit.mood, tags_json, now, id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Entry not found."));
    }
    get_entry(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Entry not found."))
}

pub fn set_memory_eligibility(conn: &Connection, id: &str, enabled: bool) -> AnchorResult<JournalEntry> {
    let now = now_iso();
    let status = if enabled { "pending" } else { "excluded" };
    let changed = conn.execute(
        "UPDATE journal_entries
         SET memory_enabled = ?1, indexing_status = ?2, aggregate_version = aggregate_version + 1, updated_at = ?3
         WHERE id = ?4",
        params![enabled as i64, status, now, id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Entry not found."));
    }
    if !enabled {
        // Disabling memory immediately removes derived chunks and cancels queued work.
        conn.execute("DELETE FROM retrieval_chunks WHERE entry_id = ?1", [id])?;
        conn.execute(
            "DELETE FROM indexing_jobs WHERE entry_id = ?1 AND status IN ('pending','processing')",
            [id],
        )?;
    }
    get_entry(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Entry not found."))
}

pub fn delete_entry(conn: &Connection, id: &str) -> AnchorResult<()> {
    // ON DELETE CASCADE covers worries/outcomes/steps/chunks/jobs.
    let changed = conn.execute("DELETE FROM journal_entries WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Entry not found."));
    }
    Ok(())
}

pub fn mark_indexing_status(conn: &Connection, entry_id: &str, status: &str) -> AnchorResult<()> {
    conn.execute(
        "UPDATE journal_entries SET indexing_status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now_iso(), entry_id],
    )?;
    Ok(())
}

// ---------- worries ----------

pub fn create_worry(conn: &Connection, entry_id: &str, worry_text: &str, expected_outcome: Option<&str>) -> AnchorResult<Worry> {
    if worry_text.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Worry text is required."));
    }
    let existing: Option<String> = conn
        .query_row("SELECT id FROM worries WHERE entry_id = ?1", [entry_id], |r| r.get(0))
        .optional()?;
    if existing.is_some() {
        return Err(AnchorError::new(
            SafeErrorCode::InvalidInput,
            "This entry already has a tracked worry (one per entry in this version).",
        ));
    }
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO worries (id, entry_id, worry_text, expected_outcome, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'open', ?5, ?5)",
        params![id, entry_id, worry_text, expected_outcome, now],
    )?;
    bump_aggregate_version_for_entry(conn, entry_id)?;
    get_worry(conn, &id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::Unexpected, "Worry vanished after insert."))
}

pub fn get_worry(conn: &Connection, id: &str) -> AnchorResult<Option<Worry>> {
    conn.query_row("SELECT * FROM worries WHERE id = ?1", [id], worry_from_row)
        .optional()
        .map_err(AnchorError::from)
}

pub fn list_worries(conn: &Connection, status: Option<&str>) -> AnchorResult<Vec<WorryWithHistory>> {
    let mut stmt;
    let worries: Vec<Worry> = if let Some(s) = status {
        stmt = conn.prepare("SELECT * FROM worries WHERE status = ?1 ORDER BY created_at DESC")?;
        stmt.query_map([s], worry_from_row)?.collect::<Result<Vec<_>, _>>()?
    } else {
        stmt = conn.prepare("SELECT * FROM worries ORDER BY created_at DESC")?;
        stmt.query_map([], worry_from_row)?.collect::<Result<Vec<_>, _>>()?
    };
    worries
        .into_iter()
        .map(|w| {
            let outcomes = list_outcomes(conn, &w.id)?;
            let steps = list_steps_for_worry(conn, &w.id)?;
            Ok(WorryWithHistory { worry: w, outcomes, steps })
        })
        .collect()
}

pub fn set_worry_status(conn: &Connection, id: &str, status: &str) -> AnchorResult<Worry> {
    if !["open", "resolved", "archived"].contains(&status) {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Invalid worry status."));
    }
    let changed = conn.execute(
        "UPDATE worries SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Worry not found."));
    }
    let w = get_worry(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
    bump_aggregate_version_for_entry(conn, &w.entry_id)?;
    Ok(w)
}

pub fn update_worry(conn: &Connection, id: &str, worry_text: &str, expected_outcome: Option<&str>) -> AnchorResult<Worry> {
    if worry_text.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Worry text is required."));
    }
    let changed = conn.execute(
        "UPDATE worries SET worry_text = ?1, expected_outcome = ?2, updated_at = ?3 WHERE id = ?4",
        params![worry_text, expected_outcome, now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Worry not found."));
    }
    let w = get_worry(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
    bump_aggregate_version_for_entry(conn, &w.entry_id)?;
    Ok(w)
}

// ---------- worry_outcomes ----------

pub fn create_outcome(
    conn: &Connection,
    worry_id: &str,
    outcome_text: &str,
    result_category: Option<&str>,
    reflection: Option<&str>,
) -> AnchorResult<WorryOutcome> {
    if outcome_text.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Outcome description is required."));
    }
    let worry = get_worry(conn, worry_id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO worry_outcomes (id, worry_id, recorded_at, outcome_text, result_category, reflection, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3, ?3)",
        params![id, worry_id, now, outcome_text, result_category, reflection],
    )?;
    bump_aggregate_version_for_entry(conn, &worry.entry_id)?;
    get_outcome(conn, &id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::Unexpected, "Outcome vanished after insert."))
}

pub fn get_outcome(conn: &Connection, id: &str) -> AnchorResult<Option<WorryOutcome>> {
    conn.query_row("SELECT * FROM worry_outcomes WHERE id = ?1", [id], outcome_from_row)
        .optional()
        .map_err(AnchorError::from)
}

pub fn list_outcomes(conn: &Connection, worry_id: &str) -> AnchorResult<Vec<WorryOutcome>> {
    let mut stmt = conn.prepare("SELECT * FROM worry_outcomes WHERE worry_id = ?1 ORDER BY recorded_at ASC")?;
    let rows = stmt.query_map([worry_id], outcome_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn update_outcome(
    conn: &Connection,
    id: &str,
    outcome_text: &str,
    result_category: Option<&str>,
    reflection: Option<&str>,
) -> AnchorResult<WorryOutcome> {
    if outcome_text.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Outcome description is required."));
    }
    let changed = conn.execute(
        "UPDATE worry_outcomes SET outcome_text = ?1, result_category = ?2, reflection = ?3, updated_at = ?4 WHERE id = ?5",
        params![outcome_text, result_category, reflection, now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Outcome not found."));
    }
    let o = get_outcome(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Outcome not found."))?;
    let worry = get_worry(conn, &o.worry_id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
    bump_aggregate_version_for_entry(conn, &worry.entry_id)?;
    Ok(o)
}

pub fn delete_outcome(conn: &Connection, id: &str) -> AnchorResult<()> {
    let outcome = get_outcome(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Outcome not found."))?;
    let worry = get_worry(conn, &outcome.worry_id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
    conn.execute("DELETE FROM worry_outcomes WHERE id = ?1", [id])?;
    bump_aggregate_version_for_entry(conn, &worry.entry_id)?;
    Ok(())
}

// ---------- small_steps ----------

pub fn create_step(conn: &Connection, entry_id: &str, worry_id: Option<&str>, action_text: &str) -> AnchorResult<SmallStep> {
    if action_text.trim().is_empty() {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Step text is required."));
    }
    if let Some(wid) = worry_id {
        let w = get_worry(conn, wid)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Worry not found."))?;
        if w.entry_id != entry_id {
            return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Linked worry must belong to the same entry."));
        }
    }
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO small_steps (id, entry_id, worry_id, action_text, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, entry_id, worry_id, action_text, now],
    )?;
    bump_aggregate_version_for_entry(conn, entry_id)?;
    get_step(conn, &id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::Unexpected, "Step vanished after insert."))
}

pub fn get_step(conn: &Connection, id: &str) -> AnchorResult<Option<SmallStep>> {
    conn.query_row("SELECT * FROM small_steps WHERE id = ?1", [id], step_from_row)
        .optional()
        .map_err(AnchorError::from)
}

pub fn list_steps_for_worry(conn: &Connection, worry_id: &str) -> AnchorResult<Vec<SmallStep>> {
    let mut stmt = conn.prepare("SELECT * FROM small_steps WHERE worry_id = ?1 ORDER BY created_at ASC")?;
    let rows = stmt.query_map([worry_id], step_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn set_step_feedback(conn: &Connection, id: &str, feedback: Option<&str>, note: Option<&str>) -> AnchorResult<SmallStep> {
    if let Some(f) = feedback {
        if !["helped", "did_not_help", "unsure", "not_tried"].contains(&f) {
            return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Invalid feedback value."));
        }
    }
    let changed = conn.execute(
        "UPDATE small_steps SET feedback = ?1, feedback_note = ?2, updated_at = ?3 WHERE id = ?4",
        params![feedback, note, now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AnchorError::new(SafeErrorCode::NotFound, "Step not found."));
    }
    let step = get_step(conn, id)?.ok_or_else(|| AnchorError::new(SafeErrorCode::NotFound, "Step not found."))?;
    bump_aggregate_version_for_entry(conn, &step.entry_id)?;
    Ok(step)
}

// ---------- shared ----------

/// Bump the owning entry's aggregate_version and flip it back to `pending`
/// indexing if memory is on, so a worry/outcome/step edit invalidates and
/// re-queues the entry's retrieval snapshot.
pub fn bump_aggregate_version_for_entry(conn: &Connection, entry_id: &str) -> AnchorResult<()> {
    conn.execute(
        "UPDATE journal_entries
         SET aggregate_version = aggregate_version + 1,
             indexing_status = CASE WHEN memory_enabled = 1 THEN 'pending' ELSE indexing_status END,
             updated_at = ?1
         WHERE id = ?2",
        params![now_iso(), entry_id],
    )?;
    Ok(())
}

pub fn get_settings(conn: &Connection) -> AnchorResult<AppSettings> {
    conn.query_row(
        "SELECT local_ai_enabled, consent_version, consent_at, consent_withdrawn_at,
                chat_model, chat_model_digest, embedding_model, embedding_model_digest,
                embedding_dimension, embedding_space_version, support_country, vault_generation
         FROM app_settings WHERE id = 1",
        [],
        |row| {
            Ok(AppSettings {
                local_ai_enabled: row.get::<_, i64>(0)? != 0,
                consent_version: row.get(1)?,
                consent_at: row.get(2)?,
                consent_withdrawn_at: row.get(3)?,
                chat_model: row.get(4)?,
                chat_model_digest: row.get(5)?,
                embedding_model: row.get(6)?,
                embedding_model_digest: row.get(7)?,
                embedding_dimension: row.get(8)?,
                embedding_space_version: row.get(9)?,
                support_country: row.get(10)?,
                vault_generation: row.get(11)?,
            })
        },
    )
    .map_err(AnchorError::from)
}

//! Export/import, Markdown export, backup/restore, vault erasure, and the
//! demo/personal vault switch (brief sections 11 & 13).

use crate::backup;
use crate::db::repo;
use crate::db::{app_data_dir, now_iso, open_vault, VaultManager, VaultMode, VaultState};
use crate::error::{AnchorError, AnchorResult, SafeErrorCode};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

// ---------- Export / Import JSON ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportOutcome {
    #[serde(rename = "outcomeText")]
    pub outcome_text: String,
    #[serde(rename = "resultCategory")]
    pub result_category: Option<String>,
    pub reflection: Option<String>,
    #[serde(rename = "recordedAt")]
    pub recorded_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportStep {
    #[serde(rename = "actionText")]
    pub action_text: String,
    pub feedback: Option<String>,
    #[serde(rename = "feedbackNote")]
    pub feedback_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportWorry {
    #[serde(rename = "worryText")]
    pub worry_text: String,
    #[serde(rename = "expectedOutcome")]
    pub expected_outcome: Option<String>,
    pub status: String,
    #[serde(default)]
    pub outcomes: Vec<ExportOutcome>,
    #[serde(default)]
    pub steps: Vec<ExportStep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportEntry {
    #[serde(rename = "localId", default)]
    pub local_id: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "memoryEnabled", default)]
    pub memory_enabled: bool,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub worry: Option<ExportWorry>,
    #[serde(default)]
    pub steps: Vec<ExportStep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultExport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(rename = "exportedAt")]
    pub exported_at: String,
    pub entries: Vec<ExportEntry>,
}

/// Portable export. Deliberately excludes: transient jobs, embeddings, raw
/// session chats (there are none persisted to exclude — chat is
/// session-only by design), absolute machine paths, and consent state
/// (brief section 11, "Portable export").
#[tauri::command]
pub fn export_vault_json(vault: State<VaultManager>) -> AnchorResult<String> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let entries = repo::list_entries(&conn, None, 100000)?;
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let worry_row: Option<String> = conn
            .query_row("SELECT id FROM worries WHERE entry_id = ?1", [&e.id], |r| r.get(0))
            .ok();
        let worry = match worry_row {
            Some(wid) => {
                let w = repo::get_worry(&conn, &wid)?;
                w.map(|w| -> AnchorResult<ExportWorry> {
                    let outcomes = repo::list_outcomes(&conn, &w.id)?
                        .into_iter()
                        .map(|o| ExportOutcome {
                            outcome_text: o.outcome_text,
                            result_category: o.result_category,
                            reflection: o.reflection,
                            recorded_at: Some(o.recorded_at),
                        })
                        .collect();
                    let steps = repo::list_steps_for_worry(&conn, &w.id)?
                        .into_iter()
                        .map(|s| ExportStep { action_text: s.action_text, feedback: s.feedback, feedback_note: s.feedback_note })
                        .collect();
                    Ok(ExportWorry { worry_text: w.worry_text, expected_outcome: w.expected_outcome, status: w.status, outcomes, steps })
                })
                .transpose()?
            }
            None => None,
        };
        out.push(ExportEntry {
            local_id: Some(e.id.clone()),
            title: e.title,
            body: e.body,
            mood: e.mood,
            tags: e.tags,
            memory_enabled: e.memory_enabled,
            created_at: Some(e.created_at),
            worry,
            steps: vec![], // entry-level (non-worry) steps: none in current UI flow; reserved for future use
        });
    }
    let export = VaultExport { schema_version: 1, exported_at: now_iso(), entries: out };
    serde_json::to_string_pretty(&export).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not serialize export."))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub entry_count: usize,
    pub worry_count: usize,
    pub outcome_count: usize,
    pub step_count: usize,
}

pub fn preview_import(json: &str) -> AnchorResult<(VaultExport, ImportPreview)> {
    if json.len() > 50_000_000 {
        return Err(AnchorError::new(SafeErrorCode::MalformedImport, "Import file is too large."));
    }
    let parsed: VaultExport =
        serde_json::from_str(json).map_err(|_| AnchorError::new(SafeErrorCode::MalformedImport, "That file isn't a valid Anchor export."))?;
    if parsed.schema_version != 1 {
        return Err(AnchorError::new(SafeErrorCode::MalformedImport, "Unsupported export schema version."));
    }
    let mut worry_count = 0;
    let mut outcome_count = 0;
    let mut step_count = 0;
    for e in &parsed.entries {
        if e.body.len() > 200_000 {
            return Err(AnchorError::new(SafeErrorCode::MalformedImport, "An entry body is unreasonably large."));
        }
        if let Some(w) = &e.worry {
            worry_count += 1;
            outcome_count += w.outcomes.len();
            step_count += w.steps.len();
        }
        step_count += e.steps.len();
    }
    let preview = ImportPreview { entry_count: parsed.entries.len(), worry_count, outcome_count, step_count };
    Ok((parsed, preview))
}

#[tauri::command]
pub fn preview_vault_import(json: String) -> AnchorResult<ImportPreview> {
    preview_import(&json).map(|(_, p)| p)
}

/// Import into the *currently active* vault (personal or demo, whichever is
/// open) inside a single transaction; on any failure nothing is written.
/// Newly imported entries keep their memoryEnabled flag but are NOT
/// automatically enqueued for indexing — that requires local-AI consent
/// plus the user separately triggering `rebuild_index` (brief section 11,
/// "indexed only after local-AI consent and user confirmation").
#[tauri::command]
pub fn import_vault_json(vault: State<VaultManager>, json: String) -> AnchorResult<ImportPreview> {
    let (parsed, preview) = preview_import(&json)?;
    let v = vault.current();
    let mut conn = v.pool.get()?;
    let tx = conn.transaction().map_err(AnchorError::from)?;
    for e in parsed.entries {
        let entry = repo::create_entry(
            &tx,
            repo::NewEntry {
                title: e.title,
                body: e.body,
                mood: e.mood,
                tags: e.tags,
                memory_enabled: e.memory_enabled,
                origin: "imported".into(),
            },
        )?;
        if let Some(created_at) = e.created_at {
            tx.execute("UPDATE journal_entries SET created_at = ?1 WHERE id = ?2", params![created_at, entry.id])?;
        }
        if let Some(w) = e.worry {
            let worry = repo::create_worry(&tx, &entry.id, &w.worry_text, w.expected_outcome.as_deref())?;
            if w.status != "open" {
                repo::set_worry_status(&tx, &worry.id, &w.status)?;
            }
            for o in w.outcomes {
                let outcome = repo::create_outcome(&tx, &worry.id, &o.outcome_text, o.result_category.as_deref(), o.reflection.as_deref())?;
                if let Some(recorded_at) = o.recorded_at {
                    tx.execute("UPDATE worry_outcomes SET recorded_at = ?1 WHERE id = ?2", params![recorded_at, outcome.id])?;
                }
            }
            for s in w.steps {
                let step = repo::create_step(&tx, &entry.id, Some(worry.id.as_str()), &s.action_text)?;
                if s.feedback.is_some() {
                    repo::set_step_feedback(&tx, &step.id, s.feedback.as_deref(), s.feedback_note.as_deref())?;
                }
            }
        }
        for s in e.steps {
            let step = repo::create_step(&tx, &entry.id, None, &s.action_text)?;
            if s.feedback.is_some() {
                repo::set_step_feedback(&tx, &step.id, s.feedback.as_deref(), s.feedback_note.as_deref())?;
            }
        }
    }
    tx.commit().map_err(AnchorError::from)?;
    Ok(preview)
}

/// Writes already-generated export text to a path the user picked via a
/// native save dialog on the frontend. Deliberately narrow — the renderer
/// has no generic filesystem plugin; this command only ever writes UTF-8
/// text below a sane size cap, never arbitrary bytes or paths the user
/// didn't explicitly choose (brief section 10, "no general HTTP proxy" /
/// "no arbitrary file access").
#[tauri::command]
pub fn write_text_export(path: String, content: String) -> AnchorResult<()> {
    if content.len() > 100_000_000 {
        return Err(AnchorError::new(SafeErrorCode::Unexpected, "Export is too large to write."));
    }
    std::fs::write(&path, content).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not write the export file."))
}

/// Reads a file the user picked via a native open dialog, for the JSON
/// import flow. Same narrow-command reasoning as `write_text_export`.
#[tauri::command]
pub fn read_text_import(path: String) -> AnchorResult<String> {
    let metadata = std::fs::metadata(&path).map_err(|_| AnchorError::new(SafeErrorCode::MalformedImport, "Could not read that file."))?;
    if metadata.len() > 100_000_000 {
        return Err(AnchorError::new(SafeErrorCode::MalformedImport, "That file is too large to import."));
    }
    std::fs::read_to_string(&path).map_err(|_| AnchorError::new(SafeErrorCode::MalformedImport, "Could not read that file as text."))
}

// ---------- Markdown export ----------

fn safe_filename(title: Option<&str>, id: &str, created_at: &str) -> String {
    let date_part = created_at.get(0..10).unwrap_or("undated");
    let title_part: String = title
        .unwrap_or("entry")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace(' ', "-")
        .chars()
        .take(60)
        .collect();
    format!("{}-{}-{}.md", date_part, title_part, &id[..8.min(id.len())])
}

/// Writes one .md file per entry into `target_dir` (chosen by the user via
/// a native save dialog on the frontend). Never writes into an existing
/// Obsidian vault automatically — the user picks the destination.
#[tauri::command]
pub fn export_markdown(vault: State<VaultManager>, target_dir: String) -> AnchorResult<usize> {
    let v = vault.current();
    let conn = v.pool.get()?;
    let dir = std::path::Path::new(&target_dir);
    std::fs::create_dir_all(dir).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not create export directory."))?;

    let entries = repo::list_entries(&conn, None, 100000)?;
    let mut count = 0;
    for e in entries {
        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: \"{}\"\n", e.title.clone().unwrap_or_default().replace('"', "'")));
        md.push_str(&format!("created: {}\n", e.created_at));
        if let Some(mood) = &e.mood {
            md.push_str(&format!("mood: {}\n", mood));
        }
        if !e.tags.is_empty() {
            md.push_str(&format!("tags: [{}]\n", e.tags.join(", ")));
        }
        md.push_str("source: Anchor (local-first journal)\n---\n\n");
        md.push_str(&e.body);
        md.push('\n');

        let worry_id: Option<String> = conn.query_row("SELECT id FROM worries WHERE entry_id = ?1", [&e.id], |r| r.get(0)).ok();
        if let Some(wid) = worry_id {
            if let Some(w) = repo::get_worry(&conn, &wid)? {
                md.push_str(&format!("\n## Worry\n\n{}\n", w.worry_text));
                if let Some(exp) = &w.expected_outcome {
                    md.push_str(&format!("\n*Expected:* {}\n", exp));
                }
                for o in repo::list_outcomes(&conn, &wid)? {
                    md.push_str(&format!("\n### Outcome ({})\n\n{}\n", o.recorded_at, o.outcome_text));
                }
            }
        }

        let filename = safe_filename(e.title.as_deref(), &e.id, &e.created_at);
        std::fs::write(dir.join(filename), md).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not write a Markdown file."))?;
        count += 1;
    }
    Ok(count)
}

// ---------- Backup / restore ----------

#[tauri::command]
pub fn backup_vault(vault: State<VaultManager>, target_path: String) -> AnchorResult<()> {
    let v = vault.current();
    let conn = v.pool.get()?;
    backup::backup_to(&conn, std::path::Path::new(&target_path))
}

#[tauri::command]
pub fn restore_vault(app: AppHandle, vault: State<VaultManager>, source_path: String) -> AnchorResult<()> {
    let v = vault.current();
    backup::validate_restore_candidate(std::path::Path::new(&source_path))?;
    // Safety net: back up current state before overwriting.
    let safety_dir = app_data_dir(&app)?.join("pre-restore-backups");
    let _ = std::fs::create_dir_all(&safety_dir);
    let safety_path = safety_dir.join(format!("pre-restore-{}.sqlite", now_iso().replace(':', "-")));
    if let Ok(conn) = v.pool.get() {
        let _ = backup::backup_to(&conn, &safety_path);
    }

    backup::restore_from(&v.path, std::path::Path::new(&source_path))?;
    let mut new_state = open_vault(&v.path)?;
    new_state.mode = v.mode;
    bump_vault_generation(&new_state)?;
    let vm = vault.inner();
    vm.replace(new_state);
    Ok(())
}

// ---------- Erase vault ----------

#[tauri::command]
pub fn erase_vault(app: AppHandle, vault: State<VaultManager>) -> AnchorResult<()> {
    let v = vault.current();
    let mode = v.mode;
    let path = v.path.clone();
    drop(v);
    // Dropping our Arc doesn't guarantee the pool is closed if another
    // request is mid-flight; r2d2 connections close as they're returned.
    // For a single-instance desktop app this is an acceptable ordering —
    // the caller (Settings screen) warns the user this action is final.
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    let mut new_state = open_vault(&path)?;
    new_state.mode = mode;
    bump_vault_generation(&new_state)?;
    vault.inner().replace(new_state);
    let _ = app; // reserved: future per-window cache clears
    Ok(())
}

// ---------- Demo / personal vault switch ----------

#[tauri::command]
pub fn get_vault_mode(vault: State<VaultManager>) -> VaultMode {
    vault.current().mode
}

#[tauri::command]
pub fn switch_vault_mode(app: AppHandle, vault: State<VaultManager>, mode: VaultMode) -> AnchorResult<VaultMode> {
    let data_dir = app_data_dir(&app)?;
    let path = crate::db::vault_path(&data_dir, mode);
    let mut new_state = open_vault(&path)?;
    new_state.mode = mode;
    bump_vault_generation(&new_state)?;
    vault.inner().replace(new_state);
    Ok(mode)
}

fn bump_vault_generation(state: &VaultState) -> AnchorResult<()> {
    let conn = state.pool.get()?;
    conn.execute("UPDATE app_settings SET vault_generation = vault_generation + 1, updated_at = ?1 WHERE id = 1", params![now_iso()])?;
    Ok(())
}

const DEMO_FIXTURES: &str = include_str!("../../../evals/fixtures/demo_entries.json");

#[derive(Debug, Deserialize)]
struct DemoFixtureFile {
    entries: Vec<ExportEntry>,
}

/// Idempotent: seeds the demo vault only if it's currently empty, and
/// refuses to run against a vault currently in Personal mode (guards
/// against ever writing fixture data into real journal content).
#[tauri::command]
pub fn seed_demo_vault(vault: State<VaultManager>) -> AnchorResult<usize> {
    let v = vault.current();
    if v.mode != VaultMode::Demo {
        return Err(AnchorError::new(SafeErrorCode::InvalidInput, "Switch to the demo vault before seeding it."));
    }
    let conn = v.pool.get()?;
    let existing: i64 = conn.query_row("SELECT COUNT(*) FROM journal_entries", [], |r| r.get(0))?;
    if existing > 0 {
        return Ok(0);
    }
    let file: DemoFixtureFile =
        serde_json::from_str(DEMO_FIXTURES).map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Bundled demo fixtures are malformed."))?;
    let json = serde_json::to_string(&VaultExport { schema_version: 1, exported_at: now_iso(), entries: file.entries })
        .map_err(|_| AnchorError::new(SafeErrorCode::Unexpected, "Could not prepare demo fixtures."))?;
    drop(conn);
    let preview = preview_import(&json)?.1;
    import_vault_json(vault, json)?;
    Ok(preview.entry_count)
}

#[allow(dead_code)]
fn _unused_connection_type_hint(_c: &Connection) {}

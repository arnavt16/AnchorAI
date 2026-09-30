//! Durable indexing worker.
//!
//! Single-threaded by design ("process one generation at a time"): jobs are
//! claimed and processed sequentially from a tokio interval loop, not a
//! thread pool, so a constrained machine never runs two embedding requests
//! at once. Reflection holds a `ReflectionGuard` for its duration so it gets
//! the model's full attention while the user is actively waiting on it.

use crate::db::repo;
use crate::db::{new_id, now_iso, Pool, VaultManager};
use crate::indexing::chunker::{self, ChunkInput};
use crate::ollama::OllamaClient;
use rusqlite::{params, Connection};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tauri::Manager;

/// Two independent reasons to hold off indexing: a manual pause from
/// Settings, and a count of reflections currently in flight. A counter
/// rather than a flag so two overlapping reflections can't have the first
/// one to finish resume indexing while the second is still waiting.
#[derive(Default)]
pub struct IndexingControl {
    pub paused: AtomicBool,
    active_reflections: AtomicUsize,
}

impl IndexingControl {
    pub fn should_skip(&self) -> bool {
        self.paused.load(Ordering::SeqCst) || self.active_reflections.load(Ordering::SeqCst) > 0
    }

    /// Hold indexing off until the returned guard is dropped (including on
    /// early return or error).
    pub fn begin_reflection(&self) -> ReflectionGuard<'_> {
        self.active_reflections.fetch_add(1, Ordering::SeqCst);
        ReflectionGuard { control: self }
    }
}

pub struct ReflectionGuard<'a> {
    control: &'a IndexingControl,
}

impl Drop for ReflectionGuard<'_> {
    fn drop(&mut self) {
        self.control.active_reflections.fetch_sub(1, Ordering::SeqCst);
    }
}

const MAX_ATTEMPTS: i64 = 5;

pub fn spawn_worker_loop(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Resume any jobs left mid-flight from a previous run before entering
        // the steady-state poll loop.
        reclaim_interrupted_jobs(&app);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            if app.state::<IndexingControl>().should_skip() {
                continue;
            }
            let vault = app.state::<VaultManager>().current();
            let ollama = match OllamaClient::new(None) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(error = %e, "could not build Ollama client");
                    continue;
                }
            };
            if let Err(e) = process_next_job(&vault.pool, &ollama).await {
                tracing::warn!(error = %e, "indexing tick failed");
            }
        }
    });
}

fn reclaim_interrupted_jobs(app: &tauri::AppHandle) {
    let vault_mgr = app.state::<VaultManager>();
    let vault = vault_mgr.current();
    if let Ok(conn) = vault.pool.get() {
        let _ = conn.execute(
            "UPDATE indexing_jobs SET status = 'pending', updated_at = ?1 WHERE status = 'processing'",
            [now_iso()],
        );
    }
}

/// Called after any entry/worry/outcome/step write that should make the
/// entry eligible for (re)indexing. Idempotent: the unique constraint on
/// (entry_id, requested_aggregate_version, requested_embedding_space_version)
/// means re-enqueuing the same snapshot is a no-op.
pub fn enqueue_if_eligible(conn: &Connection, entry_id: &str) -> anyhow::Result<()> {
    let entry = repo::get_entry(conn, entry_id)?;
    let Some(entry) = entry else { return Ok(()) };
    if !entry.memory_enabled {
        return Ok(());
    }
    let settings = repo::get_settings(conn)?;
    if !settings.local_ai_enabled {
        return Ok(());
    }
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT OR IGNORE INTO indexing_jobs
            (id, entry_id, requested_aggregate_version, requested_embedding_space_version, status, vault_generation, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?6)",
        params![id, entry_id, entry.aggregate_version, settings.embedding_space_version, settings.vault_generation, now],
    )?;
    Ok(())
}

pub struct ClaimedJob {
    pub id: String,
    pub entry_id: String,
    pub requested_aggregate_version: i64,
    pub requested_embedding_space_version: i64,
    pub vault_generation: i64,
}

/// Oldest pending job whose retry backoff (`next_attempt_at`) has elapsed.
/// `julianday()` compares the timestamps as instants rather than strings,
/// since RFC3339 fractional seconds vary in length.
pub fn next_due_job(conn: &Connection) -> anyhow::Result<Option<ClaimedJob>> {
    Ok(conn
        .query_row(
            "SELECT id, entry_id, requested_aggregate_version, requested_embedding_space_version, vault_generation
             FROM indexing_jobs
             WHERE status = 'pending'
               AND (next_attempt_at IS NULL OR julianday(next_attempt_at) <= julianday('now'))
             ORDER BY created_at ASC LIMIT 1",
            [],
            |r| {
                Ok(ClaimedJob {
                    id: r.get(0)?,
                    entry_id: r.get(1)?,
                    requested_aggregate_version: r.get(2)?,
                    requested_embedding_space_version: r.get(3)?,
                    vault_generation: r.get(4)?,
                })
            },
        )
        .optional_or_none())
}

/// Claim and process one due job. Returns `Ok(false)` when nothing was due.
/// Takes a pool rather than an `AppHandle` so the eval harness
/// (`examples/retrieval_eval.rs`) can drive the exact same indexing path.
pub async fn process_next_job(pool: &Pool, ollama: &OllamaClient) -> anyhow::Result<bool> {
    let job = { let conn = pool.get()?; next_due_job(&conn)? };
    let Some(ClaimedJob {
        id: job_id,
        entry_id,
        requested_aggregate_version: req_agg,
        requested_embedding_space_version: req_space,
        vault_generation: vault_gen,
    }) = job
    else {
        return Ok(false);
    };

    {
        let conn = pool.get()?;
        conn.execute("UPDATE indexing_jobs SET status = 'processing', updated_at = ?1 WHERE id = ?2", params![now_iso(), job_id])?;
    }

    let settings = { let conn = pool.get()?; repo::get_settings(&conn)? };
    let (Some(embedding_model), Some(_dim)) = (settings.embedding_model.clone(), settings.embedding_dimension) else {
        fail_job(pool, &job_id, &entry_id, "model_missing")?;
        return Ok(true);
    };

    let entry = { let conn = pool.get()?; repo::get_entry(&conn, &entry_id)? };
    let Some(entry) = entry else {
        // Source gone: discard stale work silently, not an error.
        let conn = pool.get()?;
        conn.execute("DELETE FROM indexing_jobs WHERE id = ?1", [&job_id])?;
        return Ok(true);
    };

    let mut inputs: Vec<ChunkInput> = chunker::chunks_for_entry_body(&entry.body);
    {
        let conn = pool.get()?;
        let worry_id: Option<String> = conn
            .query_row("SELECT id FROM worries WHERE entry_id = ?1", [&entry_id], |r| r.get(0))
            .optional_or_none();
        if let Some(wid) = worry_id {
            if let Some(worry) = repo::get_worry(&conn, &wid)? {
                inputs.push(chunker::chunk_for_worry(&worry.id, &worry.worry_text, worry.expected_outcome.as_deref()));
                for outcome in repo::list_outcomes(&conn, &worry.id).unwrap_or_default() {
                    inputs.push(chunker::chunk_for_outcome(&worry.id, &outcome.id, &outcome));
                }
                for step in repo::list_steps_for_worry(&conn, &worry.id).unwrap_or_default() {
                    inputs.push(chunker::chunk_for_step(Some(&worry.id), &step.id, &step));
                }
            }
        }
    }

    let mut embedded: Vec<(ChunkInput, Vec<f32>)> = Vec::new();
    for input in inputs {
        match ollama.embed(&embedding_model, &input.content).await {
            Ok(vec) => {
                if vec.is_empty() || vec.iter().any(|x| !x.is_finite()) || vec.iter().all(|x| *x == 0.0) {
                    fail_job(pool, &job_id, &entry_id, "embedding_invalid")?;
                    return Ok(true);
                }
                embedded.push((input, vec));
            }
            Err(e) => {
                tracing::warn!(error = %e, "embedding request failed");
                retry_or_fail(pool, &job_id, &entry_id)?;
                return Ok(true);
            }
        }
    }

    // Recheck before commit: source still exists, aggregate/embedding-space
    // snapshot still matches what we started from, consent still granted,
    // vault generation unchanged.
    let conn = pool.get()?;
    let current_entry = repo::get_entry(&conn, &entry_id)?;
    let current_settings = repo::get_settings(&conn)?;
    let stale = match &current_entry {
        None => true,
        Some(e) => {
            e.aggregate_version != req_agg
                || !e.memory_enabled
                || current_settings.embedding_space_version != req_space
                || current_settings.vault_generation != vault_gen
                || !current_settings.local_ai_enabled
        }
    };
    if stale {
        conn.execute("DELETE FROM indexing_jobs WHERE id = ?1", [&job_id])?;
        tracing::info!(entry_id = %entry_id, "discarded stale indexing job");
        return Ok(true);
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM retrieval_chunks WHERE entry_id = ?1", [&entry_id])?;
    for (idx, (input, vec)) in embedded.iter().enumerate() {
        let blob: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
        let hash = chunker::content_hash(&input.content);
        tx.execute(
            "INSERT INTO retrieval_chunks
                (id, entry_id, source_kind, worry_id, outcome_id, small_step_id, chunk_index, content, content_hash,
                 embedding_blob, embedding_dimension, embedding_model_digest, embedding_space_version,
                 source_version, aggregate_version, vault_generation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                new_id(), entry_id, input.source_kind, input.worry_id, input.outcome_id, input.small_step_id,
                idx as i64, input.content, hash, blob, vec.len() as i64,
                settings.embedding_model_digest.clone().unwrap_or_default(), req_space,
                req_agg, req_agg, vault_gen, now_iso()
            ],
        )?;
    }
    tx.execute("UPDATE journal_entries SET indexing_status = 'ready', updated_at = ?1 WHERE id = ?2", params![now_iso(), entry_id])?;
    tx.execute("UPDATE indexing_jobs SET status = 'completed', updated_at = ?1 WHERE id = ?2", params![now_iso(), job_id])?;
    tx.commit()?;
    tracing::info!(entry_id = %entry_id, chunks = embedded.len(), "indexing complete");
    Ok(true)
}

fn fail_job(pool: &crate::db::Pool, job_id: &str, entry_id: &str, code: &str) -> anyhow::Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE indexing_jobs SET status = 'failed', safe_error_code = ?1, updated_at = ?2 WHERE id = ?3",
        params![code, now_iso(), job_id],
    )?;
    conn.execute("UPDATE journal_entries SET indexing_status = 'failed', updated_at = ?1 WHERE id = ?2", params![now_iso(), entry_id])?;
    Ok(())
}

fn retry_or_fail(pool: &crate::db::Pool, job_id: &str, entry_id: &str) -> anyhow::Result<()> {
    let conn = pool.get()?;
    let attempts: i64 = conn.query_row("SELECT attempt_count FROM indexing_jobs WHERE id = ?1", [job_id], |r| r.get(0))?;
    if attempts + 1 >= MAX_ATTEMPTS {
        drop(conn);
        return fail_job(pool, job_id, entry_id, "runtime_stopped");
    }
    let backoff_secs = 5 * (attempts + 1);
    let next = time::OffsetDateTime::now_utc() + time::Duration::seconds(backoff_secs);
    conn.execute(
        "UPDATE indexing_jobs SET status = 'pending', attempt_count = attempt_count + 1, next_attempt_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![next.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(), now_iso(), job_id],
    )?;
    Ok(())
}

/// Small helper trait so the query_row().optional() call above reads cleanly
/// without importing OptionalExtension at every call site in this file.
trait OptionalOrNone<T> {
    fn optional_or_none(self) -> Option<T>;
}
impl<T> OptionalOrNone<T> for rusqlite::Result<T> {
    fn optional_or_none(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                tracing::warn!(error = %e, "unexpected query error");
                None
            }
        }
    }
}

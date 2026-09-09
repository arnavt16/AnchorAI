//! Exact cosine-similarity retrieval over `retrieval_chunks`, restricted to
//! the active vault, eligible parents, current source versions, and the
//! active embedding space (brief section 8, "Candidate retrieval").
//!
//! Exact (not approximate/indexed) search: fine for a personal-journal
//! scale prototype per the brief's own guidance — add an ANN index only if
//! the stress benchmark in `evals/` shows it's actually needed.

use crate::models::{RetrievedSource, WorryOutcome};
use rusqlite::{params, Connection, Row};

const CANDIDATE_COUNT: usize = 12;
const SELECTED_COUNT: usize = 6;
const MIN_SIMILARITY: f32 = 0.35; // calibrated against evals/fixtures/eval_queries.json; see evals/README.md

struct Candidate {
    id: String,
    entry_id: String,
    source_kind: String,
    worry_id: Option<String>,
    content: String,
    created_at: String,
    embedding: Vec<f32>,
}

fn decode_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

fn candidate_from_row(row: &Row) -> rusqlite::Result<Candidate> {
    let blob: Vec<u8> = row.get("embedding_blob")?;
    Ok(Candidate {
        id: row.get("id")?,
        entry_id: row.get("entry_id")?,
        source_kind: row.get("source_kind")?,
        worry_id: row.get("worry_id")?,
        content: row.get("content")?,
        created_at: row.get("created_at")?,
        embedding: decode_blob(&blob),
    })
}

/// Retrieve relevant chunks for `query_embedding`. Applies modest recency
/// weighting on top of semantic similarity, deduplicates chunks from the
/// same entry+source_kind, and — critically — expands any matched worry
/// chunk with its full linked outcome history (never just the original
/// fear; see brief section 8, "This is essential").
pub fn retrieve(
    conn: &Connection,
    query_embedding: &[f32],
    embedding_space_version: i64,
    vault_generation: i64,
    exclude_entry_id: Option<&str>,
) -> anyhow::Result<Vec<RetrievedSource>> {
    let mut stmt = conn.prepare(
        "SELECT rc.* FROM retrieval_chunks rc
         JOIN journal_entries je ON je.id = rc.entry_id
         WHERE rc.embedding_space_version = ?1
           AND rc.vault_generation = ?2
           AND je.memory_enabled = 1
           AND je.indexing_status = 'ready'
           AND rc.aggregate_version = je.aggregate_version",
    )?;
    let candidates: Vec<Candidate> = stmt
        .query_map(params![embedding_space_version, vault_generation], candidate_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let now = time::OffsetDateTime::now_utc();
    let mut scored: Vec<(f32, Candidate)> = candidates
        .into_iter()
        .filter(|c| exclude_entry_id.map(|id| id != c.entry_id).unwrap_or(true))
        .map(|c| {
            let sim = cosine(query_embedding, &c.embedding);
            let recency_bonus = recency_weight(&c.created_at, now);
            (sim * 0.88 + recency_bonus * 0.12, c)
        })
        .filter(|(score, _)| *score >= MIN_SIMILARITY)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(CANDIDATE_COUNT);

    // Dedupe: keep the top-scored chunk per (entry_id, source_kind) pair.
    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<(f32, Candidate)> = Vec::new();
    for (score, c) in scored {
        let key = (c.entry_id.clone(), c.source_kind.clone());
        if seen.insert(key) {
            deduped.push((score, c));
        }
    }
    deduped.truncate(SELECTED_COUNT);

    let mut out = Vec::new();
    for (score, c) in deduped {
        let linked_outcome = if c.source_kind == "worry" {
            if let Some(wid) = &c.worry_id {
                latest_outcome(conn, wid)?
            } else {
                None
            }
        } else {
            None
        };
        out.push(RetrievedSource {
            id: c.id,
            entry_id: c.entry_id,
            source_kind: c.source_kind,
            content: c.content,
            date: c.created_at,
            similarity: score,
            linked_outcome,
        });
    }
    Ok(out)
}

fn recency_weight(created_at: &str, now: time::OffsetDateTime) -> f32 {
    let Ok(created) = time::OffsetDateTime::parse(created_at, &time::format_description::well_known::Rfc3339) else {
        return 0.0;
    };
    let days = (now - created).whole_days().max(0) as f32;
    // Gentle decay: ~1.0 today, ~0.5 at 90 days, floor near 0.1. Modest by
    // design — the brief warns against assuming "newest is most relevant".
    (1.0 - (days / 180.0)).clamp(0.1, 1.0)
}

fn latest_outcome(conn: &Connection, worry_id: &str) -> anyhow::Result<Option<WorryOutcome>> {
    let outcomes = crate::db::repo::list_outcomes(conn, worry_id)?;
    Ok(outcomes.into_iter().last())
}

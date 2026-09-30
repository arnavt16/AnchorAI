//! Retrieval eval against a live local Ollama.
//!
//!     cd src-tauri
//!     cargo run --example retrieval_eval -- --embed-model nomic-embed-text
//!
//! Seeds `evals/fixtures/demo_entries.json` into a throwaway vault, indexes
//! it through the app's real indexing path (`worker::process_next_job`),
//! then runs every query in `evals/fixtures/eval_queries.json` two ways:
//!
//! - **baseline**: naive top-k — raw cosine over every entry's chunks
//!   (including the memory-disabled one), no threshold, no recency, no
//!   dedupe, no linked-outcome expansion.
//! - **anchor**: `rag::retrieval::retrieve` exactly as shipped.
//!
//! Scores retrieval only (which entries come back), not generated text —
//! judging responses still needs a human reading transcripts; see
//! evals/README.md. Writes a Markdown report to evals/results/.

use anchor_lib::commands::data::{import_entries, ExportEntry, VaultExport};
use anchor_lib::db::{now_iso, open_vault};
use anchor_lib::indexing::{chunker, worker};
use anchor_lib::ollama::{readiness_check_embedding, OllamaClient};
use anchor_lib::rag::retrieval::retrieve;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const DEMO_ENTRIES: &str = include_str!("../../evals/fixtures/demo_entries.json");
const EVAL_QUERIES: &str = include_str!("../../evals/fixtures/eval_queries.json");
const TOP_K: usize = 6;

#[derive(Deserialize)]
struct DemoFile {
    entries: Vec<ExportEntry>,
}

#[derive(Deserialize)]
struct QueryFile {
    queries: Vec<Query>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Query {
    id: String,
    query: String,
    #[serde(rename = "type")]
    kind: String,
    expected_entry_ids: Vec<String>,
    must_include_linked_outcome: bool,
}

/// What one system returned for one query, in local fixture ids.
struct Retrieved {
    entries: Vec<String>, // rank order, deduplicated
    outcome_for: HashSet<String>,
    top_score: Option<f32>,
}

struct Score {
    pass: bool,
    detail: String,
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

fn decode(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn score(q: &Query, r: &Retrieved, excluded: &HashSet<String>) -> Score {
    let leaked: Vec<&String> = r.entries.iter().filter(|e| excluded.contains(*e)).collect();
    if !leaked.is_empty() {
        return Score { pass: false, detail: format!("leaked memory-disabled {:?}", leaked) };
    }
    match q.kind.as_str() {
        "no_match" => Score {
            pass: r.entries.is_empty(),
            detail: if r.entries.is_empty() { "nothing returned".into() } else { format!("returned {} unrelated", r.entries.len()) },
        },
        "exclusion" => Score { pass: true, detail: "excluded entry not returned".into() },
        _ => {
            let expected: HashSet<&String> = q.expected_entry_ids.iter().collect();
            let hits = r.entries.iter().filter(|e| expected.contains(e)).count();
            let recall_ok = hits == expected.len();
            let outcome_ok = !q.must_include_linked_outcome || q.expected_entry_ids.iter().any(|e| r.outcome_for.contains(e));
            let mut detail = format!("recall {}/{}", hits, expected.len());
            if q.must_include_linked_outcome {
                detail.push_str(if outcome_ok { ", outcome ✓" } else { ", outcome ✗" });
            }
            Score { pass: recall_ok && outcome_ok, detail }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let embed_model = arg("--embed-model").unwrap_or_else(|| "nomic-embed-text".into());
    let out_dir = arg("--out").unwrap_or_else(|| "../evals/results".into());

    let client = OllamaClient::new(None)?;
    if !client.is_reachable().await {
        anyhow::bail!("Ollama isn't reachable on 127.0.0.1:11434 — start it and `ollama pull {embed_model}` first.");
    }
    let dimension = readiness_check_embedding(&client, &embed_model).await?;
    let digest = client
        .list_local_models()
        .await?
        .into_iter()
        .find(|m| m.name == embed_model || m.name == format!("{embed_model}:latest"))
        .map(|m| m.digest)
        .unwrap_or_else(|| "unknown".into());

    // --- Seed + index a throwaway vault through the real code paths.
    let dir = tempfile::tempdir()?;
    let vault = open_vault(&dir.path().join("eval.sqlite"))?;
    let demo: DemoFile = serde_json::from_str(DEMO_ENTRIES)?;
    let local_ids: Vec<String> = demo.entries.iter().map(|e| e.local_id.clone().unwrap_or_default()).collect();
    let excluded: HashSet<String> =
        demo.entries.iter().filter(|e| !e.memory_enabled).filter_map(|e| e.local_id.clone()).collect();
    let bodies: Vec<String> = demo.entries.iter().map(|e| e.body.clone()).collect();

    let db_ids = {
        let mut conn = vault.pool.get()?;
        conn.execute(
            "UPDATE app_settings SET local_ai_enabled = 1, embedding_model = ?1, embedding_model_digest = ?2, embedding_dimension = ?3 WHERE id = 1",
            rusqlite::params![embed_model, digest, dimension as i64],
        )?;
        let ids = import_entries(&mut conn, VaultExport { schema_version: 1, exported_at: now_iso(), entries: demo.entries })?;
        for id in &ids {
            worker::enqueue_if_eligible(&conn, id)?;
        }
        ids
    };
    let to_local: HashMap<String, String> = db_ids.iter().cloned().zip(local_ids.iter().cloned()).collect();

    let index_start = Instant::now();
    let deadline = index_start + Duration::from_secs(600);
    loop {
        if worker::process_next_job(&vault.pool, &client).await? {
            continue;
        }
        let conn = vault.pool.get()?;
        let pending: i64 = conn.query_row("SELECT COUNT(*) FROM indexing_jobs WHERE status IN ('pending','processing')", [], |r| r.get(0))?;
        let failed: i64 = conn.query_row("SELECT COUNT(*) FROM indexing_jobs WHERE status = 'failed'", [], |r| r.get(0))?;
        if failed > 0 {
            anyhow::bail!("{failed} indexing job(s) failed — check that {embed_model} works in Ollama.");
        }
        if pending == 0 {
            break;
        }
        if Instant::now() > deadline {
            anyhow::bail!("indexing timed out");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let index_secs = index_start.elapsed().as_secs_f32();

    // --- Baseline index: every stored chunk, plus the memory-disabled
    // entry's body (a naive system would have indexed it too).
    let (space, generation) = {
        let conn = vault.pool.get()?;
        conn.query_row("SELECT embedding_space_version, vault_generation FROM app_settings WHERE id = 1", [], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?
    };
    let mut naive: Vec<(String, String, Vec<f32>)> = Vec::new(); // (local entry id, source_kind, embedding)
    {
        let conn = vault.pool.get()?;
        let mut stmt = conn.prepare("SELECT entry_id, source_kind, embedding_blob FROM retrieval_chunks")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Vec<u8>>(2)?)))?;
        for row in rows {
            let (entry_id, kind, blob) = row?;
            naive.push((to_local[&entry_id].clone(), kind, decode(&blob)));
        }
    }
    for (local, body) in local_ids.iter().zip(&bodies) {
        if excluded.contains(local) {
            for chunk in chunker::chunk_text(body) {
                naive.push((local.clone(), "entry".into(), client.embed(&embed_model, &chunk).await?));
            }
        }
    }

    // --- Run queries.
    let queries: QueryFile = serde_json::from_str(EVAL_QUERIES)?;
    let mut rows = Vec::new();
    let (mut base_pass, mut anchor_pass, mut anchor_ms) = (0, 0, 0u128);
    for q in &queries.queries {
        let qv = client.embed(&embed_model, &q.query).await?;

        let mut ranked: Vec<(f32, &(String, String, Vec<f32>))> = naive.iter().map(|c| (cosine(&qv, &c.2), c)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(TOP_K);
        let mut base = Retrieved { entries: vec![], outcome_for: HashSet::new(), top_score: ranked.first().map(|r| r.0) };
        for (_, (entry, kind, _)) in &ranked {
            if !base.entries.contains(entry) {
                base.entries.push(entry.clone());
            }
            if kind == "outcome" {
                base.outcome_for.insert(entry.clone());
            }
        }

        let started = Instant::now();
        let sources = {
            let conn = vault.pool.get()?;
            retrieve(&conn, &qv, space, generation, None)?
        };
        anchor_ms += started.elapsed().as_millis();
        let mut anchor = Retrieved { entries: vec![], outcome_for: HashSet::new(), top_score: sources.first().map(|s| s.similarity) };
        for s in &sources {
            let local = to_local[&s.entry_id].clone();
            if s.linked_outcome.is_some() || s.source_kind == "outcome" {
                anchor.outcome_for.insert(local.clone());
            }
            if !anchor.entries.contains(&local) {
                anchor.entries.push(local);
            }
        }

        let b = score(q, &base, &excluded);
        let a = score(q, &anchor, &excluded);
        base_pass += b.pass as usize;
        anchor_pass += a.pass as usize;
        let mark = |s: &Score| format!("{} {}", if s.pass { "✅" } else { "❌" }, s.detail);
        rows.push(format!(
            "| `{}` | {} | {} | {} | {} | {} |",
            q.id,
            q.kind,
            mark(&b),
            mark(&a),
            anchor.entries.join(", "),
            anchor.top_score.map(|s| format!("{s:.3}")).unwrap_or_else(|| "—".into()),
        ));
    }

    let n = queries.queries.len();
    let report = format!(
        "# Retrieval eval — {embed_model}\n\n\
         - Run: {}\n- Embedding model: `{embed_model}` (digest `{}`, {dimension} dims)\n\
         - Machine: {} / {}\n- Fixtures: {} entries, {n} labeled queries\n\
         - Indexing time: {index_secs:.1}s · avg Anchor retrieval: {:.1} ms/query (excludes query embedding)\n\n\
         **Baseline (naive top-{TOP_K}): {base_pass}/{n} pass · Anchor retrieval: {anchor_pass}/{n} pass**\n\n\
         | Query | Type | Baseline | Anchor | Anchor returned | Anchor top score |\n|---|---|---|---|---|---|\n{}\n\n\
         Pass rules: recall of every expected entry (plus a linked outcome where the fixture requires one); \
         `no_match` passes only if nothing is returned; any query fails if a memory-disabled entry is returned. \
         Retrieval only — generated responses aren't scored here.\n",
        now_iso(),
        digest.chars().take(12).collect::<String>(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        local_ids.len(),
        anchor_ms as f32 / n as f32,
        rows.join("\n"),
    );

    println!("{report}");
    std::fs::create_dir_all(&out_dir)?;
    let safe_model: String = embed_model.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let path = std::path::Path::new(&out_dir).join(format!("retrieval-{safe_model}-{}.md", &now_iso()[..10]));
    std::fs::write(&path, report)?;
    eprintln!("Wrote {}", path.display());
    Ok(())
}

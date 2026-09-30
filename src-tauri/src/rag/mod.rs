pub mod generation;
pub mod retrieval;

use crate::db::repo;
use crate::db::Pool;
use crate::models::{AppSettings, ChatTurn, ValidatedReflection};
use crate::ollama::OllamaClient;
use crate::safety::{self, DistressLevel};

pub struct ReflectRequest<'a> {
    pub message: &'a str,
    pub intention: Option<&'a str>,
    pub use_memory: bool,
    pub exclude_entry_id: Option<&'a str>,
    /// Earlier turns of the in-memory conversation, oldest first.
    pub history: &'a [ChatTurn],
}

/// Text to embed for retrieval. A follow-up like "what about the second
/// one?" embeds to almost nothing useful on its own, so the previous user
/// turn is prepended to carry the topic forward.
pub fn retrieval_query_text(message: &str, history: &[ChatTurn]) -> String {
    match history.iter().rev().find(|t| t.role == "user") {
        Some(prev) => {
            let prev: String = prev.content.chars().take(generation::MAX_HISTORY_TURN_CHARS).collect();
            format!("{}\n{}", prev.trim(), message)
        }
        None => message.to_string(),
    }
}

/// Top-level reflection pipeline: end-to-end retrieval/generation plus
/// distress routing. `Imminent` skips the model entirely; `Elevated` runs
/// normal reflection with an extra system instruction and attaches a
/// bundled support note to whatever comes back.
///
/// Takes a `&Pool` rather than a `&Connection` deliberately: rusqlite's
/// `Connection` is `!Sync`, so a Tauri async command must never hold one
/// across an `.await` point (the generated command future has to be
/// `Send`). Each synchronous DB step below gets its own short-lived pooled
/// connection that is dropped before the next `await`.
pub async fn reflect(pool: &Pool, client: &OllamaClient, req: ReflectRequest<'_>) -> anyhow::Result<ValidatedReflection> {
    let distress = safety::classify(req.message);
    if distress == DistressLevel::Imminent {
        let country = {
            let conn = pool.get()?;
            repo::get_settings(&conn)?.support_country
        };
        let msg = safety::imminent_danger_response(country.as_deref());
        return Ok(ValidatedReflection {
            sections: vec![crate::models::ReflectionSection { text: msg, source_ids: vec![] }],
            follow_up_question: None,
            suggested_step: None,
            sources: vec![],
            used_memory: false,
            urgent_path_triggered: true,
            citations_verified: false,
            support_note: None,
        });
    }

    let settings = {
        let conn = pool.get()?;
        repo::get_settings(&conn)?
    };

    let elevated = distress == DistressLevel::Elevated;
    let support_note = elevated.then(|| safety::elevated_support_note(settings.support_country.as_deref()));

    let mut reflection = match reflect_with_model(pool, client, &req, &settings, elevated).await {
        Ok(r) => r,
        // A runtime failure must never swallow the support note: fall back
        // to the honest "couldn't reach the model" reply and still show it.
        Err(e) if elevated => {
            tracing::warn!(error = %e, "reflection failed on elevated path; returning bundled note");
            generation::fallback_reflection(&[])
        }
        Err(e) => return Err(e),
    };
    reflection.support_note = support_note;
    Ok(reflection)
}

/// Retrieval + generation + the post-generation citation recheck: every
/// step that depends on the model runtime.
async fn reflect_with_model(
    pool: &Pool,
    client: &OllamaClient,
    req: &ReflectRequest<'_>,
    settings: &AppSettings,
    elevated: bool,
) -> anyhow::Result<ValidatedReflection> {
    let sources = if req.use_memory && settings.local_ai_enabled {
        if let Some(embedding_model) = settings.embedding_model.clone() {
            let query_vec = client.embed(&embedding_model, &retrieval_query_text(req.message, req.history)).await?;
            let conn = pool.get()?;
            retrieval::retrieve(&conn, &query_vec, settings.embedding_space_version, settings.vault_generation, req.exclude_entry_id)?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let Some(chat_model) = settings.chat_model.clone() else {
        return Ok(generation::fallback_reflection(&sources));
    };

    let ctx = generation::GenerationContext {
        user_message: req.message,
        intention: req.intention,
        sources: &sources,
        history: req.history,
        elevated_distress: elevated,
    };
    let reflection = generation::generate_reflection(client, &chat_model, &ctx).await?;

    // Recheck cited sources are still current before returning — discard
    // and regenerate if source data changed mid-flight.
    let cited_ids: std::collections::HashSet<String> =
        reflection.sections.iter().flat_map(|s| s.source_ids.iter().cloned()).collect();
    if !cited_ids.is_empty() {
        let conn = pool.get()?;
        let still_valid = sources_still_current(&conn, &cited_ids, settings.vault_generation)?;
        if !still_valid {
            tracing::info!("cited sources changed mid-generation; returning honest fallback");
            return Ok(generation::fallback_reflection(&[]));
        }
    }

    Ok(reflection)
}

fn sources_still_current(conn: &rusqlite::Connection, ids: &std::collections::HashSet<String>, vault_generation: i64) -> anyhow::Result<bool> {
    for id in ids {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM retrieval_chunks WHERE id = ?1 AND vault_generation = ?2)",
                rusqlite::params![id, vault_generation],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !exists {
            return Ok(false);
        }
    }
    Ok(true)
}

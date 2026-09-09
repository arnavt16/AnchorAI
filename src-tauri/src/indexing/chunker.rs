//! Paragraph-aware chunking, targeting ~300-500 tokens per chunk.
//!
//! We don't bundle a real tokenizer (keeps the dependency tree and binary
//! size down for a portfolio-scale prototype). Word count is a conservative
//! proxy: English text averages meaningfully more than one token per word,
//! so a 300-500 *word* cap keeps chunks well under typical embedding-model
//! context limits rather than silently truncating. This is documented as an
//! approximation, not presented as exact token accounting.

use sha2::{Digest, Sha256};

pub const TARGET_MIN_WORDS: usize = 220;
pub const TARGET_MAX_WORDS: usize = 420;

#[derive(Debug, Clone)]
pub struct ChunkInput {
    pub source_kind: &'static str, // "entry" | "worry" | "outcome" | "small_step"
    pub worry_id: Option<String>,
    pub outcome_id: Option<String>,
    pub small_step_id: Option<String>,
    pub content: String,
}

pub fn content_hash(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    format!("{:x}", h.finalize())
}

fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Split `text` into paragraph-aware chunks. Short text stays a single
/// chunk; long text is grouped by paragraph boundaries (blank lines) up to
/// the target word budget, and a lone paragraph that still exceeds the max
/// is hard-wrapped by sentence-ish boundaries so nothing is silently
/// dropped or truncated.
pub fn chunk_text(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    if word_count(trimmed) <= TARGET_MAX_WORDS {
        return vec![trimmed.to_string()];
    }

    let paragraphs: Vec<&str> = trimmed.split("\n\n").map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_words = 0usize;

    for para in paragraphs {
        let pw = word_count(para);
        if pw > TARGET_MAX_WORDS {
            // A single oversized paragraph: flush what we have, then hard-wrap this one.
            if !current.is_empty() {
                chunks.push(current.clone());
                current.clear();
                current_words = 0;
            }
            chunks.extend(hard_wrap(para));
            continue;
        }
        if current_words + pw > TARGET_MAX_WORDS && current_words >= TARGET_MIN_WORDS {
            chunks.push(current.clone());
            current.clear();
            current_words = 0;
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);
        current_words += pw;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn hard_wrap(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .chunks(TARGET_MAX_WORDS)
        .map(|w| w.join(" "))
        .collect()
}

pub fn chunks_for_entry_body(body: &str) -> Vec<ChunkInput> {
    chunk_text(body)
        .into_iter()
        .map(|c| ChunkInput {
            source_kind: "entry",
            worry_id: None,
            outcome_id: None,
            small_step_id: None,
            content: c,
        })
        .collect()
}

pub fn chunk_for_worry(worry_id: &str, worry_text: &str, expected_outcome: Option<&str>) -> ChunkInput {
    let mut content = format!("Worry: {}", worry_text);
    if let Some(exp) = expected_outcome {
        content.push_str(&format!("\nWhat they thought might happen: {}", exp));
    }
    ChunkInput {
        source_kind: "worry",
        worry_id: Some(worry_id.to_string()),
        outcome_id: None,
        small_step_id: None,
        content,
    }
}

pub fn chunk_for_outcome(worry_id: &str, outcome_id: &str, outcome: &crate::models::WorryOutcome) -> ChunkInput {
    let mut content = format!("Outcome recorded {}: {}", outcome.recorded_at, outcome.outcome_text);
    if let Some(cat) = &outcome.result_category {
        content.push_str(&format!("\nResult category: {}", cat));
    }
    if let Some(r) = &outcome.reflection {
        content.push_str(&format!("\nReflection: {}", r));
    }
    ChunkInput {
        source_kind: "outcome",
        worry_id: Some(worry_id.to_string()),
        outcome_id: Some(outcome_id.to_string()),
        small_step_id: None,
        content,
    }
}

pub fn chunk_for_step(worry_id: Option<&str>, step_id: &str, step: &crate::models::SmallStep) -> ChunkInput {
    let mut content = format!("Small step tried: {}", step.action_text);
    if let Some(fb) = &step.feedback {
        content.push_str(&format!("\nFeedback: {}", fb));
    }
    if let Some(note) = &step.feedback_note {
        content.push_str(&format!(" ({})", note));
    }
    ChunkInput {
        source_kind: "small_step",
        worry_id: worry_id.map(|s| s.to_string()),
        outcome_id: None,
        small_step_id: Some(step_id.to_string()),
        content,
    }
}

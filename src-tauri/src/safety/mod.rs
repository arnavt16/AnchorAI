//! Urgent-distress routing.
//!
//! This is a conservative, local, keyword-anchored pre-filter — explicitly
//! NOT presented as reliable clinical triage. It exists so that a bundled,
//! reviewed supportive response is available even when Ollama is down, and
//! so normal memory-based reflection is skipped for evident imminent-danger
//! statements. It is deliberately biased toward false positives (routing a
//! merely dramatic sentence to the supportive path costs little) rather
//! than false negatives. A local model classifier or a real crisis-triage
//! system is future work — see SECURITY.md limitations and
//! `evals/fixtures/safety_fixtures.json` for the labeled test set this was
//! tuned against, none of which is a substitute for the human review
//! required before any public release.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistressLevel {
    None,
    Elevated,
    Imminent,
}

/// High-signal phrases for evident imminent danger. Kept short and
/// conservative on purpose; figurative language ("this deadline is going to
/// kill me") is intentionally NOT matched — see the figurative-language
/// fixtures in evals/.
const IMMINENT_PATTERNS: &[&str] = &[
    "kill myself",
    "end my life",
    "ending my life",
    "end it all",
    "ending it all",
    "want to die",
    "wanting to die",
    "suicide plan",
    "going to end it",
    "not going to be here",
    "planning to hurt myself",
    "ready to jump",
    "have the pills",
];

const ELEVATED_PATTERNS: &[&str] = &[
    "self harm",
    "self-harm",
    "hurting myself",
    "can't go on",
    "no reason to keep going",
    "better off without me",
];

pub fn classify(text: &str) -> DistressLevel {
    let lower = text.to_lowercase();
    if IMMINENT_PATTERNS.iter().any(|p| lower.contains(p)) {
        return DistressLevel::Imminent;
    }
    if ELEVATED_PATTERNS.iter().any(|p| lower.contains(p)) {
        return DistressLevel::Elevated;
    }
    DistressLevel::None
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CrisisResource {
    pub country: &'static str,
    pub label: &'static str,
    pub contact: &'static str,
    pub source_url: &'static str,
    pub verified_at: &'static str,
}

/// Small, auditable, offline-bundled resource list. Verify and refresh
/// these during an actual reviewed release — do not treat this list as
/// currently re-verified; `verified_at` records when it was last checked
/// against the source, and this build has not re-checked it against the
/// live sources listed.
pub const CRISIS_RESOURCES: &[CrisisResource] = &[
    CrisisResource {
        country: "US",
        label: "988 Suicide & Crisis Lifeline",
        contact: "Call or text 988",
        source_url: "https://988lifeline.org",
        verified_at: "unverified-in-this-build",
    },
    CrisisResource {
        country: "UK",
        label: "Samaritans",
        contact: "Call 116 123 (free, 24/7)",
        source_url: "https://www.samaritans.org",
        verified_at: "unverified-in-this-build",
    },
];

pub fn resources_for_country(country: Option<&str>) -> Vec<CrisisResource> {
    match country {
        Some(c) => CRISIS_RESOURCES.iter().filter(|r| r.country.eq_ignore_ascii_case(c)).cloned().collect(),
        None => vec![],
    }
}

/// Bundled, reviewed supportive response for evident imminent-danger
/// statements. Must remain available even if Ollama is unreachable —
/// callers should show this before ever attempting a model call.
pub fn imminent_danger_response(country: Option<&str>) -> String {
    let mut msg = String::from(
        "What you're describing sounds like it may be an emergency. I'm not able to \
         provide crisis support myself, and I want you to reach real, immediate help \
         right now rather than continue this conversation: contact your local \
         emergency number, a crisis line, or a person you trust who can be with you.",
    );
    let resources = resources_for_country(country);
    if !resources.is_empty() {
        msg.push_str("\n\nBundled resources for your region:");
        for r in resources {
            msg.push_str(&format!("\n- {}: {} ({})", r.label, r.contact, r.source_url));
        }
    } else {
        msg.push_str(
            "\n\nI don't have a verified regional resource on file for you yet — you can set \
             your country in Settings so Anchor can show one, or search for a local crisis \
             line or emergency number directly.",
        );
    }
    msg
}

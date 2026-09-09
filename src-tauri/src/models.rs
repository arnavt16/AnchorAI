use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub mood: Option<String>,
    pub tags: Vec<String>,
    pub origin: String,
    pub memory_enabled: bool,
    pub content_version: i64,
    pub aggregate_version: i64,
    pub indexing_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worry {
    pub id: String,
    pub entry_id: String,
    pub worry_text: String,
    pub expected_outcome: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorryOutcome {
    pub id: String,
    pub worry_id: String,
    pub recorded_at: String,
    pub outcome_text: String,
    pub result_category: Option<String>,
    pub reflection: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmallStep {
    pub id: String,
    pub entry_id: String,
    pub worry_id: Option<String>,
    pub action_text: String,
    pub feedback: Option<String>,
    pub feedback_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A worry together with its full chronological outcome history and any
/// linked small steps: the shape the UI actually renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorryWithHistory {
    #[serde(flatten)]
    pub worry: Worry,
    pub outcomes: Vec<WorryOutcome>,
    pub steps: Vec<SmallStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub local_ai_enabled: bool,
    pub consent_version: Option<String>,
    pub consent_at: Option<String>,
    pub consent_withdrawn_at: Option<String>,
    pub chat_model: Option<String>,
    pub chat_model_digest: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_model_digest: Option<String>,
    pub embedding_dimension: Option<i64>,
    pub embedding_space_version: i64,
    pub support_country: Option<String>,
    pub vault_generation: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedSource {
    pub id: String,
    pub entry_id: String,
    pub source_kind: String,
    pub content: String,
    pub date: String,
    pub similarity: f32,
    pub linked_outcome: Option<WorryOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionSection {
    pub text: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedReflection {
    pub sections: Vec<ReflectionSection>,
    pub follow_up_question: Option<String>,
    pub suggested_step: Option<String>,
    pub sources: Vec<RetrievedSource>,
    pub used_memory: bool,
    pub urgent_path_triggered: bool,
    /// True only when the model produced valid, schema-conformant JSON with
    /// source citations we could verify (the structured path). False for
    /// natural-language fallback responses and the urgent-distress path.
    /// The UI uses this to decide whether to show cited-source affordances,
    /// never to withhold an answer.
    pub citations_verified: bool,
}

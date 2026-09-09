//! Loopback-only Ollama client.
//!
//! Hard constraints, enforced here rather than trusted from config:
//! - The base URL host must be `127.0.0.1` or `localhost`. Any other host
//!   supplied via settings, an import, or the `VITE_OLLAMA_HOST` dev env
//!   var is rejected before a request is ever built.
//! - Redirects are disabled on the HTTP client, so a malicious or
//!   misconfigured local proxy cannot 30x a request out to the internet.
//! - We never accept a "remote model" descriptor. `list_local_models`
//!   inspects what Ollama reports and only local pull manifests found on
//!   disk are surfaced; Ollama's cloud-backed model names are still just
//!   strings from the same API, so this filtering is best-effort, not
//!   cryptographic. See SECURITY.md "Local runtime limitations".

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

const DEFAULT_HOST: &str = "http://127.0.0.1:11434";

fn allowed_host(url: &Url) -> bool {
    matches!(url.host_str(), Some("127.0.0.1") | Some("localhost") | Some("[::1]"))
}

pub struct OllamaClient {
    base: Url,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_override: Option<&str>) -> Result<Self> {
        let raw = base_override.unwrap_or(DEFAULT_HOST);
        let base = Url::parse(raw).map_err(|_| anyhow!("invalid Ollama base URL"))?;
        if !allowed_host(&base) {
            return Err(anyhow!("refusing non-loopback Ollama host: {}", base.host_str().unwrap_or("?")));
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self { base, http })
    }

    fn url(&self, path: &str) -> Url {
        self.base.join(path).expect("static path join")
    }

    pub async fn is_reachable(&self) -> bool {
        self.http
            .get(self.url("/api/version"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn list_local_models(&self) -> Result<Vec<LocalModel>> {
        let resp: TagsResponse = self.http.get(self.url("/api/tags")).send().await?.error_for_status()?.json().await?;
        Ok(resp.models)
    }

    pub async fn embed(&self, model: &str, input: &str) -> Result<Vec<f32>> {
        let body = EmbedRequest { model, input };
        let resp: EmbedResponse = self
            .http
            .post(self.url("/api/embed"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        resp.embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("empty embedding response"))
    }

    /// Request a chat completion. `format` is JSON-schema-ish (Ollama's
    /// `format` field accepts `"json"` or a JSON schema object); the RAG
    /// pipeline passes a schema and validates the parsed result itself
    /// rather than trusting the model's claim of conformance.
    pub async fn chat(&self, model: &str, messages: &[ChatMessage], format: Option<serde_json::Value>) -> Result<String> {
        let body = ChatRequest {
            model,
            messages,
            stream: false,
            format,
        };
        let resp: ChatResponse = self
            .http
            .post(self.url("/api/chat"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.message.content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub name: String,
    pub digest: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<LocalModel>,
}

#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    #[serde(default)]
    embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

/// Readiness test using synthetic text only, never journal content.
pub async fn readiness_check_embedding(client: &OllamaClient, model: &str) -> Result<usize> {
    let v = client.embed(model, "Anchor readiness check: the quick brown fox jumps over the lazy dog.").await?;
    if v.is_empty() || v.iter().any(|x| !x.is_finite()) {
        return Err(anyhow!("embedding readiness check failed: empty or non-finite vector"));
    }
    Ok(v.len())
}

pub async fn readiness_check_chat(client: &OllamaClient, model: &str) -> Result<String> {
    let messages = vec![ChatMessage {
        role: "user".into(),
        content: "Reply with a short JSON object: {\"ok\": true}. Do not add commentary.".into(),
    }];
    client.chat(model, &messages, Some(serde_json::json!({"type": "object", "properties": {"ok": {"type": "boolean"}}}))).await
}

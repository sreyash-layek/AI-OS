use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ToolPreview {
    pub name: String,
    pub risk_tier: u8,
    pub requires_confirmation: bool,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub mode: &'static str,
    pub tool_preview: ToolPreview,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub service: String,
    pub status: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventEnvelope {
    pub event: String,
    pub at: String,
    pub source: &'static str,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SpeakRequest {
    pub text: String,
    pub voice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpeakResponse {
    pub ok: bool,
    pub request_id: String,
    pub mode: &'static str,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceSettings {
    pub provider: String,
    pub auto_speak: bool,
    pub default_voice: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVoiceSettingsRequest {
    pub provider: Option<String>,
    pub auto_speak: Option<bool>,
    pub default_voice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoiceProviderHealth {
    pub configured_provider: String,
    pub effective_provider: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexScope {
    pub id: String,
    pub path: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateIndexScopeRequest {
    pub path: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct IndexScopesResponse {
    pub scopes: Vec<IndexScope>,
}

#[derive(Debug, Serialize)]
pub struct DeleteScopeResponse {
    pub ok: bool,
    pub deleted_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub scope_id: String,
    pub path: String,
    pub match_reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResultItem>,
}

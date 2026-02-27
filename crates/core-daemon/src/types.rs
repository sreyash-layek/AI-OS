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

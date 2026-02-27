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

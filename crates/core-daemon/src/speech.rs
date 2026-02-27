use crate::types::EventEnvelope;
use tokio::time::{sleep, Duration};

pub async fn run_mock_speech(request_id: String, text: String, voice: String) -> EventEnvelope {
    // Simulate duration based on text length (more realistic than fixed 2s)
    let chars = text.chars().count() as u64;
    let millis = (chars * 35).clamp(1200, 9000);
    sleep(Duration::from_millis(millis)).await;

    EventEnvelope {
        event: "speech_stopped".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "request_id": request_id,
          "reason": "mock_complete",
          "provider": "mock",
          "voice": voice,
          "duration_ms": millis
        }),
    }
}

pub async fn run_system_stub(request_id: String, voice: String) -> EventEnvelope {
    // Placeholder for native adapters in next step
    sleep(Duration::from_millis(600)).await;

    EventEnvelope {
        event: "speech_stopped".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "request_id": request_id,
          "reason": "system_stub_complete",
          "provider": "system",
          "voice": voice
        }),
    }
}

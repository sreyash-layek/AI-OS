use crate::types::{EventEnvelope, VoiceSettings};
use std::process::Command;
use tokio::time::{sleep, Duration};

pub fn detect_effective_provider(settings: &VoiceSettings) -> (String, bool, String) {
    match settings.provider.as_str() {
        "system" => {
            #[cfg(target_os = "linux")]
            {
                let has_spd = Command::new("sh")
                    .arg("-lc")
                    .arg("command -v spd-say >/dev/null 2>&1")
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                if has_spd {
                    ("system".to_string(), true, "spd-say detected".to_string())
                } else {
                    (
                        "mock".to_string(),
                        false,
                        "system requested, but spd-say not found; using mock".to_string(),
                    )
                }
            }

            #[cfg(not(target_os = "linux"))]
            {
                (
                    "system".to_string(),
                    false,
                    "native system adapter not wired yet on this OS; using stub".to_string(),
                )
            }
        }
        _ => ("mock".to_string(), true, "mock provider active".to_string()),
    }
}

pub async fn run_mock_speech(request_id: String, text: String, voice: String) -> EventEnvelope {
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

#[cfg(target_os = "linux")]
pub async fn run_linux_system_speech(request_id: String, text: String, voice: String) -> EventEnvelope {
    let status = Command::new("spd-say").arg(text).status();

    let (reason, ok) = match status {
        Ok(s) if s.success() => ("linux_spd_say_complete", true),
        Ok(_) => ("linux_spd_say_failed", false),
        Err(_) => ("linux_spd_say_missing", false),
    };

    EventEnvelope {
        event: "speech_stopped".to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        source: "core-daemon",
        data: serde_json::json!({
          "request_id": request_id,
          "reason": reason,
          "ok": ok,
          "provider": "system",
          "voice": voice
        }),
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn run_system_stub(request_id: String, voice: String) -> EventEnvelope {
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

use crate::types::{EventEnvelope, VoiceSettings};
use std::process::Command;
use tokio::time::{sleep, Duration};

pub fn detect_effective_provider(settings: &VoiceSettings) -> (String, bool, String) {
    match settings.provider.as_str() {
        "system" => detect_system_provider(),
        _ => ("mock".to_string(), true, "mock provider active".to_string()),
    }
}

fn detect_system_provider() -> (String, bool, String) {
    #[cfg(target_os = "linux")]
    {
        return detect_linux_provider_with(command_exists_linux);
    }

    #[cfg(target_os = "macos")]
    {
        let has_say = command_exists("say");
        if has_say {
            return ("system".to_string(), true, "macOS provider: say".to_string());
        }
        return (
            "mock".to_string(),
            false,
            "system requested, but macOS `say` not available; using mock".to_string(),
        );
    }

    #[cfg(target_os = "windows")]
    {
        let has_powershell = command_exists("powershell");
        if has_powershell {
            return (
                "system".to_string(),
                true,
                "windows provider: powershell SpeechSynthesizer".to_string(),
            );
        }
        return (
            "mock".to_string(),
            false,
            "system requested, but powershell not found; using mock".to_string(),
        );
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        (
            "mock".to_string(),
            false,
            "system provider unsupported on this OS; using mock".to_string(),
        )
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_provider_with<F>(has_cmd: F) -> (String, bool, String)
where
    F: Fn(&str) -> bool,
{
    if has_cmd("spd-say") {
        ("system".to_string(), true, "linux provider: spd-say".to_string())
    } else {
        (
            "mock".to_string(),
            false,
            "system requested, but spd-say not found; using mock".to_string(),
        )
    }
}

#[cfg(target_os = "linux")]
fn command_exists_linux(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {} >/dev/null 2>&1", cmd))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    #[cfg(target_os = "windows")]
    {
        return Command::new("where")
            .arg(cmd)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
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

pub async fn run_system_speech(request_id: String, text: String, voice: String) -> EventEnvelope {
    let result = run_system_command(&text, &voice);
    let (reason, ok): (String, bool) = match result {
        Ok(()) => ("system_complete".to_string(), true),
        Err(err) => (err, false),
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

fn run_system_command(text: &str, voice: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("spd-say").arg(text).status();
        return match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("linux_spd_say_failed".to_string()),
            Err(_) => Err("linux_spd_say_missing".to_string()),
        };
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("say").arg("-v").arg(voice).arg(text).status();
        return match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("macos_say_failed".to_string()),
            Err(_) => Err("macos_say_missing".to_string()),
        };
    }

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; try {{$s.SelectVoice('{}')}} catch {{}}; $s.Speak('{}')",
            escape_ps(voice),
            escape_ps(text)
        );

        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(script)
            .status();

        return match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err("windows_sapi_failed".to_string()),
            Err(_) => Err("windows_powershell_missing".to_string()),
        };
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (text, voice);
        Err("system_provider_unsupported_os".to_string())
    }
}

#[cfg(target_os = "windows")]
fn escape_ps(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_mock_provider_when_configured_mock() {
        let settings = VoiceSettings {
            provider: "mock".to_string(),
            auto_speak: false,
            default_voice: "system-default".to_string(),
        };

        let (effective, available, detail) = detect_effective_provider(&settings);
        assert_eq!(effective, "mock");
        assert!(available);
        assert!(detail.contains("mock"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_provider_detects_system_when_command_exists() {
        let (effective, available, _detail) = detect_linux_provider_with(|_| true);
        assert_eq!(effective, "system");
        assert!(available);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_provider_falls_back_when_command_missing() {
        let (effective, available, detail) = detect_linux_provider_with(|_| false);
        assert_eq!(effective, "mock");
        assert!(!available);
        assert!(detail.contains("spd-say"));
    }

    #[tokio::test]
    async fn mock_speech_emits_stopped_event_shape() {
        let event = run_mock_speech(
            "req-1".to_string(),
            "hello world".to_string(),
            "system-default".to_string(),
        )
        .await;

        assert_eq!(event.event, "speech_stopped");
        assert_eq!(event.source, "core-daemon");
        assert_eq!(event.data["request_id"], "req-1");
        assert_eq!(event.data["provider"], "mock");
        assert_eq!(event.data["voice"], "system-default");
        assert_eq!(event.data["reason"], "mock_complete");
    }

    #[tokio::test]
    async fn system_speech_emits_system_provider_event() {
        let event = run_system_speech(
            "req-2".to_string(),
            "hello system".to_string(),
            "system-default".to_string(),
        )
        .await;

        assert_eq!(event.event, "speech_stopped");
        assert_eq!(event.data["request_id"], "req-2");
        assert_eq!(event.data["provider"], "system");
        assert_eq!(event.data["voice"], "system-default");
        assert!(event.data["reason"].is_string());
    }
}

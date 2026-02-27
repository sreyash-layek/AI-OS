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
    build_system_stop_event(request_id, voice, result)
}

fn build_system_stop_event(
    request_id: String,
    voice: String,
    result: Result<(), String>,
) -> EventEnvelope {
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

#[cfg(target_os = "linux")]
fn run_system_command(text: &str, _voice: &str) -> Result<(), String> {
    run_linux_system_command_with(text, |arg| Command::new("spd-say").arg(arg).status())
}

#[cfg(target_os = "linux")]
fn run_linux_system_command_with<F>(text: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(&str) -> std::io::Result<std::process::ExitStatus>,
{
    match exec(text) {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("linux_spd_say_failed".to_string()),
        Err(_) => Err("linux_spd_say_missing".to_string()),
    }
}

#[cfg(target_os = "macos")]
fn run_system_command(text: &str, voice: &str) -> Result<(), String> {
    let status = Command::new("say").arg("-v").arg(voice).arg(text).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("macos_say_failed".to_string()),
        Err(_) => Err("macos_say_missing".to_string()),
    }
}

#[cfg(target_os = "windows")]
fn run_system_command(text: &str, voice: &str) -> Result<(), String> {
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

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("windows_sapi_failed".to_string()),
        Err(_) => Err("windows_powershell_missing".to_string()),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn run_system_command(_text: &str, _voice: &str) -> Result<(), String> {
    Err("system_provider_unsupported_os".to_string())
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
    fn detect_effective_provider_system_executes_linux_resolution_path() {
        let settings = VoiceSettings {
            provider: "system".to_string(),
            auto_speak: false,
            default_voice: "system-default".to_string(),
        };

        let (effective, _available, detail) = detect_effective_provider(&settings);
        assert!(effective == "system" || effective == "mock");
        assert!(!detail.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_provider_falls_back_when_command_missing() {
        let (effective, available, detail) = detect_linux_provider_with(|_| false);
        assert_eq!(effective, "mock");
        assert!(!available);
        assert!(detail.contains("spd-say"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_command_runner_maps_success() {
        let result = run_linux_system_command_with("hello", |_arg| {
            Command::new("sh").arg("-lc").arg("exit 0").status()
        });
        assert!(result.is_ok());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_command_runner_maps_failure() {
        let result = run_linux_system_command_with("hello", |_arg| {
            Command::new("sh").arg("-lc").arg("exit 3").status()
        });
        assert_eq!(result.unwrap_err(), "linux_spd_say_failed");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_command_runner_maps_missing() {
        let result = run_linux_system_command_with("hello", |_arg| {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
        });
        assert_eq!(result.unwrap_err(), "linux_spd_say_missing");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn command_exists_linux_true_and_false_paths() {
        assert!(command_exists_linux("sh"));
        assert!(!command_exists_linux("definitely-not-a-real-command-xyz"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn run_system_command_is_wired_to_linux_runner() {
        let result = run_system_command("hello", "ignored");
        let ok = result.is_ok();
        let expected_err = matches!(
            result.as_ref().err().map(|e| e.as_str()),
            Some("linux_spd_say_missing") | Some("linux_spd_say_failed")
        );
        assert!(ok || expected_err);
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
    async fn mock_speech_duration_bounds_are_enforced() {
        let short = run_mock_speech(
            "req-short".to_string(),
            "a".to_string(),
            "system-default".to_string(),
        )
        .await;
        assert_eq!(short.data["duration_ms"], 1200);

        let long_text = "x".repeat(1000);
        let long = run_mock_speech(
            "req-long".to_string(),
            long_text,
            "system-default".to_string(),
        )
        .await;
        assert_eq!(long.data["duration_ms"], 9000);
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

    #[test]
    fn build_system_stop_event_maps_success_result() {
        let event = build_system_stop_event(
            "req-success".to_string(),
            "voice-a".to_string(),
            Ok(()),
        );

        assert_eq!(event.data["request_id"], "req-success");
        assert_eq!(event.data["ok"], true);
        assert_eq!(event.data["reason"], "system_complete");
    }

    #[test]
    fn build_system_stop_event_maps_error_result() {
        let event = build_system_stop_event(
            "req-error".to_string(),
            "voice-b".to_string(),
            Err("custom_error".to_string()),
        );

        assert_eq!(event.data["request_id"], "req-error");
        assert_eq!(event.data["ok"], false);
        assert_eq!(event.data["reason"], "custom_error");
    }
}

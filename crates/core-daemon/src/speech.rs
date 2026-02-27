use crate::types::{EventEnvelope, VoiceSettings};
use std::process::{Command, ExitStatus};
use tokio::time::{sleep, Duration};

fn current_os() -> &'static str {
    std::env::consts::OS
}

pub fn detect_effective_provider(settings: &VoiceSettings) -> (String, bool, String) {
    match settings.provider.as_str() {
        "system" => detect_system_provider_for_os(current_os()),
        _ => ("mock".to_string(), true, "mock provider active".to_string()),
    }
}

fn detect_system_provider_for_os(os: &str) -> (String, bool, String) {
    detect_system_provider_with(os, |cmd| command_exists_for_os(os, cmd))
}

fn detect_system_provider_with<F>(os: &str, command_exists: F) -> (String, bool, String)
where
    F: Fn(&str) -> bool,
{
    match os {
        "linux" => {
            if command_exists("spd-say") {
                (
                    "system".to_string(),
                    true,
                    "linux provider: spd-say".to_string(),
                )
            } else {
                (
                    "mock".to_string(),
                    false,
                    "system requested, but spd-say not found; using mock".to_string(),
                )
            }
        }
        "macos" => {
            if command_exists("say") {
                (
                    "system".to_string(),
                    true,
                    "macOS provider: say".to_string(),
                )
            } else {
                (
                    "mock".to_string(),
                    false,
                    "system requested, but macOS `say` not available; using mock".to_string(),
                )
            }
        }
        "windows" => {
            if command_exists("powershell") {
                (
                    "system".to_string(),
                    true,
                    "windows provider: powershell SpeechSynthesizer".to_string(),
                )
            } else {
                (
                    "mock".to_string(),
                    false,
                    "system requested, but powershell not found; using mock".to_string(),
                )
            }
        }
        _ => (
            "mock".to_string(),
            false,
            "system provider unsupported on this OS; using mock".to_string(),
        ),
    }
}

fn command_exists_for_os(os: &str, cmd: &str) -> bool {
    match os {
        "windows" => command_exists_windows(cmd),
        _ => command_exists_shell(cmd),
    }
}

fn status_ok(status: std::io::Result<ExitStatus>) -> bool {
    status.map(|s| s.success()).unwrap_or(false)
}

fn command_exists_windows(cmd: &str) -> bool {
    status_ok(Command::new("where").arg(cmd).status())
}

fn command_exists_shell(cmd: &str) -> bool {
    status_ok(
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {} >/dev/null 2>&1", cmd))
            .status(),
    )
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
    let result = run_system_command_for_os(current_os(), &text, &voice);
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

fn run_system_command_for_os(os: &str, text: &str, voice: &str) -> Result<(), String> {
    match os {
        "linux" => {
            run_linux_system_command_with(text, |arg| Command::new("spd-say").arg(arg).status())
        }
        "macos" => run_macos_system_command_with(text, voice, |t, v| {
            Command::new("say").arg("-v").arg(v).arg(t).status()
        }),
        "windows" => run_windows_system_command_with(text, voice, |script| {
            Command::new("powershell")
                .arg("-NoProfile")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(script)
                .status()
        }),
        _ => Err("system_provider_unsupported_os".to_string()),
    }
}

fn run_linux_system_command_with<F>(text: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(&str) -> std::io::Result<ExitStatus>,
{
    match exec(text) {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("linux_spd_say_failed".to_string()),
        Err(_) => Err("linux_spd_say_missing".to_string()),
    }
}

fn run_macos_system_command_with<F>(text: &str, voice: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(&str, &str) -> std::io::Result<ExitStatus>,
{
    match exec(text, voice) {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("macos_say_failed".to_string()),
        Err(_) => Err("macos_say_missing".to_string()),
    }
}

fn run_windows_system_command_with<F>(text: &str, voice: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(String) -> std::io::Result<ExitStatus>,
{
    let script = format!(
        "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; try {{$s.SelectVoice('{}')}} catch {{}}; $s.Speak('{}')",
        escape_ps(voice),
        escape_ps(text)
    );

    match exec(script) {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("windows_sapi_failed".to_string()),
        Err(_) => Err("windows_powershell_missing".to_string()),
    }
}

fn escape_ps(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(provider: &str) -> VoiceSettings {
        VoiceSettings {
            provider: provider.to_string(),
            auto_speak: false,
            default_voice: "system-default".to_string(),
        }
    }

    #[test]
    fn detect_mock_provider_when_configured_mock() {
        let (effective, available, detail) = detect_effective_provider(&settings("mock"));
        assert_eq!(effective, "mock");
        assert!(available);
        assert!(detail.contains("mock"));
    }

    #[test]
    fn detect_provider_linux_paths() {
        let yes = detect_system_provider_for_os("linux");
        assert!(yes.0 == "system" || yes.0 == "mock");

        let no = detect_system_provider_for_os("haiku");
        assert_eq!(no.0, "mock");
    }

    #[test]
    fn detect_provider_all_os_paths_with_injected_presence() {
        let linux_yes = detect_system_provider_with("linux", |cmd| cmd == "spd-say");
        let linux_no = detect_system_provider_with("linux", |_cmd| false);
        assert_eq!(linux_yes.0, "system");
        assert_eq!(linux_no.0, "mock");

        let mac_yes = detect_system_provider_with("macos", |cmd| cmd == "say");
        let mac_no = detect_system_provider_with("macos", |_cmd| false);
        assert_eq!(mac_yes.0, "system");
        assert_eq!(mac_no.0, "mock");

        let win_yes = detect_system_provider_with("windows", |cmd| cmd == "powershell");
        let win_no = detect_system_provider_with("windows", |_cmd| false);
        assert_eq!(win_yes.0, "system");
        assert_eq!(win_no.0, "mock");

        let unknown = detect_system_provider_with("unknown", |_cmd| true);
        assert_eq!(unknown.0, "mock");
        assert!(!unknown.1);
    }

    #[test]
    fn command_exists_functions_are_callable() {
        let _ = command_exists_for_os("windows", "definitely-not-real");
        let _ = command_exists_for_os("linux", "sh");
        let _ = command_exists_windows("definitely-not-real");
        let _ = command_exists_shell("sh");
    }

    #[test]
    fn status_ok_maps_ok_and_err_paths() {
        assert!(status_ok(
            Command::new("sh").arg("-lc").arg("exit 0").status()
        ));
        assert!(!status_ok(
            Command::new("sh").arg("-lc").arg("exit 3").status()
        ));
        assert!(!status_ok(Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing",
        ))));
    }

    #[test]
    fn linux_runner_maps_all_outcomes() {
        assert!(run_linux_system_command_with("x", |_t| {
            Command::new("sh").arg("-lc").arg("exit 0").status()
        })
        .is_ok());

        assert_eq!(
            run_linux_system_command_with("x", |_t| {
                Command::new("sh").arg("-lc").arg("exit 2").status()
            })
            .unwrap_err(),
            "linux_spd_say_failed"
        );

        assert_eq!(
            run_linux_system_command_with("x", |_t| {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
            })
            .unwrap_err(),
            "linux_spd_say_missing"
        );
    }

    #[test]
    fn macos_runner_maps_all_outcomes() {
        assert!(run_macos_system_command_with("x", "v", |_t, _v| {
            Command::new("sh").arg("-lc").arg("exit 0").status()
        })
        .is_ok());

        assert_eq!(
            run_macos_system_command_with("x", "v", |_t, _v| {
                Command::new("sh").arg("-lc").arg("exit 2").status()
            })
            .unwrap_err(),
            "macos_say_failed"
        );

        assert_eq!(
            run_macos_system_command_with("x", "v", |_t, _v| {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
            })
            .unwrap_err(),
            "macos_say_missing"
        );
    }

    #[test]
    fn windows_runner_maps_all_outcomes_and_escapes() {
        assert_eq!(escape_ps("a'b"), "a''b");

        assert!(run_windows_system_command_with("x", "v", |_script| {
            Command::new("sh").arg("-lc").arg("exit 0").status()
        })
        .is_ok());

        assert_eq!(
            run_windows_system_command_with("x", "v", |_script| {
                Command::new("sh").arg("-lc").arg("exit 2").status()
            })
            .unwrap_err(),
            "windows_sapi_failed"
        );

        assert_eq!(
            run_windows_system_command_with("x", "v", |_script| {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
            })
            .unwrap_err(),
            "windows_powershell_missing"
        );
    }

    #[test]
    fn os_dispatch_for_system_command_covers_all_paths() {
        let _ = run_system_command_for_os("linux", "x", "v");
        let _ = run_system_command_for_os("macos", "x", "v");
        let _ = run_system_command_for_os("windows", "x", "v");
        assert_eq!(
            run_system_command_for_os("unknown", "x", "v").unwrap_err(),
            "system_provider_unsupported_os"
        );
    }

    #[test]
    fn current_os_is_non_empty() {
        assert!(!current_os().is_empty());
    }

    #[tokio::test]
    async fn mock_speech_emits_shape_and_duration_bounds() {
        let short = run_mock_speech(
            "req-short".to_string(),
            "a".to_string(),
            "system-default".to_string(),
        )
        .await;
        assert_eq!(short.event, "speech_stopped");
        assert_eq!(short.data["duration_ms"], 1200);

        let long = run_mock_speech(
            "req-long".to_string(),
            "x".repeat(1000),
            "system-default".to_string(),
        )
        .await;
        assert_eq!(long.data["duration_ms"], 9000);
        assert_eq!(long.data["provider"], "mock");
    }

    #[tokio::test]
    async fn system_speech_and_event_builder_cover_result_branches() {
        let evt = run_system_speech(
            "req-2".to_string(),
            "hello system".to_string(),
            "system-default".to_string(),
        )
        .await;
        assert_eq!(evt.event, "speech_stopped");
        assert_eq!(evt.data["provider"], "system");

        let ok_evt = build_system_stop_event("id1".to_string(), "v".to_string(), Ok(()));
        assert_eq!(ok_evt.data["ok"], true);
        assert_eq!(ok_evt.data["reason"], "system_complete");

        let err_evt = build_system_stop_event(
            "id2".to_string(),
            "v".to_string(),
            Err("custom_error".to_string()),
        );
        assert_eq!(err_evt.data["ok"], false);
        assert_eq!(err_evt.data["reason"], "custom_error");
    }
}

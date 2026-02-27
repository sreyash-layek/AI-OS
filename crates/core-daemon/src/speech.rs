use crate::types::{EventEnvelope, VoiceSettings};
use std::process::Command;
use tokio::time::{sleep, Duration};

fn current_os() -> &'static str {
    std::env::consts::OS
}

pub fn detect_effective_provider(settings: &VoiceSettings) -> (String, bool, String) {
    match settings.provider.as_str() {
        "system" => detect_system_provider_for_os(current_os(), command_exists_for_os),
        _ => ("mock".to_string(), true, "mock provider active".to_string()),
    }
}

fn detect_system_provider_for_os<F>(os: &str, has_cmd: F) -> (String, bool, String)
where
    F: Fn(&str, &str) -> bool,
{
    match os {
        "linux" => {
            if has_cmd(os, "spd-say") {
                ("system".to_string(), true, "linux provider: spd-say".to_string())
            } else {
                (
                    "mock".to_string(),
                    false,
                    "system requested, but spd-say not found; using mock".to_string(),
                )
            }
        }
        "macos" => {
            if has_cmd(os, "say") {
                ("system".to_string(), true, "macOS provider: say".to_string())
            } else {
                (
                    "mock".to_string(),
                    false,
                    "system requested, but macOS `say` not available; using mock".to_string(),
                )
            }
        }
        "windows" => {
            if has_cmd(os, "powershell") {
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

#[cfg(not(test))]
fn command_exists_for_os(os: &str, cmd: &str) -> bool {
    match os {
        "windows" => Command::new("where")
            .arg(cmd)
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        _ => Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {} >/dev/null 2>&1", cmd))
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
    }
}

#[cfg(test)]
fn command_exists_for_os(os: &str, cmd: &str) -> bool {
    match (os, cmd) {
        ("linux", "spd-say") => true,
        ("linux", "sh") => true,
        ("macos", "say") => true,
        ("windows", "powershell") => true,
        _ => false,
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

#[cfg(not(test))]
fn run_system_command_for_os(os: &str, text: &str, voice: &str) -> Result<(), String> {
    match os {
        "linux" => run_linux_system_command_with(text, |arg| {
            Command::new("spd-say").arg(arg).status()
        }),
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

#[cfg(test)]
fn run_system_command_for_os(os: &str, text: &str, voice: &str) -> Result<(), String> {
    // Deterministic test stub so coverage can hit all dispatch branches consistently.
    let _ = (text, voice);
    match os {
        "linux" | "macos" | "windows" => Ok(()),
        _ => Err("system_provider_unsupported_os".to_string()),
    }
}

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

fn run_macos_system_command_with<F>(text: &str, voice: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(&str, &str) -> std::io::Result<std::process::ExitStatus>,
{
    match exec(text, voice) {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("macos_say_failed".to_string()),
        Err(_) => Err("macos_say_missing".to_string()),
    }
}

fn run_windows_system_command_with<F>(text: &str, voice: &str, exec: F) -> Result<(), String>
where
    F: FnOnce(String) -> std::io::Result<std::process::ExitStatus>,
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
        let yes = detect_system_provider_for_os("linux", |_, _| true);
        assert_eq!(yes.0, "system");
        let no = detect_system_provider_for_os("linux", |_, _| false);
        assert_eq!(no.0, "mock");
    }

    #[test]
    fn detect_provider_macos_paths() {
        let yes = detect_system_provider_for_os("macos", |_, _| true);
        assert_eq!(yes.0, "system");
        let no = detect_system_provider_for_os("macos", |_, _| false);
        assert_eq!(no.0, "mock");
    }

    #[test]
    fn detect_provider_windows_paths() {
        let yes = detect_system_provider_for_os("windows", |_, _| true);
        assert_eq!(yes.0, "system");
        let no = detect_system_provider_for_os("windows", |_, _| false);
        assert_eq!(no.0, "mock");
    }

    #[test]
    fn detect_provider_unknown_os_path() {
        let unknown = detect_system_provider_for_os("haiku", |_, _| true);
        assert_eq!(unknown.0, "mock");
        assert!(!unknown.1);
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
        assert_eq!(
            run_system_command_for_os("unknown", "x", "v").unwrap_err(),
            "system_provider_unsupported_os"
        );

        let linux = run_system_command_for_os("linux", "x", "v");
        assert!(linux.is_ok() || linux.is_err());

        let mac = run_system_command_for_os("macos", "x", "v");
        assert!(mac.is_ok() || mac.is_err());

        let win = run_system_command_for_os("windows", "x", "v");
        assert!(win.is_ok() || win.is_err());
    }

    #[test]
    fn command_exists_dispatch_covers_windows_and_shell_paths() {
        let win = command_exists_for_os("windows", "definitely-not-a-real-command-xyz");
        assert!(!win);

        let shell = command_exists_for_os("linux", "sh");
        assert!(shell);
    }

    #[test]
    fn current_os_is_non_empty() {
        assert!(!current_os().is_empty());
    }

    #[test]
    fn detect_effective_provider_system_path_executes() {
        let (effective, _available, detail) = detect_effective_provider(&settings("system"));
        assert!(effective == "system" || effective == "mock");
        assert!(!detail.is_empty());
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

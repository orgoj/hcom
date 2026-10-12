//! Orca terminal backend: open an agent in an Orca terminal and close it by
//! its opaque handle.

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};

use super::{
    PaneCloseResult, TERMINAL_CLOSE_TIMEOUT, TerminalCommandContext, format_close_command,
    get_launcher_env, ps_quote, shell_quote, validate_terminal_launch_output, which_bin,
};

const ORCA_INTERACTIVE_AGENT_CAPABILITY: &str = "terminal.create-interactive-agent.v1";
const ORCA_FOLDER_WORKSPACE_CAPABILITY: &str = "terminal.create-folder-workspace.v1";

/// Parse Orca's terminal.create response without accepting unrelated JSON
/// envelopes. Terminal handles are opaque and scoped to one Orca runtime.
fn extract_orca_terminal_handle(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/result/terminal/handle")
        .and_then(|v| v.as_str())
        .filter(|handle| !handle.is_empty())
        .map(str::to_string)
}

fn parse_orca_terminal_handle(captured: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(captured.trim())
        .context("Orca returned malformed JSON while creating the terminal")?;
    if value.get("ok").and_then(|v| v.as_bool()) != Some(true)
        || !value
            .get("id")
            .and_then(|v| v.as_str())
            .is_some_and(|id| !id.is_empty())
        || !value
            .pointer("/result/terminal/worktreeId")
            .and_then(|v| v.as_str())
            .is_some_and(|worktree_id| !worktree_id.is_empty())
    {
        bail!("Orca returned an unexpected terminal-create response envelope");
    }
    extract_orca_terminal_handle(&value).ok_or_else(|| {
        anyhow!("Orca created no addressable terminal (missing result.terminal.handle)")
    })
}

fn orca_agent_kind(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" => Some("claude"),
        "codex" => Some("codex"),
        _ => None,
    }
}

fn orca_remote_selector() -> Option<&'static str> {
    orca_remote_selector_from(|key| std::env::var(key).ok())
}

fn orca_remote_selector_from(get: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    [
        "ORCA_ENVIRONMENT",
        "ORCA_PAIRING_CODE",
        "ORCA_REMOTE_PAIRING",
    ]
    .into_iter()
    .find(|key| get(key).is_some_and(|value| !value.is_empty()))
}

pub(super) fn orca_binary() -> Option<String> {
    orca_binary_from(cfg!(target_os = "linux"), |binary| {
        which_bin(binary).is_some()
    })
}

fn orca_binary_from(linux: bool, exists: impl Fn(&str) -> bool) -> Option<String> {
    if exists("orca") {
        return Some("orca".to_string());
    }
    if linux && exists("orca-ide") {
        return Some("orca-ide".to_string());
    }
    None
}

fn parse_orca_runtime_available(captured: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(captured.trim()) else {
        return false;
    };
    value.get("ok").and_then(|v| v.as_bool()) == Some(true)
        && value
            .pointer("/result/runtime/reachable")
            .and_then(|v| v.as_bool())
            == Some(true)
        && value
            .pointer("/result/runtime/capabilities")
            .and_then(|v| v.as_array())
            .is_some_and(|capabilities| {
                [
                    ORCA_INTERACTIVE_AGENT_CAPABILITY,
                    ORCA_FOLDER_WORKSPACE_CAPABILITY,
                ]
                .into_iter()
                .all(|required| {
                    capabilities
                        .iter()
                        .any(|capability| capability.as_str() == Some(required))
                })
            })
}

pub(super) fn orca_runtime_is_available(binary: &str) -> bool {
    if orca_remote_selector().is_some() {
        return false;
    }
    let launcher_env = get_launcher_env();
    let mut child = match Command::new(binary)
        .args(["status", "--json"])
        .env_clear()
        .envs(&launcher_env)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                return false;
            }
        }
    };
    if !status.success() {
        return false;
    }
    let mut captured = String::new();
    child.stdout.take().is_some_and(|mut stdout| {
        stdout.read_to_string(&mut captured).is_ok() && parse_orca_runtime_available(&captured)
    })
}

fn orca_runner_command(script: &str, windows: bool) -> String {
    if windows {
        format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {}",
            ps_quote(script)
        )
    } else {
        format!("bash {}", shell_quote(script))
    }
}

fn orca_create_argv(
    binary: String,
    canonical_cwd: &str,
    script: &str,
    instance_name: &str,
    tool: &str,
    windows: bool,
) -> Vec<String> {
    let mut argv = vec![
        binary,
        "terminal".to_string(),
        "create".to_string(),
        "--worktree".to_string(),
        format!("path:{canonical_cwd}"),
        "--ensure-folder-workspace".to_string(),
        "--command".to_string(),
        orca_runner_command(script, windows),
        "--title".to_string(),
        instance_name.to_string(),
    ];
    if let Some(agent) = orca_agent_kind(tool) {
        argv.extend(["--interactive-agent".to_string(), agent.to_string()]);
    }
    argv.push("--json".to_string());
    argv
}

pub(super) fn launch_orca_terminal(
    ctx: TerminalCommandContext<'_>,
    inside_ai_tool: bool,
) -> Result<(bool, String)> {
    if let Some(selector) = orca_remote_selector() {
        bail!(
            "Orca terminal support is local-only; unset {selector} before launching with terminal=orca"
        );
    }

    let binary = orca_binary().ok_or_else(|| {
        anyhow!(
            "Orca CLI was not found; install `orca` (or the Linux `orca-ide` package) and start the Orca desktop app or local `orca serve` runtime"
        )
    })?;
    let cwd = if ctx.cwd.is_empty() {
        std::env::current_dir().context("Failed to resolve the Orca workspace directory")?
    } else {
        PathBuf::from(ctx.cwd)
    };
    let canonical_cwd = cwd
        .canonicalize()
        .with_context(|| format!("Failed to resolve Orca workspace path {}", cwd.display()))?;
    let canonical_cwd = canonical_cwd
        .to_str()
        .ok_or_else(|| anyhow!("Orca workspace path is not valid UTF-8"))?;

    let argv = orca_create_argv(
        binary,
        canonical_cwd,
        ctx.script,
        ctx.instance_name,
        ctx.tool,
        cfg!(windows),
    );

    let launcher_env = get_launcher_env();
    let output = Command::new(&argv[0])
        .args(&argv[1..])
        .env_clear()
        .envs(&launcher_env)
        .output()
        .context(
            "Failed to run the Orca CLI; start the Orca desktop app or local `orca serve` runtime",
        )?;
    if !output.status.success()
        && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        && let Some(code) = value.pointer("/error/code").and_then(|v| v.as_str())
    {
        let detail = value
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or(code);
        match code {
            "incompatible_runtime" => bail!(
                "Installed Orca runtime lacks required capabilities {ORCA_INTERACTIVE_AGENT_CAPABILITY} and {ORCA_FOLDER_WORKSPACE_CAPABILITY}; update Orca and retry: {detail}"
            ),
            "runtime_unavailable" => bail!(
                "Local Orca runtime is unavailable; start the Orca desktop app or `orca serve`, then retry: {detail}"
            ),
            _ => bail!("Orca terminal creation failed ({code}): {detail}"),
        }
    }
    validate_terminal_launch_output(&argv, &output, inside_ai_tool)?;
    let captured = String::from_utf8(output.stdout)
        .context("Orca returned non-UTF-8 JSON while creating the terminal")?;
    let parsed = serde_json::from_str::<serde_json::Value>(captured.trim());
    let recoverable_handle = parsed.as_ref().ok().and_then(extract_orca_terminal_handle);
    let handle = match parse_orca_terminal_handle(&captured) {
        Ok(handle) => handle,
        Err(err) => {
            if let Some(handle) = recoverable_handle {
                let _ = close_orca_terminal(&handle);
            }
            return Err(err);
        }
    };
    Ok((true, handle))
}

fn parse_orca_close_receipt(captured: &str, expected_handle: &str) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(captured.trim())
        .context("Orca returned malformed JSON while closing the terminal")?;
    if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        bail!("Orca close response is not successful");
    }
    let close = value
        .pointer("/result/close")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("Orca close response is missing result.close"))?;
    let handle = close
        .get("handle")
        .and_then(|v| v.as_str())
        .filter(|handle| !handle.is_empty())
        .ok_or_else(|| anyhow!("Orca close response is missing the terminal handle"))?;
    if handle != expected_handle {
        bail!("Orca close response addressed a different terminal handle");
    }
    if !close
        .get("ptyKilled")
        .is_some_and(|value| value.is_boolean())
    {
        bail!("Orca close response is missing the PTY stop receipt");
    }
    if let Some(verdict) = close.get("ptyStopVerdict").and_then(|v| v.as_str()) {
        bail!("Orca could not verify terminal cleanup ({verdict})");
    }
    Ok(())
}

pub(super) fn close_orca_terminal(handle: &str) -> PaneCloseResult {
    let Some(binary) = orca_binary() else {
        eprintln!("Failed to close Orca terminal: Orca CLI was not found");
        return PaneCloseResult {
            closed: false,
            retry_command: None,
        };
    };
    let argv = vec![
        binary,
        "terminal".to_string(),
        "close".to_string(),
        "--terminal".to_string(),
        handle.to_string(),
        "--json".to_string(),
    ];
    let retry_command = format_close_command(&argv);
    let failed = || PaneCloseResult {
        closed: false,
        retry_command: Some(retry_command.clone()),
    };
    let launcher_env = get_launcher_env();
    let mut child = match Command::new(&argv[0])
        .args(&argv[1..])
        .env_clear()
        .envs(&launcher_env)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to close Orca terminal: {err}");
            return failed();
        }
    };

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < TERMINAL_CLOSE_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                eprintln!("Timed out closing Orca terminal {handle}");
                return failed();
            }
            Err(err) => {
                let _ = child.kill();
                eprintln!("Failed waiting for Orca terminal close: {err}");
                return failed();
            }
        }
    };
    if !status.success() {
        eprintln!("Failed to close Orca terminal {handle}: {status}");
        return failed();
    }
    let mut captured = String::new();
    if child
        .stdout
        .take()
        .is_none_or(|mut stdout| stdout.read_to_string(&mut captured).is_err())
    {
        eprintln!("Failed to read Orca terminal close receipt");
        return failed();
    }
    if let Err(err) = parse_orca_close_receipt(&captured, handle) {
        eprintln!("Failed to validate Orca terminal close receipt: {err}");
        return failed();
    }
    PaneCloseResult {
        closed: true,
        retry_command: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_orca_terminal_handle_requires_exact_nonempty_path() {
        assert_eq!(
            parse_orca_terminal_handle(
                r#"{"id":"request-42","ok":true,"result":{"terminal":{"handle":"term:runtime:42","worktreeId":"workspace:42"}}}"#
            )
            .unwrap(),
            "term:runtime:42"
        );
        assert!(parse_orca_terminal_handle(r#"{"result":{"handle":"wrong"}}"#).is_err());
        assert!(
            parse_orca_terminal_handle(
                r#"{"id":"request-42","ok":true,"result":{"terminal":{"handle":"","worktreeId":"workspace:42"}}}"#
            )
            .is_err()
        );
        assert!(
            parse_orca_terminal_handle(
                r#"{"id":"request-42","ok":false,"result":{"terminal":{"handle":"term:wrong","worktreeId":"workspace:42"}}}"#
            )
            .is_err()
        );
        assert!(
            parse_orca_terminal_handle(
                r#"{"id":"request-42","ok":true,"result":{"terminal":{"handle":"term:wrong"}}}"#
            )
            .is_err()
        );
        assert!(parse_orca_terminal_handle("not-json").is_err());
    }

    #[test]
    fn orca_agent_kind_never_mislabels_tools() {
        assert_eq!(orca_agent_kind("claude"), Some("claude"));
        assert_eq!(orca_agent_kind("codex"), Some("codex"));
        assert_eq!(orca_agent_kind("gemini"), None);
        assert_eq!(orca_agent_kind("hcom"), None);
    }

    #[test]
    fn orca_remote_selectors_fail_closed_only_when_nonempty() {
        assert_eq!(
            orca_remote_selector_from(|key| (key == "ORCA_PAIRING_CODE").then(|| "code".into())),
            Some("ORCA_PAIRING_CODE")
        );
        assert_eq!(orca_remote_selector_from(|_| Some(String::new())), None);
    }

    #[test]
    fn orca_runtime_availability_requires_reachability_and_capability() {
        let available = format!(
            r#"{{"ok":true,"result":{{"runtime":{{"reachable":true,"capabilities":["{}","{}"]}}}}}}"#,
            ORCA_INTERACTIVE_AGENT_CAPABILITY, ORCA_FOLDER_WORKSPACE_CAPABILITY
        );
        assert!(parse_orca_runtime_available(&available));
        assert!(!parse_orca_runtime_available(
            r#"{"ok":true,"result":{"runtime":{"reachable":false,"capabilities":["terminal.create-interactive-agent.v1"]}}}"#
        ));
        assert!(!parse_orca_runtime_available(
            r#"{"ok":true,"result":{"runtime":{"reachable":true,"capabilities":[]}}}"#
        ));
        assert!(!parse_orca_runtime_available(
            r#"{"ok":true,"result":{"runtime":{"reachable":true,"capabilities":["terminal.create-interactive-agent.v1"]}}}"#
        ));
        assert!(!parse_orca_runtime_available(
            r#"{"ok":true,"result":{"runtime":{"reachable":true,"capabilities":["terminal.create-folder-workspace.v1"]}}}"#
        ));
        assert!(!parse_orca_runtime_available("not-json"));
    }

    #[test]
    fn orca_binary_prefers_public_cli_and_supports_linux_package_name() {
        assert_eq!(
            orca_binary_from(true, |binary| binary == "orca" || binary == "orca-ide"),
            Some("orca".into())
        );
        assert_eq!(
            orca_binary_from(true, |binary| binary == "orca-ide"),
            Some("orca-ide".into())
        );
        assert_eq!(orca_binary_from(false, |binary| binary == "orca-ide"), None);
    }

    #[test]
    fn orca_preset_is_cross_platform_and_uses_native_adapter() {
        let preset = crate::shared::terminal_presets::get_terminal_preset("orca").unwrap();
        assert_eq!(preset.binary, Some("orca"));
        assert_eq!(preset.platforms, &["Darwin", "Linux", "Windows"]);
        assert_eq!(preset.open.select(false).unwrap(), &["orca"]);
    }

    #[test]
    fn orca_create_argv_keeps_values_in_single_arguments() {
        let argv = orca_create_argv(
            "orca".into(),
            "/tmp/project with space/žluťoučký",
            "/tmp/runner with space.sh",
            "agent žluťoučký",
            "codex",
            false,
        );
        assert_eq!(
            argv,
            vec![
                "orca",
                "terminal",
                "create",
                "--worktree",
                "path:/tmp/project with space/žluťoučký",
                "--ensure-folder-workspace",
                "--command",
                "bash '/tmp/runner with space.sh'",
                "--title",
                "agent žluťoučký",
                "--interactive-agent",
                "codex",
                "--json",
            ]
        );
    }

    #[test]
    fn orca_create_argv_uses_native_windows_runner_and_omits_unknown_agent_hint() {
        let argv = orca_create_argv(
            "orca.exe".into(),
            r"C:\project with space",
            r"C:\runner with space.ps1",
            "agent",
            "gemini",
            true,
        );
        assert_eq!(argv[4], r"path:C:\project with space");
        assert_eq!(
            argv[7],
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\\runner with space.ps1'"
        );
        assert!(!argv.iter().any(|arg| arg == "--interactive-agent"));
    }

    #[test]
    fn orca_close_receipt_requires_matching_handle_and_stop_evidence() {
        parse_orca_close_receipt(
            r#"{"ok":true,"result":{"close":{"handle":"term:42","tabId":"tab:1","ptyKilled":true}}}"#,
            "term:42",
        )
        .unwrap();
        assert!(
            parse_orca_close_receipt(
                r#"{"ok":false,"result":{"close":{"handle":"term:42","ptyKilled":true}}}"#,
                "term:42",
            )
            .is_err()
        );
        assert!(
            parse_orca_close_receipt(
                r#"{"result":{"close":{"handle":"term:42","ptyKilled":true}}}"#,
                "term:42",
            )
            .is_err()
        );
        assert!(
            parse_orca_close_receipt(
                r#"{"ok":true,"result":{"close":{"handle":"term:other","ptyKilled":true}}}"#,
                "term:42",
            )
            .is_err()
        );
        assert!(
            parse_orca_close_receipt(
                r#"{"ok":true,"result":{"close":{"handle":"term:42","ptyKilled":false,"ptyStopVerdict":"unverifiable"}}}"#,
                "term:42",
            )
            .is_err()
        );
        assert!(
            parse_orca_close_receipt(
                r#"{"ok":true,"result":{"close":{"handle":"term:42"}}}"#,
                "term:42",
            )
            .is_err()
        );
    }
}

//! Auto-update checker — checks latest release via git ls-remote once daily.
//! Uses git ls-remote instead of the GitHub REST API to avoid rate limits.

use crate::paths::{FLAGS_DIR, atomic_write, hcom_path};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const CHECK_INTERVAL: Duration = Duration::from_secs(86400); // 24 hours
const UNIX_INSTALL_CMD: &str =
    "curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh";
const WINDOWS_INSTALL_CMD: &str = "powershell -NoProfile -ExecutionPolicy Bypass -Command \"irm https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.ps1 | iex\"";

pub(crate) fn flag_path() -> PathBuf {
    hcom_path(&[FLAGS_DIR, "update_check"])
}

/// Parse upstream and fork versions into a comparable tuple.
fn parse_version(v: &str) -> Option<(u32, u32, u32, u32)> {
    let parts: Vec<&str> = v.trim().trim_start_matches('v').split('.').collect();
    if parts.len() >= 3 {
        let (patch, fork_revision) = match parts[2].split_once('-') {
            Some((patch, "orgoj")) => (
                patch,
                parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
            ),
            Some((patch, _)) => (patch, 0),
            None => (parts[2], 0),
        };
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            patch.parse().ok()?,
            fork_revision,
        ))
    } else {
        None
    }
}

/// Spawn a detached background process to fetch latest version and write the cache file.
/// Returns immediately — result shows up on next command.
///
/// No-op on Windows: the script below is POSIX (`sh -c`, `awk`, `git`/`curl`
/// piping), and there's no `sh` to run it. Porting this to PowerShell is
/// disproportionate for a fire-and-forget cache refresh (errors are already
/// silently swallowed), so Windows just skips the doomed spawn attempt.
fn spawn_background_check(flag: &Path, current: &str) {
    if cfg!(windows) {
        return;
    }
    let flag_str = flag.to_string_lossy().to_string();
    let current = current.to_string();

    // Shell script: uses git ls-remote (no rate limits) to get latest tag, compares, writes cache.
    // Runs completely detached — parent doesn't wait.
    let script = format!(
        r#"
TAG=$(GIT_HTTP_LOW_SPEED_LIMIT=1000 GIT_HTTP_LOW_SPEED_TIME=5 git ls-remote --tags --sort=version:refname https://github.com/orgoj/hcom.git 2>/dev/null | grep -v '\^{{}}' | tail -1 | sed 's|.*refs/tags/||')
# Fallback to GitHub API if git unavailable
if [ -z "$TAG" ]; then
    TAG=$(curl -fsSL --max-time 5 'https://api.github.com/repos/orgoj/hcom/releases?per_page=1' 2>/dev/null | grep '"tag_name"' | head -1 | cut -d'"' -f4)
fi
VER="${{TAG#v}}"
if [ -n "$VER" ]; then
    # Compare: if remote > current, write version; else write empty
    REMOTE=$(echo "$VER" | awk -F. '{{split($3,p,"-"); printf "%d%06d%06d%06d", $1, $2, p[1], $4}}')
    LOCAL=$(echo "{current}" | awk -F. '{{split($3,p,"-"); printf "%d%06d%06d%06d", $1, $2, p[1], $4}}')
    if [ "$REMOTE" -gt "$LOCAL" ] 2>/dev/null; then
        printf '%s' "$VER" > "{flag_str}"
    else
        printf '' > "{flag_str}"
    fi
else
    printf '' > "{flag_str}"
fi
"#
    );

    // Fire and forget — detach from parent process
    let _ = std::process::Command::new("sh")
        .args(["-c", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Synchronously fetch the latest version. Tries git ls-remote first (no rate limits),
/// falls back to GitHub API if git is unavailable.
fn fetch_latest_version() -> Option<String> {
    fetch_via_git().or_else(fetch_via_curl)
}

fn fetch_via_git() -> Option<String> {
    let output = std::process::Command::new("git")
        .args([
            "ls-remote",
            "--tags",
            "--sort=version:refname",
            "https://github.com/orgoj/hcom.git",
        ])
        .env("GIT_HTTP_LOW_SPEED_LIMIT", "1000")
        .env("GIT_HTTP_LOW_SPEED_TIME", "5")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let tag = body
        .lines()
        .rfind(|l| !l.ends_with("^{}"))?
        .split("refs/tags/")
        .nth(1)?
        .trim()
        .to_string();

    let ver = tag.trim_start_matches('v').to_string();
    if ver.is_empty() { None } else { Some(ver) }
}

fn fetch_via_curl() -> Option<String> {
    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "5",
            "https://api.github.com/repos/orgoj/hcom/releases?per_page=1",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let tag = body
        .lines()
        .find(|l| l.contains("\"tag_name\""))?
        .split('"')
        .nth(3)?
        .to_string();

    let ver = tag.trim_start_matches('v').to_string();
    if ver.is_empty() { None } else { Some(ver) }
}

/// Structured update information: current version, latest available, availability, and update command.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub cmd: &'static str,
}

/// Synchronously fetch current + latest version info from GitHub.
/// Single source of truth for all update-related logic (fetching, parsing, command selection).
/// Used by `hcom update` command for fresh checks.
pub fn fetch_update_info() -> anyhow::Result<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest =
        fetch_latest_version().ok_or_else(|| anyhow::anyhow!("Could not reach GitHub API"))?;

    let current_parsed = parse_version(&current);
    let latest_parsed = parse_version(&latest);
    let available = current_parsed < latest_parsed;
    let cmd = get_update_cmd();

    Ok(UpdateInfo {
        current,
        latest,
        available,
        cmd,
    })
}

/// Whether `cmd` needs POSIX shell semantics to run (currently: the Unix
/// installer, which pipes curl to `sh`).
///
/// Platform-independent so it's testable on any host; `cmd_update` uses this
/// on Windows (which has no `sh`) to decide whether to refuse instead of
/// attempting a doomed spawn.
pub(crate) fn is_shell_pipe_command(cmd: &str) -> bool {
    cmd.starts_with("curl ")
}

pub(crate) fn is_powershell_installer_command(cmd: &str) -> bool {
    cmd == WINDOWS_INSTALL_CMD
}

/// Prefer `pwsh` over `powershell`: Windows PowerShell 5.1's module load can
/// fail on a polluted `PSModulePath` (nested shells, OneDrive redirects);
/// `pwsh` isn't affected. Falls back to `powershell` if pwsh isn't installed.
pub(crate) fn windows_installer_program() -> &'static str {
    let pwsh_available = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-Command", "exit 0"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if pwsh_available { "pwsh" } else { "powershell" }
}

/// Split a plain `program arg1 arg2 ...` command string into program + args.
/// Only meant for the shell-free update commands `get_update_cmd()` returns
/// (no quoting to worry about); not a general shell parser.
pub(crate) fn split_program_args(cmd: &str) -> Option<(&str, Vec<&str>)> {
    let mut parts = cmd.split_whitespace();
    let program = parts.next()?;
    Some((program, parts.collect()))
}

/// Detect install method and return appropriate update command.
fn get_update_cmd() -> &'static str {
    platform_installer_cmd()
}

fn platform_installer_cmd() -> &'static str {
    if cfg!(windows) {
        WINDOWS_INSTALL_CMD
    } else {
        UNIX_INSTALL_CMD
    }
}

/// Check for updates (once daily cached). Returns (latest_version, update_cmd) or None.
///
/// Never blocks: if the cache is stale, spawns a background process to refresh it
/// and returns the current (possibly stale) cached result.
pub fn get_update_info() -> Option<(String, &'static str)> {
    let flag = flag_path();
    let current = env!("CARGO_PKG_VERSION");

    // Check if cache is stale and needs refresh
    let should_check = if flag.exists() {
        match flag.metadata().and_then(|m| m.modified()) {
            Ok(mtime) => {
                SystemTime::now()
                    .duration_since(mtime)
                    .unwrap_or(Duration::ZERO)
                    > CHECK_INTERVAL
            }
            Err(_) => true,
        }
    } else {
        true
    };

    if should_check {
        // Non-blocking: spawn background check, result appears on next command
        spawn_background_check(&flag, current);
    }

    // Read cached result (may be from a previous check)
    let latest = fs::read_to_string(&flag).ok()?.trim().to_string();
    if latest.is_empty() {
        return None;
    }

    // Double-check (handles manual upgrades)
    if parse_version(current) >= parse_version(&latest) {
        atomic_write(&flag, "");
        return None;
    }

    Some((latest, get_update_cmd()))
}

/// Return update notice string for stderr, or None if up to date.
pub fn get_update_notice() -> Option<String> {
    let (latest, _cmd) = get_update_info()?;
    Some(format!("→ hcom v{latest} available — run `hcom update`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("0.7.0"), Some((0, 7, 0, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3, 0)));
        assert_eq!(parse_version("0.7.24-orgoj.2"), Some((0, 7, 24, 2)));
        assert_eq!(parse_version("bad"), None);
        assert_eq!(parse_version("1.2"), None);
    }

    #[test]
    fn test_is_shell_pipe_command() {
        assert!(is_shell_pipe_command(
            "curl -fsSL https://example.com/install.sh | sh"
        ));
        assert!(!is_shell_pipe_command("tool update --yes"));
        assert!(!is_shell_pipe_command(WINDOWS_INSTALL_CMD));
        assert!(is_powershell_installer_command(WINDOWS_INSTALL_CMD));
        assert!(!is_powershell_installer_command("tool update --yes"));
    }

    #[test]
    fn test_split_program_args() {
        assert_eq!(
            split_program_args("tool update --yes"),
            Some(("tool", vec!["update", "--yes"]))
        );
        assert_eq!(
            split_program_args("program arg"),
            Some(("program", vec!["arg"]))
        );
        assert_eq!(split_program_args(""), None);
        assert_eq!(split_program_args("   "), None);
    }

    #[test]
    fn test_version_comparison() {
        assert!(parse_version("0.8.0") > parse_version("0.7.0"));
        assert!(parse_version("1.0.0") > parse_version("0.99.99"));
        assert!(parse_version("0.7.0") == parse_version("0.7.0"));
    }

    #[test]
    fn test_get_update_cmd_default() {
        // Test binary path won't match any known install method.
        let cmd = get_update_cmd();
        if cfg!(windows) {
            assert!(
                cmd.contains("hcom-installer.ps1"),
                "expected PowerShell fallback, got: {cmd}"
            );
        } else {
            assert!(cmd.contains("curl"), "expected curl fallback, got: {cmd}");
        }
    }
}

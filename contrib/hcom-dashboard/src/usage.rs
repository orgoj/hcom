//! Context window fill and rate-limit quota read from the tail of a Claude or
//! Codex transcript (port of hcom-gui `transcript_usage`).

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::SystemTime;

/// Claude transcripts do not record the context window; current Claude Code
/// models run with 1M.
pub const CLAUDE_CONTEXT_WINDOW: u64 = 1_000_000;
pub const TRANSCRIPT_TAIL_BYTES: u64 = 1 << 20;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Usage {
    /// (tokens, window) of the latest model call.
    pub context: Option<(u64, u64)>,
    /// Codex primary rate limit, used percent.
    pub quota: Option<f64>,
}

/// Transcript path -> ((mtime, size), usage).
pub type UsageCache = HashMap<String, ((SystemTime, u64), Usage)>;

/// Usage of a transcript, cached by (mtime, size). Symlinks and anything that
/// is not a regular file (FIFO, device) give no usage and are never opened.
pub fn transcript_usage(tool: &str, path: &str, cache: &mut UsageCache) -> Usage {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Usage::default();
    };
    if !meta.file_type().is_file() {
        return Usage::default();
    }
    let key = (
        meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        meta.len(),
    );
    if let Some((k, u)) = cache.get(path)
        && *k == key
    {
        return *u;
    }
    let usage = read_tail(Path::new(path))
        .map(|tail| parse_usage(tool, &tail))
        .unwrap_or_default();
    cache.insert(path.to_string(), (key, usage));
    usage
}

fn read_tail(path: &Path) -> Option<Vec<u8>> {
    // O_NOFOLLOW: a path swapped to a symlink after the lstat check fails
    // (ELOOP); O_NONBLOCK: one swapped to a FIFO does not block the open.
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = f.metadata().ok()?;
    if !meta.file_type().is_file() {
        return None;
    }
    f.seek(SeekFrom::Start(
        meta.len().saturating_sub(TRANSCRIPT_TAIL_BYTES),
    ))
    .ok()?;
    let mut buf = Vec::new();
    f.take(TRANSCRIPT_TAIL_BYTES).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Usage from transcript bytes, newest line first.
pub fn parse_usage(tool: &str, tail: &[u8]) -> Usage {
    let marker: &[u8] = match tool {
        "claude" => b"\"usage\"",
        "codex" => b"\"token_count\"",
        _ => return Usage::default(),
    };
    let mut usage = Usage::default();
    for line in tail.split(|b| *b == b'\n').rev() {
        if !line.windows(marker.len()).any(|w| w == marker) {
            continue;
        }
        // The cut first line of the tail does not parse.
        let Ok(e) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        if tool == "claude" {
            if e["type"] != "assistant" || e["isSidechain"].as_bool() == Some(true) {
                continue;
            }
            let u = &e["message"]["usage"];
            let tokens: u64 = [
                "input_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
            ]
            .iter()
            .map(|k| u[*k].as_u64().unwrap_or(0))
            .sum();
            if tokens > 0 {
                usage.context = Some((tokens, CLAUDE_CONTEXT_WINDOW.max(tokens)));
                break;
            }
        } else {
            let payload = &e["payload"];
            let info = &payload["info"];
            let tokens = info["last_token_usage"]["input_tokens"]
                .as_u64()
                .unwrap_or(0);
            let window = info["model_context_window"].as_u64().unwrap_or(0);
            if tokens > 0 && window > 0 && usage.context.is_none() {
                usage.context = Some((tokens, window));
            }
            if usage.quota.is_none() {
                usage.quota = payload["rate_limits"]["primary"]["used_percent"].as_f64();
            }
            if usage.context.is_some() && usage.quota.is_some() {
                break;
            }
        }
    }
    usage
}

/// Context cell text, e.g. `"123k 45%"` (parsed back by `parse_context_percent`).
pub fn context_text(tokens: u64, window: u64) -> String {
    format!(
        "{}k {}%",
        (tokens as f64 / 1000.0).round() as u64,
        (100.0 * tokens as f64 / window as f64).round() as u64
    )
}

/// Fill each row's context and quota cells from its transcript.
pub fn apply_usage(rows: &mut [crate::table_data::TableRow], cache: &mut UsageCache) {
    for r in rows {
        let usage = match &r.transcript_path {
            Some(path) if !path.is_empty() => transcript_usage(&r.cli, path, cache),
            _ => Usage::default(),
        };
        r.context_tokens = usage.context.map(|(t, _)| t);
        r.context_fill = usage
            .context
            .map(|(t, w)| context_text(t, w))
            .unwrap_or_default();
        r.quota_percent = usage.quota.map(|q| q.round() as u64);
        r.quota_fill = usage.quota.map(quota_text).unwrap_or_default();
    }
}
pub fn quota_text(percent: f64) -> String {
    format!("{}%", percent.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_uses_newest_main_chain_assistant_usage() {
        let tail = concat!(
            "cut line \"usage\" {\n",
            r#"{"type":"assistant","message":{"usage":{"input_tokens":1,"cache_creation_input_tokens":2,"cache_read_input_tokens":3000}}}"#,
            "\n",
            r#"{"type":"assistant","isSidechain":true,"message":{"usage":{"input_tokens":999999}}}"#,
            "\n",
        );
        let u = parse_usage("claude", tail.as_bytes());
        assert_eq!(u.context, Some((3003, CLAUDE_CONTEXT_WINDOW)));
        assert_eq!(u.quota, None);
    }

    #[test]
    fn claude_window_grows_to_token_count() {
        let tail = r#"{"type":"assistant","message":{"usage":{"input_tokens":1500000}}}"#;
        let u = parse_usage("claude", tail.as_bytes());
        assert_eq!(u.context, Some((1_500_000, 1_500_000)));
    }

    #[test]
    fn codex_takes_newest_context_and_quota_separately() {
        let tail = concat!(
            r#"{"payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":50000},"model_context_window":200000}}}"#,
            "\n",
            r#"{"payload":{"type":"token_count","info":null,"rate_limits":{"primary":{"used_percent":41.6}}}}"#,
            "\n",
        );
        let u = parse_usage("codex", tail.as_bytes());
        assert_eq!(u.context, Some((50000, 200000)));
        assert_eq!(u.quota, Some(41.6));
        assert_eq!(context_text(50000, 200000), "50k 25%");
        assert_eq!(quota_text(41.6), "42%");
    }

    #[test]
    fn other_tools_and_missing_fields_give_no_usage() {
        let line = r#"{"type":"assistant","message":{"usage":{}}}"#;
        assert_eq!(parse_usage("claude", line.as_bytes()), Usage::default());
        assert_eq!(parse_usage("gemini", line.as_bytes()), Usage::default());
    }

    #[test]
    fn cache_is_reused_until_the_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.jsonl");
        let line = |n: u64| {
            format!(r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":{n}}}}}}}"#) + "\n"
        };
        std::fs::write(&path, line(1000)).unwrap();
        let p = path.to_str().unwrap();
        let mut cache = UsageCache::new();
        assert_eq!(
            transcript_usage("claude", p, &mut cache).context.unwrap().0,
            1000
        );
        std::fs::write(&path, line(20000)).unwrap();
        assert_eq!(
            transcript_usage("claude", p, &mut cache).context.unwrap().0,
            20000
        );
    }

    #[test]
    fn symlink_and_fifo_paths_are_rejected_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.jsonl");
        std::fs::write(
            &real,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":5}}}"#,
        )
        .unwrap();
        let link = dir.path().join("link.jsonl");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let fifo = dir.path().join("fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(status.success());
        let mut cache = UsageCache::new();
        for p in [&link, &fifo, &dir.path().to_path_buf()] {
            assert_eq!(
                transcript_usage("claude", p.to_str().unwrap(), &mut cache),
                Usage::default()
            );
        }
        assert!(cache.is_empty());
    }

    #[test]
    fn apply_usage_fills_cells_and_sort_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex.jsonl");
        std::fs::write(
            &path,
            r#"{"payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":120000},"model_context_window":400000},"rate_limits":{"primary":{"used_percent":7.4}}}}"#,
        )
        .unwrap();
        let mut rows = vec![
            crate::table_data::TableRow {
                cli: "codex".into(),
                transcript_path: Some(path.to_string_lossy().into_owned()),
                ..Default::default()
            },
            crate::table_data::TableRow::default(),
        ];
        apply_usage(&mut rows, &mut UsageCache::new());
        assert_eq!(rows[0].context_fill, "120k 30%");
        assert_eq!(rows[0].context_tokens, Some(120000));
        assert_eq!(rows[0].quota_fill, "7%");
        assert_eq!(rows[0].quota_percent, Some(7));
        assert_eq!(rows[1].context_fill, "");
        assert_eq!(rows[1].quota_fill, "");
    }

    /// The open itself rejects a symlink or FIFO swapped in after the lstat
    /// check, without blocking.
    #[test]
    fn read_tail_rejects_symlink_and_fifo_at_open() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.jsonl");
        std::fs::write(&real, "x").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let fifo = dir.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(read_tail(&real).as_deref(), Some(&b"x"[..]));
        assert_eq!(read_tail(&link), None);
        assert_eq!(read_tail(&fifo), None);
    }
}

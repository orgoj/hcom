//! Fork: run the configured `bigboss_notify_command` for messages addressed to
//! bigboss, so the user notices them without watching hcom.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::log::log_warn;

/// Characters of `{preview}`; desktop notifications show only a few lines.
const PREVIEW_CHARS: usize = 200;

/// `text` on one line (whitespace runs collapsed), cut to [`PREVIEW_CHARS`]
/// with a trailing `…`.
fn preview(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(PREVIEW_CHARS) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    }
}

/// Argv of `command` (shell-quoted, no shell) with `{id}`, `{from}`,
/// `{preview}` and `{text}` substituted inside each token, so every value
/// stays one argument. One pass: substituted values are never re-expanded.
fn build_argv(command: &str, from: &str, text: &str, event_id: i64) -> Result<Vec<String>, String> {
    let values = [
        ("{id}", event_id.to_string()),
        ("{from}", from.to_string()),
        ("{preview}", preview(text)),
        ("{text}", text.to_string()),
    ];
    let argv: Vec<String> = shell_words::split(command)
        .map_err(|e| format!("invalid shell quoting: {e}"))?
        .into_iter()
        .map(|token| {
            let mut out = String::new();
            let mut rest = token.as_str();
            while !rest.is_empty() {
                match values.iter().find(|(key, _)| rest.starts_with(key)) {
                    Some((key, value)) => {
                        out.push_str(value);
                        rest = &rest[key.len()..];
                    }
                    None => {
                        let ch = rest.chars().next().unwrap();
                        out.push(ch);
                        rest = &rest[ch.len_utf8()..];
                    }
                }
            }
            out
        })
        .collect();
    if argv.is_empty() {
        return Err("empty command".into());
    }
    Ok(argv)
}

/// Spawn the notify command detached when it is configured. Never fails the
/// send: problems are logged.
pub(crate) fn notify(from: &str, text: &str, event_id: i64) {
    let command = crate::config::HcomConfig::load(None).bigboss_notify_command;
    if command.trim().is_empty() {
        return;
    }
    let result = build_argv(&command, from, text, event_id).and_then(|argv| {
        Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| format!("{}: {e}", argv[0]))
    });
    match result {
        // Reap in the background so a long-lived sender leaves no zombie; a
        // short `hcom send` exits first and the child is reparented.
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(error) => log_warn(
            "send",
            "bigboss_notify.failed",
            &format!("event={event_id} error={error}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_stay_single_tokens() {
        let argv = build_argv(
            "notify-send 'hcom: {from}' {text} --hint=id:{id}",
            "knowledge",
            "two words; $(rm -rf ~) {from}",
            42,
        )
        .unwrap();
        assert_eq!(
            argv,
            [
                "notify-send",
                "hcom: knowledge",
                "two words; $(rm -rf ~) {from}",
                "--hint=id:42"
            ]
        );
    }

    #[test]
    fn preview_is_one_short_line_and_never_expands() {
        let long = format!("Threat check\n\n{}", "ž".repeat(300));
        let argv = build_argv("x {preview}", "a", &long, 1).unwrap();
        assert!(argv[1].starts_with("Threat check žž"), "{}", argv[1]);
        assert_eq!(argv[1].chars().count(), PREVIEW_CHARS + 1);
        assert!(argv[1].ends_with('…'));

        let argv = build_argv("x {preview}", "a", "short {text} {id}", 7).unwrap();
        assert_eq!(argv[1], "short {text} {id}");
    }

    #[test]
    fn rejects_empty_and_bad_quoting() {
        assert!(build_argv("   ", "a", "b", 1).is_err());
        assert!(build_argv("notify-send 'open", "a", "b", 1).is_err());
    }
}

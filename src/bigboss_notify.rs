//! Fork: run the configured `bigboss_notify_command` for messages addressed to
//! bigboss, so the user notices them without watching hcom.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::log::log_warn;

/// Argv of `command` (shell-quoted, no shell) with `{id}`, `{from}` and
/// `{text}` substituted inside each token, so every value stays one argument.
/// `{text}` goes last: message text must not expand the other placeholders.
fn build_argv(command: &str, from: &str, text: &str, event_id: i64) -> Result<Vec<String>, String> {
    let id = event_id.to_string();
    let argv: Vec<String> = shell_words::split(command)
        .map_err(|e| format!("invalid shell quoting: {e}"))?
        .into_iter()
        .map(|token| {
            token
                .replace("{id}", &id)
                .replace("{from}", from)
                .replace("{text}", text)
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
    fn rejects_empty_and_bad_quoting() {
        assert!(build_argv("   ", "a", "b", 1).is_err());
        assert!(build_argv("notify-send 'open", "a", "b", 1).is_err());
    }
}

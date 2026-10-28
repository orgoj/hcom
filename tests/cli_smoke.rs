//! Hermetic CLI smoke tests: invoke the `hcom` binary in a temp HCOM_DIR and
//! assert exit codes + stdout shape.

mod support;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};
use support::{Hcom, parse_hcom_marker};

#[cfg(unix)]
#[test]
fn fixture_pins_cargo_target_before_dev_root_reexec() {
    use std::os::unix::fs::PermissionsExt;

    let h = Hcom::new();
    let foreign_root = h.root_path().join("foreign-checkout");
    let foreign_bin = foreign_root.join("target/debug/hcom");
    std::fs::create_dir_all(foreign_bin.parent().unwrap()).unwrap();
    std::fs::write(&foreign_bin, "#!/bin/sh\nprintf 'foreign binary\\n'\n").unwrap();
    std::fs::set_permissions(&foreign_bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = h
        .cmd()
        .env("HCOM_DEV_ROOT", &foreign_root)
        .args(["config", "HCOM_DIR"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        h.hcom_dir.to_string_lossy(),
    );
    let log = std::fs::read_to_string(h.hcom_dir.join(".tmp/logs/hcom.log")).unwrap_or_default();
    assert!(!log.contains("\"event\":\"dev_root_reexec\""), "{log}");
}

#[cfg(unix)]
#[test]
fn pty_defaults_missing_term_and_preserves_explicit_term() {
    let h = Hcom::new();
    for term in [None, Some("xterm-kitty"), Some("dumb"), Some("")] {
        let mut command = h.cmd();
        command.env_remove("TERM");
        if let Some(term) = term {
            command.env("TERM", term);
        }
        let output = command
            .args([
                "pty",
                "sh",
                "-c",
                "printf 'CHILD_TERM=<%s>' \"${TERM-unset}\"",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "TERM={term:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = format!("CHILD_TERM=<{}>", term.unwrap_or("xterm-256color"));
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(&expected),
            "TERM={term:?}: stdout={:?}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_pty_answers_startup_queries_while_waiting_for_terminal_metadata() {
    let h = Hcom::new();
    let name = h.start();
    let process_id = "cpr-startup-metadata";
    let id_dir = h.hcom_dir.join(".tmp/terminal_ids");
    std::fs::create_dir_all(&id_dir).unwrap();
    let id_file = id_dir.join(process_id);
    let log_file = h.hcom_dir.join(".tmp/logs/hcom.log");

    // The terminal ID arrives only once the proxy has answered ConPTY's
    // cursor query. Before the fix, metadata collection finished (and missed
    // this ID) before the reader could answer; slow launches also leaked the
    // expired cursor reply into the child's prompt.
    let terminal = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if std::fs::read_to_string(&log_file)
                .unwrap_or_default()
                .contains("\"event\":\"startup.dsr_answered\"")
            {
                std::fs::write(id_file, "terminal-after-cpr").unwrap();
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    });
    let output = h
        .cmd()
        .env("HCOM_INSTANCE_NAME", &name)
        .env("HCOM_PROCESS_ID", process_id)
        // Ignore pane IDs inherited from the terminal running this test.
        .env("HCOM_LAUNCHED_PRESET", "cpr-test-without-pane")
        .args([
            "pty",
            "codex",
            "--hcom-tool-path",
            "cmd.exe",
            "/d",
            "/c",
            "echo CPR_CHILD_READY",
        ])
        .output()
        .unwrap();
    assert!(terminal.join().unwrap(), "proxy did not answer startup CPR");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CPR_CHILD_READY"));
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).unwrap();
    let context: String = conn
        .query_row(
            "SELECT launch_context FROM instances WHERE name = ?1",
            [&name],
            |row| row.get(0),
        )
        .unwrap();
    let context: serde_json::Value = serde_json::from_str(&context).unwrap();
    assert_eq!(context["terminal_id"], "terminal-after-cpr");
}

#[test]
fn windows_hook_cleanup_preserves_foreign_hooks_and_stays_in_fixture() {
    let h = Hcom::new();
    for (tool, event, suffix, filename) in [
        (
            "claude",
            "SessionStart",
            "claude-sessionstart",
            "settings.json",
        ),
        (
            "cursor",
            "sessionStart",
            "cursor-sessionstart",
            "hooks.json",
        ),
        ("codex", "SessionStart", "codex-sessionstart", "hooks.json"),
    ] {
        let dir = h.home.join(format!(".{tool}"));
        std::fs::create_dir_all(&dir).unwrap();
        let handlers = serde_json::json!([
            {"type": "command", "command": format!("C:/dev/hcom.exe {suffix}")},
            {"type": "command", "command": "user-hook"},
            {"type": "command", "command": "echo \"hcom cursor-stop\""}
        ]);
        let hooks = if tool == "cursor" {
            serde_json::json!({"hooks": {event: handlers}})
        } else {
            serde_json::json!({"hooks": {event: [{"hooks": handlers}]}})
        };
        let path = dir.join(filename);
        std::fs::write(&path, hooks.to_string()).unwrap();
        let mut command = h.cmd();
        if tool == "codex" {
            command.env("CODEX_HOME", &dir);
        }
        let output = command.args(["hooks", "remove", tool]).output().unwrap();
        assert!(
            output.status.success(),
            "{tool}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let remaining = std::fs::read_to_string(path).unwrap();
        assert!(!remaining.contains("hcom.exe"), "{tool}: {remaining}");
        assert!(remaining.contains("user-hook"), "{tool}: {remaining}");
        assert!(remaining.contains("echo"), "{tool}: {remaining}");
    }

    let outside = tempfile::tempdir().unwrap();
    let sentinel = outside.path().join("hooks.json");
    let original =
        r#"{"hooks":{"SessionStart":[{"hooks":[{"command":"hcom codex-sessionstart"}]}]}}"#;
    std::fs::write(&sentinel, original).unwrap();
    let output = h
        .cmd()
        .env("CODEX_HOME", outside.path())
        .args(["hooks", "remove", "codex"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(sentinel).unwrap(), original);
}

#[test]
fn fixture_drop_terminates_registered_process_group() {
    #[cfg(unix)]
    let mut child = Command::new("sh")
        .args(["-c", "exec sleep 60"])
        .process_group(0)
        .spawn()
        .expect("spawn cleanup test process group");
    #[cfg(windows)]
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
        .spawn()
        .expect("spawn cleanup test process group");
    let pid = i64::from(child.id());

    let h = Hcom::new();
    h.track_cleanup_pid(pid);
    assert!(
        h.process_group_alive(pid),
        "cleanup test process group did not start"
    );

    let reaper = std::thread::spawn(move || child.wait().expect("reap cleanup test process"));
    drop(h);
    let status = reaper.join().expect("cleanup reaper thread");
    assert!(
        !status.success(),
        "fixture cleanup should terminate the registered process"
    );

    let deadline = Instant::now() + Duration::from_secs(7);
    while Instant::now() < deadline && support::process_group_alive(pid) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !support::process_group_alive(pid),
        "fixture drop left process group {pid} alive"
    );
}

#[cfg(unix)]
#[test]
fn fixture_drop_terminates_orphan_without_instance_row() {
    let h = Hcom::new();
    // Only the pidfile owns this process; neither instance rows nor explicit
    // fixture cleanup registration can discover it.
    // Reap the descendant on group termination so zombie lifetime does not
    // depend on the host init process. Killing only the shell still leaves it
    // waiting for its live child, which the bounded exit check detects.
    let ready_path = h.root_path().join("orphan-ready");
    let mut child = Command::new("sh")
        .args([
            "-c",
            r#"sleep 60 & descendant=$!; trap 'wait "$descendant"; exit 143' TERM; printf ready > "$1"; wait "$descendant""#,
            "orphan-fixture",
        ])
        .arg(&ready_path)
        .process_group(0)
        .spawn()
        .expect("spawn orphan process group");
    let pid = i64::from(child.id());
    let tmp = h.hcom_dir.join(".tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("launched_pids.json"),
        serde_json::json!({pid.to_string(): {
            "tool": "claude", "names": ["orphan"], "launched_at": 1.0
        }})
        .to_string(),
    )
    .unwrap();
    // Confirm the descendant exists without an unbounded pipe read. Register
    // its group first so fixture cleanup also runs if readiness times out.
    h.eventually("orphan descendant started", Duration::from_secs(3), || {
        Ok(ready_path.exists().then_some(()))
    });
    drop(h);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().expect("poll orphan") {
            assert!(
                !status.success(),
                "orphan must be terminated by fixture teardown"
            );
            break;
        }
        if Instant::now() >= deadline {
            support::terminate_process_group(pid);
            let _ = child.kill();
            let _ = child.wait();
            panic!("fixture teardown left orphan {pid} alive");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while support::process_group_alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if support::process_group_alive(pid) {
        support::terminate_process_group(pid);
        panic!("fixture teardown left orphan descendants in group {pid}");
    }
}

#[test]
fn help_prints_and_exits_zero() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["--help"]);
    assert_eq!(code, 0, "stdout={stdout}");
    assert!(stdout.contains("hcom"), "stdout={stdout}");
    assert!(
        stdout.contains("Commands:") || stdout.contains("Launch:"),
        "stdout={stdout}"
    );
}

#[test]
fn config_terminal_accepts_here_mode() {
    let h = Hcom::new();

    let (code, stdout, stderr) = h.run(["config", "terminal", "here"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Terminal set to: here"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["config", "terminal"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Terminal: here"), "stdout={stdout}");
    assert!(stdout.contains("here ← current"), "stdout={stdout}");
}

#[test]
fn version_prints_and_exits_zero() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["--version"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(
        stdout.trim(),
        concat!("hcom ", env!("CARGO_PKG_VERSION")),
        "stderr={stderr}"
    );
}

#[test]
fn status_json_in_fresh_dir() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["status", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("status json: {e}\n{stdout}"));
    assert_eq!(v["hcom_dir"].as_str(), Some(h.path().to_str().unwrap()));
    assert_eq!(v["instances"]["total"], 0);
}

#[test]
fn status_clean_logs_displays_path() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["status"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("logs:      ✓ ok"));
    let log_path = h.path().join(".tmp/logs/hcom.log");
    assert!(
        stdout.contains(&log_path.display().to_string()),
        "stdout should contain log path: {stdout}"
    );
}

#[test]
fn list_json_empty() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    let arr = v.as_array().expect("list returns array");
    assert!(arr.is_empty(), "expected empty list, got {stdout}");
}

#[test]
fn kill_json_reports_every_target() {
    let h = Hcom::new();
    let agent = h.start();
    let (code, _, stderr) = h.run(["stop", &agent]);
    assert_eq!(code, 0, "stop failed: {stderr}");

    let (code, stdout, stderr) = h.run(["kill", &agent, "nosuchagent", "--json"]);
    assert_eq!(
        code, 1,
        "a missing target fails the command: stderr={stderr}"
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("kill --json object");
    assert_eq!(report["ok"], false);
    let results = report["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2, "one outcome per target: {stdout}");
    assert_eq!(results[0]["target"], agent.as_str());
    assert_eq!(results[0]["kind"], "stopped");
    assert_eq!(results[0]["result"], "already_stopped");
    assert_eq!(results[0]["resume"], format!("hcom r {agent}"));
    assert_eq!(results[1]["target"], "nosuchagent");
    assert_eq!(results[1]["result"], "error");
    assert!(results[1]["error"].is_string(), "{stdout}");
}

#[cfg(unix)]
#[test]
fn launch_json_reports_names_for_kill_json() {
    use std::os::unix::fs::PermissionsExt;

    let h = Hcom::new();
    // A stand-in `claude` that answers --version and then idles: enough for
    // hcom to launch, track, and kill it without a real tool.
    let fake_bin = h.root_path().join("fake-bin");
    std::fs::create_dir_all(&fake_bin).expect("create fake bin");
    let fake_claude = fake_bin.join("claude");
    std::fs::write(
        &fake_claude,
        "#!/bin/sh\ncase \"$1\" in --version) echo '2.1.200 (Claude Code)'; exit 0;; esac\nsleep 300\n",
    )
    .expect("write fake claude");
    std::fs::set_permissions(&fake_claude, std::fs::Permissions::from_mode(0o755))
        .expect("chmod fake claude");
    let hcom_bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_hcom"))
        .parent()
        .expect("hcom bin dir")
        .to_path_buf();
    let path = std::env::join_paths([hcom_bin_dir, fake_bin].into_iter().chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("join PATH");

    let output = h
        .cmd()
        .args(["1", "claude", "--headless", "--json"])
        .env("PATH", &path)
        .output()
        .expect("run launch");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    let launch: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is exactly one JSON object");
    assert_eq!(launch["action"], "launch");
    assert_eq!(launch["tool"], "claude");
    assert_eq!(launch["launched"], 1);
    assert_eq!(launch["failed"], 0);
    assert!(
        launch["batch_id"].as_str().is_some_and(|b| !b.is_empty()),
        "{launch}"
    );
    let names = launch["names"].as_array().expect("names array");
    assert_eq!(names.len(), 1, "{launch}");
    let name = names[0].as_str().expect("name string");

    let (_, stdout, stderr) = h.run(["kill", name, "--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout} / {stderr}"));
    let outcome = &report["results"][0];
    assert_eq!(outcome["target"], name, "{report}");
    // Usually still running; a slow host may have already given up waiting
    // for the fake tool to become ready, which stops it first.
    assert!(
        outcome["kind"] == "agent" || outcome["kind"] == "stopped",
        "{report}"
    );
    assert_eq!(outcome["resume"], format!("hcom r {name}"));
}

#[test]
fn events_empty_in_fresh_dir() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["events", "--last", "5"]);
    assert_eq!(code, 0);
    assert!(stdout.trim().is_empty(), "expected no events, got {stdout}");
}

#[test]
fn send_without_identity_errors_with_hint() {
    let h = Hcom::new();
    let (code, _stdout, stderr) = h.run(["send", "@nobody", "--", "hi"]);
    assert_ne!(code, 0, "send without identity must fail: stderr={stderr}");
    assert!(
        stderr.contains("identity not found"),
        "expected stable hint, got: {stderr}"
    );
}

#[test]
fn send_to_missing_agent_lists_available() {
    let h = Hcom::new();
    let me = h.start();

    let (code, _stdout, stderr) = h.run(["send", "@nope", "--name", &me, "--", "hi"]);
    assert_ne!(code, 0, "send to nonexistent must fail");
    assert!(
        stderr.contains("@nope") && stderr.contains("Available:"),
        "stderr={stderr}"
    );
}

#[test]
fn send_strips_redundant_trailing_name_from_auto_resolved_sender() {
    let h = Hcom::new();
    let recipient = h.start();
    let process_id = "send-trailing-name-process";

    let mut start = h.cmd();
    start.env("HCOM_PROCESS_ID", process_id).arg("start");
    let start_out = start.output().expect("spawn hcom start");
    let start_stdout = String::from_utf8_lossy(&start_out.stdout);
    let start_stderr = String::from_utf8_lossy(&start_out.stderr);
    assert!(
        start_out.status.success(),
        "stdout={start_stdout} stderr={start_stderr}"
    );
    let sender = parse_hcom_marker(&start_stdout).expect("sender marker");

    let mut send = h.cmd();
    send.env("HCOM_PROCESS_ID", process_id).args([
        "send",
        &format!("@{recipient}"),
        "--",
        "ack",
        "--name",
        &sender,
    ]);
    let send_out = send.output().expect("spawn hcom send");
    let send_stdout = String::from_utf8_lossy(&send_out.stdout);
    let send_stderr = String::from_utf8_lossy(&send_out.stderr);
    assert!(
        send_out.status.success(),
        "stdout={send_stdout} stderr={send_stderr}"
    );

    let (_, events, _) = h.run(["events", "--type", "message", "--last", "1"]);
    assert!(events.contains(r#""text":"ack""#), "events={events}");
    assert!(!events.contains("--name"), "events={events}");
}

#[test]
fn ai_tool_broadcast_to_many_requires_go_preview() {
    let h = Hcom::new();
    let sender = h.start();
    for _ in 0..4 {
        h.start();
    }

    let mut cmd = h.cmd();
    cmd.env("CODEX_SANDBOX", "1").args([
        "send",
        "--name",
        &sender,
        "--",
        "probably meant one person",
    ]);
    let out = cmd.output().expect("spawn hcom send");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        code, 0,
        "send should preview first: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("BROADCAST SEND PREVIEW")
            && stdout.contains("broadcast to 4 agents")
            && stdout.contains("Did you mean to send this to everyone?")
            && stdout.contains("hcom send --go"),
        "stdout={stdout}"
    );

    let (_, events_out, _) = h.run(["events", "--type", "message", "--last", "5"]);
    assert!(
        events_out.trim().is_empty(),
        "preview must not send a message: events={events_out}"
    );

    let mut go_cmd = h.cmd();
    go_cmd.env("CODEX_SANDBOX", "1").args([
        "--go",
        "send",
        "--name",
        &sender,
        "--",
        "confirmed broadcast",
    ]);
    let go_out = go_cmd.output().expect("spawn hcom --go send");
    let go_code = go_out.status.code().unwrap_or(-1);
    let go_stdout = String::from_utf8_lossy(&go_out.stdout);
    let go_stderr = String::from_utf8_lossy(&go_out.stderr);
    assert_eq!(go_code, 0, "stdout={go_stdout} stderr={go_stderr}");
    assert!(
        go_stdout.contains("Sent to:") || go_stdout.contains("Sent to 4 agents"),
        "stdout={go_stdout}"
    );
}

#[test]
fn non_destructive_reset_paths_deliver_pending_messages() {
    let h = Hcom::new();
    let sender = h.start();
    let process_id = "reset-pending-delivery-process";
    let recipient = h.start_with_process_id(process_id);

    let send_message = |text: &str| {
        let (code, stdout, stderr) = h.run([
            "send",
            "--name",
            &sender,
            &format!("@{recipient}"),
            "--",
            text,
        ]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    };

    send_message("pending during reset preview");
    let mut preview = h.cmd();
    preview
        .env("HCOM_PROCESS_ID", process_id)
        .env("CODEX_SANDBOX", "1")
        .arg("reset");
    let output = preview.output().expect("run reset preview");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("RESET PREVIEW"), "stdout={stdout}");
    assert!(
        stdout.contains("pending during reset preview"),
        "stdout={stdout}"
    );

    send_message("pending during reset hooks");
    let mut hooks = h.cmd();
    hooks
        .env("HCOM_PROCESS_ID", process_id)
        .env("CODEX_SANDBOX", "1")
        .args(["--go", "reset", "hooks"]);
    let output = hooks.output().expect("run reset hooks");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("pending during reset hooks"),
        "stdout={stdout}"
    );
}

#[test]
fn start_send_events_roundtrip() {
    let h = Hcom::new();
    let sender = h.start();
    let recipient = h.start();
    assert_ne!(sender, recipient, "two starts must assign distinct names");

    let (c, stdout, stderr) = h.run([
        "send",
        &format!("@{recipient}"),
        "--name",
        &sender,
        "--",
        "hello there",
    ]);
    assert_eq!(c, 0, "stderr={stderr} stdout={stdout}");

    let (c4, events_out, _) = h.run(["events", "--last", "10"]);
    assert_eq!(c4, 0);
    let message_lines: Vec<_> = events_out
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["type"] == "message")
        .collect();
    assert_eq!(message_lines.len(), 1, "events={events_out}");
    let msg = &message_lines[0];
    assert_eq!(msg["instance"], sender.as_str(), "attribution = sender");
    assert_eq!(msg["data"]["from"], sender.as_str());
    assert_eq!(msg["data"]["text"], "hello there");

    // Recipient/scope contract: the message event only carries from/text, so
    // we check routing via per-instance unread on `list --json`. Recipient
    // must show unread=1, sender unread=0.
    let (c5, list_out, _) = h.run(["list", "--json"]);
    assert_eq!(c5, 0);
    let list: serde_json::Value = serde_json::from_str(&list_out).expect("list json");
    let by_name: std::collections::HashMap<_, _> = list
        .as_array()
        .expect("array")
        .iter()
        .map(|v| {
            (
                v["name"].as_str().unwrap().to_string(),
                v["unread_count"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    assert_eq!(
        by_name.get(&recipient).copied(),
        Some(1),
        "recipient unread; list={list_out}"
    );
    assert_eq!(
        by_name.get(&sender).copied(),
        Some(0),
        "sender unread; list={list_out}"
    );

    let (c6, listen_out, listen_err) =
        h.run(["listen", "--name", &recipient, "--timeout", "1", "--json"]);
    assert_eq!(c6, 0, "listen failed: stderr={listen_err}");
    let delivered: Vec<serde_json::Value> = listen_out
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert_eq!(delivered.len(), 1, "listen output={listen_out}");
    assert_eq!(delivered[0]["from"], sender.as_str());
    assert_eq!(delivered[0]["text"], "hello there");
    let event_id = delivered[0]["event_id"].as_i64().expect("event_id");
    assert_eq!(delivered[0]["reply_id"], event_id.to_string());

    let (c7, list_after_listen_out, _) = h.run(["list", "--json"]);
    assert_eq!(c7, 0);
    let list_after_listen: serde_json::Value =
        serde_json::from_str(&list_after_listen_out).expect("list json after listen");
    let after_by_name: std::collections::HashMap<_, _> = list_after_listen
        .as_array()
        .expect("array")
        .iter()
        .map(|v| {
            (
                v["name"].as_str().unwrap().to_string(),
                v["unread_count"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    assert_eq!(
        after_by_name.get(&recipient).copied(),
        Some(0),
        "listen should advance recipient cursor; list={list_after_listen_out}"
    );
}

#[test]
fn intent_and_reply_to_roundtrip() {
    // Wiki contract (messaging.md §Intent + event-model.md `msg_intent`/`reply_to_local`):
    // request → ack with --reply-to flattens through `events_v` so threads/replies
    // can be traced. Locks: data.intent on send, and data.reply_to_local resolved
    // from the parent event id.
    let h = Hcom::new();
    let a = h.start();
    let b = h.start();

    let (c, _, e) = h.run([
        "send",
        &format!("@{b}"),
        "--name",
        &a,
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(c, 0, "request send failed: stderr={e}");

    let (_, req_out, _) = h.run(["events", "--type", "message", "--from", &a, "--last", "5"]);
    let req: serde_json::Value = req_out
        .lines()
        .find_map(|l| serde_json::from_str(l).ok())
        .expect("request event present");
    assert_eq!(req["data"]["intent"], "request");
    let req_id = req["id"].as_i64().expect("event id is i64");

    let (c2, _, e2) = h.run([
        "send",
        &format!("@{a}"),
        "--name",
        &b,
        "--intent",
        "ack",
        "--reply-to",
        &req_id.to_string(),
        "--",
        "pong",
    ]);
    assert_eq!(c2, 0, "ack send failed: stderr={e2}");

    let (_, ack_out, _) = h.run(["events", "--intent", "ack", "--last", "5"]);
    let ack: serde_json::Value = ack_out
        .lines()
        .find_map(|l| serde_json::from_str(l).ok())
        .expect("ack event present");
    assert_eq!(ack["data"]["intent"], "ack");
    assert_eq!(ack["data"]["from"], b.as_str());
    assert_eq!(
        ack["data"]["reply_to_local"].as_i64(),
        Some(req_id),
        "reply_to_local must resolve to request event id; ack={ack}"
    );
}

#[test]
fn remote_reply_to_origin_resolves_imported_event_row() {
    let h = Hcom::new();
    let a = h.start();
    let b = h.start();

    // Direct SQLite access to seed colliding local event and imported remote event
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");

    // Colliding local message at id = 42
    let local_data = serde_json::json!({
        "from": a,
        "text": "colliding local message",
        "intent": "request"
    });
    conn.execute(
        "INSERT INTO events (id, type, instance, timestamp, data) VALUES (42, 'message', ?1, '2026-09-04T00:00:00Z', ?2)",
        rusqlite::params![a, local_data.to_string()],
    )
    .expect("insert colliding local event");

    // Imported relay message at id = 100 with origin 42 and device short BOXE
    let remote_data = serde_json::json!({
        "from": "remote:BOXE",
        "text": "remote request",
        "intent": "request",
        "_relay": {
            "id": 42,
            "short": "BOXE",
            "device": "dev-uuid-boxe"
        }
    });
    conn.execute(
        "INSERT INTO events (id, type, instance, timestamp, data) VALUES (100, 'message', 'remote:BOXE', '2026-09-04T00:00:01Z', ?1)",
        rusqlite::params![remote_data.to_string()],
    )
    .expect("insert remote imported event");

    // Reply using remote token 42:BOXE
    let (c, _, e) = h.run([
        "send",
        &format!("@{a}"),
        "--name",
        &b,
        "--intent",
        "ack",
        "--reply-to",
        "42:BOXE",
        "--",
        "pong",
    ]);
    assert_eq!(c, 0, "ack send with remote reply-to failed: stderr={e}");

    let (_, ack_out, _) = h.run(["events", "--intent", "ack", "--last", "5", "--full"]);
    let ack: serde_json::Value = ack_out
        .lines()
        .find_map(|l| serde_json::from_str(l).ok())
        .expect("ack event present");

    assert_eq!(ack["data"]["intent"], "ack");
    assert_eq!(ack["data"]["reply_to"], "42:BOXE");
    assert_eq!(
        ack["data"]["reply_to_local"].as_i64(),
        Some(100),
        "reply_to_local must resolve to imported event id 100, not colliding local id 42; ack={ack}"
    );
}

#[test]
fn lifecycle_events_emitted_for_start_and_stop() {
    // Wiki contract (agent-lifecycle.md + event-model.md): start emits
    // life.started, stop emits life.stopped — filterable via --action.
    // events table is the lifecycle source of truth (see
    // feedback_events_are_source_of_truth memory).
    let h = Hcom::new();
    let a = h.start();

    let (c, started_out, _) = h.run([
        "events", "--action", "started", "--agent", &a, "--last", "5",
    ]);
    assert_eq!(c, 0);
    let started: Vec<serde_json::Value> = started_out
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert_eq!(
        started.len(),
        1,
        "expected 1 life.started for {a}, got: {started_out}"
    );
    assert_eq!(started[0]["instance"], a.as_str());
    assert_eq!(started[0]["data"]["action"], "started");

    let (cs, _, es) = h.run(["stop", &a]);
    assert_eq!(cs, 0, "stop failed: {es}");

    let (_, stopped_out, _) = h.run([
        "events", "--action", "stopped", "--agent", &a, "--last", "5",
    ]);
    let stopped: Vec<serde_json::Value> = stopped_out
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert_eq!(
        stopped.len(),
        1,
        "expected 1 life.stopped for {a}, got: {stopped_out}"
    );
    assert_eq!(stopped[0]["data"]["action"], "stopped");
    // Snapshot lives on the event but is streamlined out by default;
    // --full surfaces it. Rebind relies on it (see start_as_reclaims_stopped_identity).
    let (_, full_out, _) = h.run([
        "events", "--action", "stopped", "--agent", &a, "--last", "5", "--full",
    ]);
    let full: serde_json::Value = full_out
        .lines()
        .find_map(|l| serde_json::from_str(l).ok())
        .expect("stopped event under --full");
    assert!(
        full["data"]["snapshot"].is_object(),
        "stop must preserve snapshot for rebind; full={full_out}"
    );
}

#[test]
fn start_as_reclaims_stopped_identity() {
    // Wiki contract (identity.md §--as + hcom-start.md Path B): after stop,
    // `start --as <name>` rebinds the same name (no random reallocation).
    // Distinct from bare `start`, which would draw a fresh name.
    let h = Hcom::new();
    let a = h.start();

    let (cs, _, es) = h.run(["stop", &a]);
    assert_eq!(cs, 0, "stop failed: {es}");

    let (cr, stdout, stderr) = h.run(["start", "--as", &a]);
    assert_eq!(cr, 0, "start --as failed: stderr={stderr}");
    assert!(
        stdout.contains(&format!("[hcom:{a}]")),
        "reclaim marker missing; stdout={stdout}"
    );

    // Reclaimed instance is alive again under the same name.
    // (Reclaim is a quiet rebind: no new life.started event, just a logged
    // rebind.complete. The marker + a re-populated instances row is the
    // observable contract.)
    let (_, names_out, _) = h.run(["list", "--names"]);
    assert!(
        names_out.lines().any(|l| l.trim() == a),
        "list --names missing {a} after reclaim: {names_out}"
    );

    // And the stopped snapshot must still be on record — that's what made
    // the cursor-preserving rebind possible.
    let (_, full_out, _) = h.run([
        "events", "--action", "stopped", "--agent", &a, "--last", "5", "--full",
    ]);
    let snap_present = full_out
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .any(|v| v["data"]["snapshot"].is_object());
    assert!(snap_present, "stopped snapshot missing; full={full_out}");
}

#[test]
fn bigboss_send_bypasses_identity_gate() {
    // Wiki contract (messaging.md §@bigboss + reference_send_bigboss_flag memory):
    // `send -b` is sender-as-bigboss and bypasses the identity gate that
    // normally requires `--name` / a bound session. Sender_kind=external
    // distinguishes the message from instance-to-instance traffic.
    let h = Hcom::new();
    let recipient = h.start();

    // Note: no --name. -b is the sole identity signal.
    let (c, _, stderr) = h.run(["send", "-b", &format!("@{recipient}"), "--", "from above"]);
    assert_eq!(c, 0, "send -b must bypass identity gate; stderr={stderr}");
    assert!(
        !stderr.contains("identity not found"),
        "gate should not fire under -b; stderr={stderr}"
    );

    // --full bypasses streamlining so sender_kind is visible.
    let (_, events_out, _) = h.run([
        "events", "--type", "message", "--from", "bigboss", "--last", "5", "--full",
    ]);
    let msg: serde_json::Value = events_out
        .lines()
        .find_map(|l| serde_json::from_str(l).ok())
        .expect("bigboss message event");
    assert_eq!(msg["data"]["from"], "bigboss");
    assert_eq!(msg["data"]["text"], "from above");
    assert_eq!(
        msg["data"]["sender_kind"], "external",
        "bigboss must record as external sender; msg={msg}"
    );
}

#[test]
fn config_unknown_key_is_rejected() {
    let h = Hcom::new();
    let (code, _stdout, stderr) = h.run(["config", "no_such_key"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Unknown config key 'no_such_key'"),
        "stderr={stderr}"
    );

    // Setting a typo must not write it to config.toml either.
    let (code, _stdout, stderr) = h.run(["config", "timout", "60"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Did you mean: timeout?"), "stderr={stderr}");
}

#[test]
fn config_known_unset_key_reports_not_set() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["config", "hints"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("(not set)"), "stdout={stdout}");
}

#[test]
fn unknown_command_errors() {
    let h = Hcom::new();
    let (code, _stdout, stderr) = h.run(["nonsense-not-a-command"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty(), "expected error message on stderr");
}

#[test]
fn claude_clear_keeps_launch_name() {
    use std::io::Write;
    use std::process::Stdio;

    let h = Hcom::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let hook = |name: &str, payload: serde_json::Value| {
        let mut cmd = h.cmd();
        cmd.arg(name)
            .env("HCOM_LAUNCHED", "1")
            .env("HCOM_INSTANCE_NAME", "clear_keep")
            .env("HCOM_PROCESS_ID", "pid-clear")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn claude hook");
        child
            .stdin
            .take()
            .expect("hook stdin")
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().expect("wait claude hook");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{name} stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let session_of = || {
        let (code, stdout, stderr) = h.run(["list", "clear_keep", "--json"]);
        assert_eq!(code, 0, "stderr={stderr}");
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
        v["session_id"].as_str().map(str::to_string)
    };

    // Stand-in for the launcher: the process binding exists before Claude starts.
    let (code, _stdout, stderr) = h.run_as_process("pid-clear", ["start", "--as", "clear_keep"]);
    assert_eq!(code, 0, "stderr={stderr}");

    hook(
        "sessionstart",
        serde_json::json!({
            "session_id": "sess-before-clear",
            "transcript_path": transcript_path,
            "source": "startup",
            "hook_event_name": "SessionStart",
        }),
    );
    assert_eq!(session_of().as_deref(), Some("sess-before-clear"));

    // /clear: SessionEnd stops the row, then SessionStart runs in the same
    // live process with a new session id.
    hook(
        "sessionend",
        serde_json::json!({
            "session_id": "sess-before-clear",
            "transcript_path": transcript_path,
            "reason": "clear",
            "hook_event_name": "SessionEnd",
        }),
    );
    hook(
        "sessionstart",
        serde_json::json!({
            "session_id": "sess-after-clear",
            "transcript_path": transcript_path,
            "source": "clear",
            "hook_event_name": "SessionStart",
        }),
    );
    assert_eq!(session_of().as_deref(), Some("sess-after-clear"));
}

#[test]
fn antigravity_e2e_hook_dispatch() {
    let h = Hcom::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();

    // Spawn hcom start with HCOM_PROCESS_ID to register a process binding
    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("HCOM_PROCESS_ID", "pid-agy-123");
    let start_out = start_cmd.output().expect("failed to run hcom start");
    let me = support::parse_hcom_marker(&String::from_utf8_lossy(&start_out.stdout))
        .expect("no [hcom:NAME] marker");
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    conn.execute(
        "UPDATE instances SET tool = 'antigravity' WHERE name = ?1",
        [&me],
    )
    .expect("mark fixture as Antigravity");

    // 1. Pipe PreInvocation (session start) to gemini-sessionstart.
    // This will bind the session_id "sess-agy-1" to the active instance.
    let session_start_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
    });

    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.args(["gemini-sessionstart"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("HCOM_PROCESS_ID", "pid-agy-123");
    cmd.env(
        "HCOM_AGENT_INSTRUCTIONS_FALLBACK",
        "RULE: SOUL INSTRUCTIONS FALLBACK",
    );
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn hcom sessionstart");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin
            .write_all(
                serde_json::to_string(&session_start_payload)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
    let out = child
        .wait_with_output()
        .expect("failed to wait sessionstart");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let first_stdout = String::from_utf8_lossy(&out.stdout);
    let first: serde_json::Value =
        serde_json::from_str(first_stdout.trim()).expect("first sessionstart json");
    let first_context = first["injectSteps"][0]["ephemeralMessage"]
        .as_str()
        .expect("initial Antigravity bootstrap");
    assert!(first_context.contains("[HCOM SESSION]"));
    assert!(first_context.contains(&format!("--name {me}")));
    assert!(first_context.contains("RULE: SOUL INSTRUCTIONS FALLBACK"));

    // Verify session_id binding matches in the DB via hcom list --json
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("failed to parse list json");
    assert_eq!(v["session_id"].as_str(), Some("sess-agy-1"));

    // Antigravity fires this hook before every model invocation. Its ephemeral
    // bootstrap must be present after the one-shot name announcement too.
    let mut cmd = h.cmd();
    cmd.args(["gemini-sessionstart"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("HCOM_PROCESS_ID", "pid-agy-123");
    cmd.env(
        "HCOM_AGENT_INSTRUCTIONS_FALLBACK",
        "RULE: SOUL INSTRUCTIONS FALLBACK",
    );
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn repeated sessionstart");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin
            .write_all(
                serde_json::to_string(&session_start_payload)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
    let out = child
        .wait_with_output()
        .expect("failed to wait repeated sessionstart");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let repeated_stdout = String::from_utf8_lossy(&out.stdout);
    let repeated: serde_json::Value =
        serde_json::from_str(repeated_stdout.trim()).expect("repeated sessionstart json");
    let repeated_context = repeated["injectSteps"][0]["ephemeralMessage"]
        .as_str()
        .expect("recurring Antigravity bootstrap");
    assert!(repeated_context.contains("[HCOM SESSION]"));
    assert!(repeated_context.contains(&format!("--name {me}")));
    assert!(
        !repeated_context.contains("RULE: SOUL INSTRUCTIONS FALLBACK"),
        "recurring preinvocation hook must not repeat fallback instructions: {repeated_context}"
    );
    assert!(
        !repeated_context.contains("## HCOM AGENT INSTRUCTIONS — SYSTEM-PROMPT FALLBACK"),
        "recurring preinvocation hook must not repeat fallback header: {repeated_context}"
    );

    // 2. Now pipe PreToolUse to gemini-beforetool.
    // Since the session is bound, it should resolve the instance and execute successfully.
    let before_tool_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
        "toolCall": {
            "name": "run_command",
            "args": { "CommandLine": "echo hello", "Cwd": "/tmp" }
        }
    });

    let mut cmd = h.cmd();
    cmd.args(["gemini-beforetool"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("HCOM_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn hcom beforetool");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin
            .write_all(
                serde_json::to_string(&before_tool_payload)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait beforetool");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("beforetool json");
    assert_eq!(parsed, serde_json::json!({ "decision": "allow" }));

    // 3. AfterTool cannot inject context for Antigravity, so it must not ack delivery.
    // Send from bigboss: self-sends are never delivered, so they'd leave nothing pending.
    let (send_code, _, send_stderr) = h.run([
        "send",
        &format!("@{me}"),
        "-b",
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let after_tool_payload = serde_json::json!({
        "conversationId": "sess-agy-1",
        "transcriptPath": transcript_path,
        "toolCall": {
            "name": "run_command",
            "args": { "CommandLine": "echo done", "Cwd": "/tmp" }
        }
    });

    let mut cmd = h.cmd();
    cmd.args(["gemini-aftertool"]);
    cmd.env("ANTIGRAVITY_AGENT", "1");
    cmd.env("HCOM_PROCESS_ID", "pid-agy-123");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn hcom aftertool");
    {
        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin
            .write_all(
                serde_json::to_string(&after_tool_payload)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
    let out = child.wait_with_output().expect("failed to wait aftertool");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after_stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(after_stdout.trim()).expect("aftertool json");
    assert_eq!(parsed, serde_json::json!({}));
}

/// Pipe a JSON payload to a native cursor hook and return its parsed stdout.
///
/// Cursor hook command names route directly to `Tool::Cursor` (no shared-prefix
/// disambiguation like Antigravity's `ANTIGRAVITY_AGENT`), so the only env the
/// gate check needs is `HCOM_PROCESS_ID` to resolve the bound instance.
fn run_cursor_hook(
    h: &Hcom,
    hook: &str,
    process_id: &str,
    payload: &serde_json::Value,
) -> serde_json::Value {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.args([hook]);
    cmd.env("HCOM_PROCESS_ID", process_id);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    {
        let mut stdin = child.stdin.take().expect("open stdin");
        stdin
            .write_all(serde_json::to_string(payload).unwrap().as_bytes())
            .unwrap();
    }
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {hook}: {e}"));
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "{hook} stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// End-to-end cursor-agent native hook lifecycle over JSON-on-stdin.
///
/// Mirrors `antigravity_e2e_hook_dispatch`, but exercises cursor's real payload
/// shape (`conversation_id`/`tool_input`/`tool_output`) and its distinct
/// delivery contract: unlike Antigravity (whose aftertool cannot inject and
/// must return `{}`), cursor's `postToolUse` injects pending messages via
/// `additional_context` and acks delivery.
#[test]
fn cursor_e2e_hook_dispatch() {
    let h = Hcom::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-cur-123";
    let session_id = "sess-cur-1";

    // Register a process binding so the hooks can resolve an instance.
    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("HCOM_PROCESS_ID", pid);
    let start_out = start_cmd.output().expect("failed to run hcom start");
    let me = support::parse_hcom_marker(&String::from_utf8_lossy(&start_out.stdout))
        .expect("no [hcom:NAME] marker");

    // 1. sessionStart binds the conversation to the active instance. Cursor reads
    //    the id from `conversation_id` (snake_case, per the docs' common schema)
    //    and the handler always returns an `env` object.
    let session_start = run_cursor_hook(
        &h,
        "cursor-sessionstart",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "workspace_roots": ["/tmp"],
            "is_background_agent": false,
            "composer_mode": "agent",
        }),
    );
    assert!(
        session_start.get("env").is_some(),
        "sessionStart should emit env block: {session_start}"
    );

    // Binding is visible via list --json.
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["session_id"].as_str(), Some(session_id));

    // 2. beforeSubmitPrompt marks the instance active and must not block the
    //    prompt (`continue: true`).
    let before_submit = run_cursor_hook(
        &h,
        "cursor-beforesubmitprompt",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "prompt": "do a thing",
        }),
    );
    assert_eq!(before_submit, serde_json::json!({ "continue": true }));

    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. preToolUse records tool status and returns an empty object.
    let pre_tool = run_cursor_hook(
        &h,
        "cursor-pretooluse",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "Shell",
            "tool_input": { "command": "echo hello", "working_directory": "/tmp" },
        }),
    );
    assert_eq!(pre_tool, serde_json::json!({}));

    // 4. Queue a message, then postToolUse delivers it via additional_context.
    //    Send from an external sender (bigboss), not `me`: the DB delivery
    //    filter (`should_deliver_to`) drops any message whose `from` equals the
    //    receiver, so a self-addressed send would never be pending and the
    //    postToolUse assertion below would pass vacuously.
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let post_tool = run_cursor_hook(
        &h,
        "cursor-posttooluse",
        pid,
        &serde_json::json!({
            "conversation_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "Shell",
            "tool_input": { "command": "echo hello" },
            "tool_output": "{\"exitCode\":0,\"stdout\":\"hello\"}",
        }),
    );
    let injected = post_tool
        .get("additional_context")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("postToolUse should inject additional_context: {post_tool}"));
    assert!(
        injected.contains("ping"),
        "delivered context should carry the message text: {injected:?}"
    );
}

/// End-to-end GitHub Copilot CLI native hook lifecycle over JSON-on-stdin.
///
/// Mirrors `cursor_e2e_hook_dispatch` but exercises copilot's real payload shape
/// (`session_id`/`tool_name`/`tool_input`/`tool_result`, Claude-style `command`
/// hooks). Copilot's `SessionStart` returns `additionalContext`/`{}` (no `env`
/// block), `PostToolUse` injects pending messages via `additionalContext` and
/// acks delivery. Reuses `run_cursor_hook` — it is a generic "pipe JSON to a
/// native hook" runner, not cursor-specific.
#[test]
fn copilot_e2e_hook_dispatch() {
    let h = Hcom::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-cop-123";
    let session_id = "sess-cop-1";

    // Register a process binding so the hooks can resolve an instance.
    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("HCOM_PROCESS_ID", pid);
    let start_out = start_cmd.output().expect("failed to run hcom start");
    let me = support::parse_hcom_marker(&String::from_utf8_lossy(&start_out.stdout))
        .expect("no [hcom:NAME] marker");

    // 1. SessionStart binds the session to the active instance.
    let _ = run_cursor_hook(
        &h,
        "copilot-sessionstart",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "cwd": "/tmp",
        }),
    );

    // Binding is visible via list --json.
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["session_id"].as_str(), Some(session_id));

    // 2. UserPromptSubmit marks the instance active and returns an empty object.
    let prompt_submit = run_cursor_hook(
        &h,
        "copilot-userpromptsubmit",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "prompt": "do a thing",
        }),
    );
    assert_eq!(prompt_submit, serde_json::json!({}));

    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. PreToolUse records tool status and returns an empty object.
    let pre_tool = run_cursor_hook(
        &h,
        "copilot-pretooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "bash",
            "tool_input": { "command": "echo hello" },
        }),
    );
    assert_eq!(pre_tool, serde_json::json!({}));

    // 4. Queue a message from an external sender, then PostToolUse delivers it
    //    via additionalContext. (Self-addressed sends are dropped by the DB
    //    delivery filter, so the assertion below would pass vacuously.)
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let post_tool = run_cursor_hook(
        &h,
        "copilot-posttooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "tool_name": "bash",
            "tool_input": { "command": "echo hello" },
            "tool_result": { "text_result_for_llm": "hello" },
        }),
    );
    let injected = post_tool
        .get("additionalContext")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("PostToolUse should inject additionalContext: {post_tool}"));
    assert!(
        injected.contains("ping"),
        "delivered context should carry the message text: {injected:?}"
    );
}

/// Pipe a Qoder hook payload to `qoder-*` and return its stdout as JSON
/// (`{}` when the hook wrote nothing, as no-op Qoder hooks do).
fn run_qoder_hook(
    h: &Hcom,
    hook: &str,
    process_id: &str,
    payload: &serde_json::Value,
) -> serde_json::Value {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = h.cmd();
    cmd.arg(hook);
    cmd.env("HCOM_PROCESS_ID", process_id);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    {
        let mut stdin = child.stdin.take().expect("open stdin");
        stdin
            .write_all(serde_json::to_string(payload).unwrap().as_bytes())
            .unwrap();
    }
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {hook}: {e}"));
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "{hook} stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// End-to-end Qoder CLI hook lifecycle over JSON-on-stdin, with Qoder's
/// Claude-shaped payloads and `hookSpecificOutput` replies.
#[test]
fn qoder_e2e_hook_dispatch() {
    let h = Hcom::new();
    let pid = "pid-qod-123";
    let session_id = "sess-qod-1";
    let transcript_path = "/home/u/.qoder/projects/p/sess-qod-1.jsonl";

    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("HCOM_PROCESS_ID", pid);
    start_cmd.env("QODER_CLI", "1");
    let start_out = start_cmd.output().expect("failed to run hcom start");
    let me = support::parse_hcom_marker(&String::from_utf8_lossy(&start_out.stdout))
        .expect("no [hcom:NAME] marker");

    // 1. UserPromptSubmit with no prior SessionStart (untrusted folder): binds
    //    lazily and carries the bootstrap.
    let first = run_qoder_hook(
        &h,
        "qoder-userpromptsubmit",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "transcript_path": transcript_path,
            "cwd": "/tmp",
            "hook_event_name": "UserPromptSubmit",
            "prompt": "do a thing",
        }),
    );
    assert_eq!(
        first["hookSpecificOutput"]["hookEventName"], "UserPromptSubmit",
        "{first}"
    );
    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["session_id"].as_str(), Some(session_id));
    assert_eq!(v["status"].as_str(), Some("active"));

    // 2. PreToolUse records tool status and writes nothing.
    let pre = run_qoder_hook(
        &h,
        "qoder-pretooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "echo hello" },
        }),
    );
    assert_eq!(pre, serde_json::json!({}));

    // 3. A queued message is delivered by PostToolUse via additionalContext.
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let post = run_qoder_hook(
        &h,
        "qoder-posttooluse",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "echo hello" },
            "tool_response": "hello",
        }),
    );
    let injected = post["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap_or_else(|| panic!("PostToolUse should inject additionalContext: {post}"));
    assert!(injected.contains("ping"), "{injected:?}");
    assert_eq!(post["hookSpecificOutput"]["hookEventName"], "PostToolUse");

    // 3b. A failed tool call delivers too, answering under its own event name.
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "inform",
        "--",
        "after-failure",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let failed = run_qoder_hook(
        &h,
        "qoder-posttoolusefailure",
        pid,
        &serde_json::json!({
            "session_id": session_id,
            "tool_name": "Bash",
            "tool_input": { "command": "false" },
            "error": "Command exited with non-zero status code 1",
        }),
    );
    assert_eq!(
        failed["hookSpecificOutput"]["hookEventName"], "PostToolUseFailure",
        "{failed}"
    );
    assert!(
        failed["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap_or("")
            .contains("after-failure"),
        "{failed}"
    );

    // 4. A message queued during the turn blocks Stop with it as the reason.
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "request",
        "--",
        "pong",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");
    let stop = run_qoder_hook(
        &h,
        "qoder-stop",
        pid,
        &serde_json::json!({ "session_id": session_id, "stop_hook_active": false }),
    );
    assert_eq!(stop["decision"], "block", "{stop}");
    assert!(stop["reason"].as_str().unwrap_or("").contains("pong"));

    // 5. With nothing pending, Stop goes listening and writes nothing.
    let stop = run_qoder_hook(
        &h,
        "qoder-stop",
        pid,
        &serde_json::json!({ "session_id": session_id, "stop_hook_active": true }),
    );
    assert_eq!(stop, serde_json::json!({}));
    let (_, stdout, _) = h.run(["list", &me, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("listening"));

    // 6. SessionEnd for the bound session removes the instance; `clear` does not.
    let _ = run_qoder_hook(
        &h,
        "qoder-sessionend",
        pid,
        &serde_json::json!({ "session_id": session_id, "reason": "clear" }),
    );
    let (code, _, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "clear must not end the instance");
    let _ = run_qoder_hook(
        &h,
        "qoder-sessionend",
        pid,
        &serde_json::json!({ "session_id": session_id, "reason": "other" }),
    );
    let (code, _, _) = h.run(["list", &me, "--json"]);
    assert_ne!(
        code, 0,
        "SessionEnd for the bound session ends the instance"
    );
}

/// Pipe argv to a native argv-style hook and return its parsed stdout.
fn run_argv_hook(
    h: &Hcom,
    hook: &str,
    process_id: Option<&str>,
    args: &[&str],
) -> serde_json::Value {
    let mut cmd = h.cmd();
    cmd.arg(hook);
    cmd.args(args);
    if let Some(process_id) = process_id {
        cmd.env("HCOM_PROCESS_ID", process_id);
    }

    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {hook}: {e}"));
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "{hook} stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{hook} json: {e}\nstdout={stdout}"))
}

/// End-to-end Pi argv hook lifecycle.
///
/// Pi's extension invokes hcom with argv, not JSON stdin. This test mirrors the
/// native hook smoke tests above while staying hermetic: no real Pi process is
/// launched, only a fake process binding plus the `pi-*` hook commands.
#[test]
fn pi_e2e_hook_dispatch() {
    let h = Hcom::new();
    let transcript = tempfile::NamedTempFile::new().expect("temp transcript");
    let transcript_path = transcript.path().to_string_lossy().to_string();
    let pid = "pid-pi-123";
    let session_id = "sess-pi-1";

    // Register a process binding so pi-start can resolve an instance.
    let mut start_cmd = h.cmd();
    start_cmd.arg("start");
    start_cmd.env("HCOM_PROCESS_ID", pid);
    let start_out = start_cmd.output().expect("failed to run hcom start");
    let me = support::parse_hcom_marker(&String::from_utf8_lossy(&start_out.stdout))
        .expect("no [hcom:NAME] marker");

    // 1. pi-start binds the session and returns bootstrap context to the plugin.
    let cwd = h.root.path().to_string_lossy().to_string();
    let start = run_argv_hook(
        &h,
        "pi-start",
        Some(pid),
        &[
            "--session-id",
            session_id,
            "--transcript-path",
            &transcript_path,
            "--cwd",
            &cwd,
        ],
    );
    assert_eq!(start["name"].as_str(), Some(me.as_str()));
    assert_eq!(start["session_id"].as_str(), Some(session_id));
    assert!(
        start["bootstrap"]
            .as_str()
            .is_some_and(|text| text.contains(&format!("--name {me}"))),
        "pi-start should return bootstrap with the agent identity: {start}"
    );

    let (code, stdout, stderr) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["tool"].as_str(), Some("pi"));
    assert_eq!(v["session_id"].as_str(), Some(session_id));
    assert_eq!(
        v["transcript_path"].as_str(),
        Some(transcript_path.as_str())
    );
    assert_eq!(v["directory"].as_str(), Some(cwd.as_str()));

    // 2. pi-status marks active/listening transitions.
    let status = run_argv_hook(
        &h,
        "pi-status",
        Some(pid),
        &[
            "--name",
            &me,
            "--status",
            "active",
            "--context",
            "prompt",
            "--detail",
            "working",
        ],
    );
    assert_eq!(status, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run(["list", &me, "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    assert_eq!(v["status"].as_str(), Some("active"));

    // 3. pi-beforetool records tool status (observation only, never blocks).
    let before_tool = run_argv_hook(
        &h,
        "pi-beforetool",
        Some(pid),
        &[
            "--name",
            &me,
            "--tool",
            "bash",
            "--input-json",
            r#"{"command":"echo hello"}"#,
        ],
    );
    assert_eq!(before_tool, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run(["events", "--agent", &me, "--type", "status", "--last", "5"]);
    assert_eq!(code, 0);
    let tool_status: serde_json::Value = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .find(|event: &serde_json::Value| event["data"]["context"] == "tool:bash")
        .unwrap_or_else(|| panic!("tool:bash status event missing: {stdout}"));
    assert!(
        tool_status["data"]["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("echo hello")),
        "tool detail should include bash command: {tool_status}"
    );

    // 4. pi-read exposes pending messages and can ack the cursor.
    let (send_code, _, send_stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        &format!("@{me}"),
        "--intent",
        "request",
        "--",
        "ping",
    ]);
    assert_eq!(send_code, 0, "send stderr={send_stderr}");

    let check = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--check"]);
    assert_eq!(check, serde_json::json!(true));

    // Another launch (e.g. this one after stop + resume elsewhere) cannot read the inbox.
    let stale = run_argv_hook(&h, "pi-read", Some("pid-stale"), &["--name", &me]);
    assert!(stale.get("error").is_some(), "stale read: {stale}");

    let batch = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me]);
    assert_eq!(batch["has_more"], false, "pi-read batch: {batch}");
    let ping = batch["messages"]
        .as_array()
        .and_then(|items| items.iter().find(|m| m["message"] == "ping"))
        .unwrap_or_else(|| panic!("pi-read should return pending ping: {batch}"));
    let ping_id = ping["event_id"].as_i64().expect("event_id").to_string();

    let ack = run_argv_hook(
        &h,
        "pi-read",
        Some(pid),
        &["--name", &me, "--ack", "--up-to", &ping_id],
    );
    assert_eq!(ack["acked_to"].to_string(), ping_id);
    let check = run_argv_hook(&h, "pi-read", Some(pid), &["--name", &me, "--check"]);
    assert_eq!(check, serde_json::json!(false));

    // 5. pi-stop finalizes the session.
    let stop = run_argv_hook(
        &h,
        "pi-stop",
        Some(pid),
        &["--name", &me, "--reason", "done"],
    );
    assert_eq!(stop, serde_json::json!({ "ok": true }));
    let (code, stdout, _) = h.run([
        "events", "--agent", &me, "--action", "stopped", "--last", "5",
    ]);
    assert_eq!(code, 0);
    let stopped: serde_json::Value = stdout
        .lines()
        .find_map(|line| serde_json::from_str(line).ok())
        .unwrap_or_else(|| panic!("stopped event missing: {stdout}"));
    assert_eq!(stopped["data"]["action"].as_str(), Some("stopped"));
}

enum UnreadTiming {
    Preexisting,
    ArrivingAfterReadiness,
}

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.as_mut().unwrap().try_wait()
    }

    /// Wait for the child to exit up to `timeout`. Kills and reaps if the deadline is exceeded.
    /// Captures all stdout and stderr.
    fn wait_bounded(mut self, timeout: Duration) -> (i32, String, String) {
        let deadline = Instant::now() + timeout;
        let mut child = self.child.take().unwrap();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    if let Some(mut out) = child.stdout.take() {
                        use std::io::Read;
                        let _ = out.read_to_end(&mut stdout);
                    }
                    if let Some(mut err) = child.stderr.take() {
                        use std::io::Read;
                        let _ = err.read_to_end(&mut stderr);
                    }
                    return (
                        status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&stdout).into_owned(),
                        String::from_utf8_lossy(&stderr).into_owned(),
                    );
                }
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "events --wait CLI child process exceeded bounded deadline of {timeout:?}"
                    );
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("events --wait CLI child try_wait error: {e}");
                }
            }
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_events_wait_cli_oracle(timing: UnreadTiming, wait_secs: u64, expected_code: i32) {
    let h = Hcom::new();
    let me = h.start();
    let other = h.start();
    let db_path = h.hcom_dir.join("hcom.db");
    let conn = rusqlite::Connection::open(&db_path).expect("open test db");

    conn.execute("DELETE FROM events", []).unwrap();
    conn.execute(
        "UPDATE instances SET last_event_id = 0 WHERE name IN (?1, ?2)",
        rusqlite::params![me, other],
    )
    .unwrap();

    let mut send_code = None;
    if matches!(timing, UnreadTiming::Preexisting) {
        let (sc, _, _) = h.run(["send", &format!("@{me}"), "--name", &other, "--", "pre"]);
        send_code = Some(sc);
    }

    let initial_cursor: i64 = conn
        .query_row(
            "SELECT last_event_id FROM instances WHERE name = ?1",
            rusqlite::params![me],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let child = h
        .cmd()
        .args([
            "events",
            "--wait",
            &wait_secs.to_string(),
            "--sql",
            "data LIKE '%sol002_target%'",
            "--name",
            &me,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn events wait");
    let mut child = ChildGuard::new(child);

    let mut endpoint_ready = false;
    if matches!(timing, UnreadTiming::ArrivingAfterReadiness) {
        for _ in 0..100 {
            if let Ok(c) = rusqlite::Connection::open(&db_path)
                && c.query_row(
                    "SELECT 1 FROM notify_endpoints WHERE instance = ?1 AND kind = 'events_wait' LIMIT 1",
                    rusqlite::params![me],
                    |_| Ok(true),
                )
                .unwrap_or(false)
            {
                endpoint_ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            endpoint_ready,
            "waiter notify endpoint must be registered before sending arriving message"
        );

        let (sc, _, _) = h.run(["send", &format!("@{me}"), "--name", &other, "--", "arr"]);
        send_code = Some(sc);
    }

    std::thread::sleep(Duration::from_millis(300));
    let mid_wait_cursor: i64 = conn
        .query_row(
            "SELECT last_event_id FROM instances WHERE name = ?1",
            rusqlite::params![me],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let premature_exit = child.try_wait().unwrap_or(None);

    if expected_code == 0
        && premature_exit.is_none()
        && let Ok(c) = rusqlite::Connection::open(&db_path)
    {
        let data = serde_json::json!({"status": "active", "detail": "sol002_target"});
        let _ = c.execute(
            "INSERT INTO events (timestamp, type, instance, data) VALUES (datetime('now'), 'status', ?1, ?2)",
            rusqlite::params![me, serde_json::to_string(&data).unwrap()],
        );
        let port: Option<u16> = c
            .query_row(
                "SELECT port FROM notify_endpoints WHERE instance = ?1 AND kind = 'events_wait'",
                rusqlite::params![me],
                |r| r.get(0),
            )
            .ok();
        if let Some(p) = port {
            let _ = std::net::TcpStream::connect(("127.0.0.1", p));
        }
    }

    let deadline_secs = wait_secs + 2;
    let (code, stdout, stderr) = child.wait_bounded(Duration::from_secs(deadline_secs));

    let preview_pattern = format!("<hcom>{other} → {me}</hcom>");
    let preview_count = stdout.matches(&preview_pattern).count();

    let send_ok = send_code.is_none_or(|c| c == 0);
    let pending = premature_exit.is_none();
    // A timeout may legitimately finish before the mid-wait probe on a busy
    // runner. Its final exit code and unchanged cursor still reject a false
    // match. Success scenarios must remain pending until we insert the match.
    let wait_state_ok = pending
        || (expected_code == 1 && premature_exit.is_some_and(|status| status.code() == Some(1)));
    let oracle_passed = send_ok
        && wait_state_ok
        && mid_wait_cursor == initial_cursor
        && preview_count <= 1
        && code == expected_code;

    assert!(
        oracle_passed,
        "events --wait oracle failed (pending={pending}, premature_exit={premature_exit:?}, mid_cursor={mid_wait_cursor}, initial_cursor={initial_cursor}, preview_count={preview_count}, code={code}, expected_code={expected_code}, endpoint_ready={endpoint_ready}): stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn events_wait_cli_preexisting_unread_times_out_with_one() {
    run_events_wait_cli_oracle(UnreadTiming::Preexisting, 2, 1);
}

#[test]
fn events_wait_cli_arriving_unread_then_matching_status_exits_zero() {
    run_events_wait_cli_oracle(UnreadTiming::ArrivingAfterReadiness, 4, 0);
}

#[test]
fn commands_on_stopped_agent_explain_when_and_how_to_resume() {
    let h = Hcom::new();
    let me = h.start();
    let gone = h.start();
    let (cs, _, es) = h.run(["stop", &gone]);
    assert_eq!(cs, 0, "stop failed: {es}");

    // kill/stop of an already-stopped agent is a no-op success, not "not found".
    for cmd in ["kill", "stop"] {
        let (code, stdout, stderr) = h.run([cmd, &gone]);
        assert_eq!(code, 0, "{cmd}: stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(&format!("'{gone}' stopped ")),
            "{cmd}: stdout={stdout}"
        );
        assert!(
            stdout.contains(&format!("hcom r {gone}")),
            "{cmd}: stdout={stdout}"
        );
    }

    for args in [vec!["list", gone.as_str()], vec!["term", gone.as_str()]] {
        let (code, _stdout, stderr) = h.run(args.clone());
        assert_eq!(code, 1, "{args:?}");
        assert!(
            stderr.contains(&format!("'{gone}' stopped ")),
            "{args:?}: stderr={stderr}"
        );
    }

    let (code, _stdout, stderr) = h.run(["send", &format!("@{gone}"), "--name", &me, "--", "hi"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains(&format!("@{gone} stopped")) && stderr.contains(&format!("hcom r {gone}")),
        "stderr={stderr}"
    );
}

#[test]
fn unknown_agent_gets_typo_suggestion() {
    let h = Hcom::new();
    let me = h.start();
    let typo: String = {
        let mut c: Vec<char> = me.chars().collect();
        c.swap(1, 2);
        c.into_iter().collect()
    };
    if typo == me {
        return; // name with repeated letters; swap is a no-op
    }
    let (code, _stdout, stderr) = h.run(["kill", &typo]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains(&format!("No agent named '{typo}'"))
            && stderr.contains(&format!("Did you mean: {me}?")),
        "stderr={stderr}"
    );
}

#[test]
fn send_rejects_ambiguous_bare_word_and_self_target() {
    let h = Hcom::new();
    let me = h.start();
    let other = h.start();

    // A lone word without '@' or '--' must not silently broadcast.
    let (code, _stdout, stderr) = h.run(["send", &other, "--name", &me]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains(&format!("Did you mean @{other}?")),
        "stderr={stderr}"
    );

    let (code, _stdout, stderr) = h.run(["send", "hello", "--name", &me]);
    assert_eq!(code, 1);
    assert!(stderr.contains("hcom send -- hello"), "stderr={stderr}");

    let (code, _stdout, stderr) = h.run(["send", &format!("@{me}"), "--name", &me, "--", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("is you"), "stderr={stderr}");

    // bigboss:DEVICE routes to bigboss, so it is also a self-only target.
    let (code, _stdout, stderr) = h.run(["send", "-b", "@bigboss:ABCD", "--", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("is you"), "stderr={stderr}");
}

#[test]
fn unknown_command_and_tool_suggest_corrections() {
    let h = Hcom::new();
    let (code, _stdout, stderr) = h.run(["lst"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Did you mean: list?"), "stderr={stderr}");

    let (code, _stdout, stderr) = h.run(["1", "claud"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Unknown tool 'claud'") && stderr.contains("Did you mean: claude?"),
        "stderr={stderr}"
    );
}

#[test]
fn send_warns_when_name_disagrees_with_the_shell_identity() {
    let h = Hcom::new();
    let sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    // pid-sender's shell is bound to `sender`, but it sends under `other` —
    // exactly the shape of an identity that drifted after a rebind or recovery.
    let (code, stdout, stderr) =
        h.run_as_process("pid-sender", ["send", "--name", &other, "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains(&sender) && stderr.contains(&other),
        "expected a drift warning naming both identities, got stderr={stderr}"
    );
}

#[test]
fn send_is_quiet_when_name_matches_the_shell_identity() {
    let h = Hcom::new();
    let sender = h.start_with_process_id("pid-sender");

    let (code, stdout, stderr) =
        h.run_as_process("pid-sender", ["send", "--name", &sender, "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        !stderr.contains("warning"),
        "a matching --name must not warn, got stderr={stderr}"
    );
}

#[test]
fn send_with_from_skips_the_name_drift_warning() {
    let h = Hcom::new();
    let _sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    // With --from the sender is the external name, so --name disagreeing with
    // the shell's binding says nothing about who is sending.
    let (code, stdout, stderr) = h.run_as_process(
        "pid-sender",
        ["send", "--name", &other, "--from", "bigboss", "--", "hi"],
    );
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        !stderr.contains("warning"),
        "--from must suppress the drift warning, got stderr={stderr}"
    );
}

#[test]
fn from_in_message_text_does_not_suppress_the_name_drift_warning() {
    let h = Hcom::new();
    let sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    let (code, stdout, stderr) = h.run_as_process(
        "pid-sender",
        ["send", "--name", &other, "--", "use", "--from", "x"],
    );
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains(&sender) && stderr.contains(&other),
        "--from after -- is message text, got stderr={stderr}"
    );
}

#[test]
fn send_with_attached_from_skips_the_name_drift_warning() {
    let h = Hcom::new();
    let _sender = h.start_with_process_id("pid-sender");
    let other = h.start_with_process_id("pid-other");

    let (code, stdout, stderr) = h.run_as_process(
        "pid-sender",
        ["send", "--name", &other, "--from=bigboss", "--", "hi"],
    );
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        !stderr.contains("warning"),
        "--from=NAME must suppress the drift warning, got stderr={stderr}"
    );
}

#[test]
fn run_help_and_docs_flags_preserve_script_help() {
    let h = Hcom::new();
    let caller = h.start();
    for args in [
        vec!["run", "--help"],
        vec!["run", "-h", "--name", &caller],
        vec!["run", "docs", "--help", "--name", &caller],
    ] {
        let (code, stdout, stderr) = h.run(&args);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        assert!(stdout.contains("hcom run <name>"), "{stdout}");
        assert!(!stdout.contains("# Creating Custom Scripts"), "{stdout}");
    }
    let (code, stdout, stderr) = h.run(["run", "debate", "--help", "--name", &caller]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("Usage: hcom run debate [OPTIONS] TOPIC"),
        "{stdout}"
    );
    let (code, stdout, stderr) = h.run(["run", "docs", "--scripts", "--name", &caller]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("# Creating Custom Scripts"), "{stdout}");
    assert!(!stdout.contains("# CLI Reference"), "{stdout}");
    let (code, stdout, stderr) = h.run(["run", "docs", "--scrpits", "--name", &caller]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("Unknown docs option: --scrpits"),
        "{stderr}"
    );
}

#[test]
fn reset_and_config_help_describe_preserved_state() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["reset", "--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("open TUI reconnects"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["config", "--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("existing HCOM_DIR/env survives"),
        "stdout={stdout}"
    );
}

#[test]
fn herdr_autostart_config_and_env() {
    let h = Hcom::new();
    let (code, stdout, _) = h.run(["config", "herdr_autostart"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "true");

    let (code, _, _) = h.run(["config", "herdr_autostart", "false"]);
    assert_eq!(code, 0);

    let (code, stdout, _) = h.run(["config", "herdr_autostart"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "false");
}

#[test]
fn agent_help_lists_catalog_layers() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["agent", "--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.starts_with("Usage:"), "stdout={stdout}");
    assert!(stdout.contains("agents.json"), "stdout={stdout}");
    assert!(stdout.contains(".hcom/agents.json"), "stdout={stdout}");
    assert!(stdout.contains("agents/<name>/SOUL.md"), "stdout={stdout}");
    assert!(
        stdout.contains(".claude/skills -> ../skills"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("system_prompt_file"), "stdout={stdout}");
    assert!(stdout.contains("cli_overrides"), "stdout={stdout}");
    assert!(
        stdout.contains("agy and antigravity use --add-dir"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("DIPPY_POLICY_CWD"), "stdout={stdout}");
    assert!(stdout.contains("--as <name>"), "stdout={stdout}");
    assert!(stdout.contains("@<group>"), "stdout={stdout}");
    assert!(stdout.contains("\"groups\""), "stdout={stdout}");
    assert!(stdout.contains("\"roaming\": true"), "stdout={stdout}");
    assert!(
        stdout.contains("<name>_<project>_<hash>"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("roots with the same basename differ"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("DIPPY_CONFIG_ONLY"), "stdout={stdout}");
    assert!(stdout.contains("--all"), "stdout={stdout}");
    assert!(stdout.contains("--local"), "stdout={stdout}");
    assert!(stdout.contains("--for-agents"), "stdout={stdout}");
    assert!(stdout.contains("--for-humans"), "stdout={stdout}");
    assert!(stdout.contains("hcom agent list"), "stdout={stdout}");
    assert!(stdout.contains("--continue"), "stdout={stdout}");
    assert!(stdout.contains("--last <N>"), "stdout={stdout}");
    assert!(
        stdout.contains("regardless of launch directory"),
        "stdout={stdout}"
    );
    for layer in [
        "1. built-in defaults",
        "2. \"defaults\" in ~/.hcom/agents.json",
        "3. matching catalog \"defaults\"",
        "4. the named agent entry",
        "5. global cli_overrides",
        "6. the matching tools.<effective-cli> profile",
        "7. command-line flags",
    ] {
        assert!(stdout.contains(layer), "missing {layer}: {stdout}");
    }
    assert!(
        stdout.contains("system_prompt replaces rather than appends"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("explicit empty string clears it"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Imports are recursive and apply before"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("not a parent agent's location"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("hcom agent ls"), "stdout={stdout}");
}

#[test]
fn agent_list_without_catalog_points_at_the_global_file() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["agent", "list", "--no-project"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("No agents defined"), "stdout={stdout}");
    assert!(stdout.contains("agents.json"), "stdout={stdout}");
}

#[test]
fn agent_bundle_is_discovered_across_nested_git_roots_and_composes_prompt() {
    let h = Hcom::new();
    let project = h.root_path().join("multi-repo");
    let nested = project.join("services/api");
    let bundle = project.join(".hcom/agents/reviewer");
    std::fs::create_dir_all(nested.join(".git")).expect("create nested git marker");
    std::fs::create_dir_all(&bundle).expect("create agent bundle");
    std::fs::write(bundle.join("SOUL.md"), "Improve this file when you learn.")
        .expect("write agent instructions");
    std::fs::write(
        project.join(".hcom/agents.json"),
        r#"{"agents":{"reviewer":{"cli":"claude","dir":".","system_prompt":"Fixed identity."}}}"#,
    )
    .expect("write project catalog");

    let output = h
        .cmd()
        .current_dir(&nested)
        .args(["agent", "show", "reviewer"])
        .output()
        .expect("show bundle agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&format!("dir:       {}", project.display())),
        "project-relative dir must use the parent of .hcom: {stdout}"
    );
    assert!(stdout.contains(&format!("bundle:    {}", bundle.display())));
    assert!(stdout.contains("Fixed identity."), "stdout={stdout}");
    assert!(
        stdout.contains("Improve this file when you learn."),
        "stdout={stdout}"
    );
    assert!(
        stdout.find("Fixed identity.") < stdout.find("Improve this file when you learn."),
        "fixed prompt must precede SOUL.md: {stdout}"
    );

    let json_output = h
        .cmd()
        .current_dir(&nested)
        .args(["agent", "list", "--json"])
        .output()
        .expect("list bundle agent");
    let rows: serde_json::Value = serde_json::from_slice(&json_output.stdout).expect("agent JSON");
    assert_eq!(rows[0]["agent_dir"], bundle.to_string_lossy().as_ref());
    assert_eq!(
        rows[0]["instructions"],
        bundle.join("SOUL.md").to_string_lossy().as_ref()
    );
}

#[test]
fn nested_project_catalog_sees_enclosing_project_agents_and_overrides_them() {
    let h = Hcom::new();
    let outer = h.root_path().join("outer");
    let inner = outer.join("repos/inner");
    std::fs::create_dir_all(outer.join(".hcom")).expect("create outer catalog dir");
    std::fs::create_dir_all(inner.join(".hcom")).expect("create inner catalog dir");
    std::fs::write(
        outer.join(".hcom/agents.json"),
        r#"{"defaults":{"model":"opus"},"agents":{"outer_only":{"cli":"claude","dir":"."},"shared":{"cli":"claude","dir":".","description":"from outer"}}}"#,
    )
    .expect("write outer catalog");
    std::fs::write(
        inner.join(".hcom/agents.json"),
        r#"{"agents":{"inner_only":{"cli":"claude","dir":"."},"shared":{"cli":"claude","dir":".","description":"from inner"}}}"#,
    )
    .expect("write inner catalog");

    let output = h
        .cmd()
        .current_dir(&inner)
        .args(["agent", "list", "--json"])
        .output()
        .expect("list nested catalogs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr={stderr}");
    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).expect("agent JSON");
    let rows = rows.as_array().expect("array");
    let by_name = |name: &str| {
        rows.iter()
            .find(|row| row["name"] == name)
            .unwrap_or_else(|| panic!("{name} missing from {rows:?}"))
    };
    assert_eq!(
        by_name("outer_only")["dir"],
        outer.to_string_lossy().as_ref()
    );
    assert_eq!(
        by_name("inner_only")["dir"],
        inner.to_string_lossy().as_ref()
    );
    assert_eq!(
        by_name("inner_only")["model"],
        "opus",
        "an agent of a nested project inherits the enclosing project's defaults"
    );
    let shared = by_name("shared");
    assert_eq!(shared["description"], "from inner");
    assert_eq!(shared["dir"], inner.to_string_lossy().as_ref());

    let outer_output = h
        .cmd()
        .current_dir(&outer)
        .args(["agent", "list", "--names"])
        .output()
        .expect("list outer catalog");
    let outer_names = String::from_utf8_lossy(&outer_output.stdout);
    assert!(
        !outer_names.contains("inner_only"),
        "a nested project must stay private to itself: {outer_names}"
    );
}

#[test]
fn agent_ignores_legacy_agents_md_only_bundle() {
    let h = Hcom::new();
    let bundle = h.path().join("agents/legacy");
    std::fs::create_dir_all(&bundle).expect("create legacy bundle");
    std::fs::write(bundle.join("AGENTS.md"), "Legacy instructions.")
        .expect("write legacy instructions");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"known":{"cli":"claude"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "list", "--no-project", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let rows: serde_json::Value = serde_json::from_str(&stdout).expect("agent JSON");
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "stdout={stdout}");
    assert_eq!(rows[0]["name"], "known", "stdout={stdout}");
}

#[test]
fn project_bundle_fully_shadows_same_named_global_agent() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"cli":"codex","dir":"/global","model":"global-model"}}}"#,
    )
    .expect("write global catalog");
    let project = h.root_path().join("project");
    let bundle = project.join(".hcom/agents/reviewer");
    std::fs::create_dir_all(&bundle).expect("create project bundle");
    std::fs::write(bundle.join("SOUL.md"), "Project reviewer.").expect("write instructions");

    let output = h
        .cmd()
        .current_dir(&project)
        .args(["agent", "show", "reviewer"])
        .output()
        .expect("show project agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("cli:       claude"), "stdout={stdout}");
    assert!(!stdout.contains("/global"), "stdout={stdout}");
    assert!(!stdout.contains("global-model"), "stdout={stdout}");
    assert!(stdout.contains("Project reviewer."), "stdout={stdout}");
}

#[test]
fn project_agent_resolves_identically_inside_and_outside_project() {
    let h = Hcom::new();
    let project = h.root_path().join("wdt");
    let project_catalog = project.join(".hcom/agents.json");
    std::fs::create_dir_all(project_catalog.parent().unwrap()).expect("create project catalog dir");
    std::fs::write(
        &project_catalog,
        r#"{"defaults":{"cli":"codex","system_prompt":"Project instructions."},
            "agents":{"wdt_main":{"dir":".","model":"shared",
                "tools":{"codex":{"model":"profile-model"}}}}}"#,
    )
    .expect("write project catalog");
    std::fs::write(
        h.path().join("agents.json"),
        serde_json::json!({
            "defaults": {
                "terminal": "herdr",
                "system_prompt": "Global instructions."
            },
            "imports": [{
                "from": project_catalog,
                "agents": ["wdt_main"]
            }]
        })
        .to_string(),
    )
    .expect("write global catalog");

    let outside = h
        .cmd()
        .args(["agent", "show", "wdt_main"])
        .output()
        .expect("show imported project agent outside project");
    let inside = h
        .cmd()
        .current_dir(&project)
        .args(["agent", "show", "wdt_main"])
        .output()
        .expect("show project agent inside project");
    let outside_stdout = String::from_utf8_lossy(&outside.stdout);
    let outside_stderr = String::from_utf8_lossy(&outside.stderr);
    let inside_stdout = String::from_utf8_lossy(&inside.stdout);
    let inside_stderr = String::from_utf8_lossy(&inside.stderr);

    assert!(
        outside.status.success(),
        "stdout={outside_stdout} stderr={outside_stderr}"
    );
    assert!(
        inside.status.success(),
        "stdout={inside_stdout} stderr={inside_stderr}"
    );
    let inside_command = inside_stdout
        .split_once("\ncommand:\n")
        .expect("inside show command")
        .1;
    let outside_command = outside_stdout
        .split_once("\ncommand:\n")
        .expect("outside show command")
        .1;
    assert_eq!(inside_command, outside_command);
    assert_eq!(
        inside_stdout
            .lines()
            .find(|line| line.starts_with("terminal:")),
        outside_stdout
            .lines()
            .find(|line| line.starts_with("terminal:"))
    );
    assert!(
        inside_stdout.contains("terminal:  preset herdr"),
        "global catalog defaults must apply: {inside_stdout}"
    );
    assert!(!inside_stdout.contains("(hcom config default)"));
    assert!(inside_stdout.contains("Project instructions."));
    assert!(!inside_stdout.contains("Global instructions."));
    assert!(inside_stdout.contains("model:     profile-model"));
}

#[test]
fn legacy_project_catalog_is_ignored_and_edit_creates_dot_hcom_catalog() {
    let h = Hcom::new();
    let project = h.root_path().join("legacy");
    std::fs::create_dir_all(&project).expect("create project");
    std::fs::write(
        project.join(".hcom-agents.json"),
        r#"{"agents":{"legacy_agent":{}}}"#,
    )
    .expect("write legacy catalog");

    let output = h
        .cmd()
        .current_dir(&project)
        .args(["agent", "list", "--names"])
        .output()
        .expect("list without project scope");
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("legacy_agent"));

    let edit = h
        .cmd()
        .current_dir(&project)
        .env("EDITOR", "true")
        .args(["agent", "edit", "--project"])
        .output()
        .expect("create project catalog");
    assert!(
        edit.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&edit.stderr)
    );
    assert!(project.join(".hcom/agents.json").is_file());
}

#[test]
fn additive_catalog_env_keeps_global_agents_and_imports_selected_project_agents() {
    let h = Hcom::new();
    let project = h.root_path().join("wdt");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project dir");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"global_main":{"dir":"/tmp"}}}"#,
    )
    .expect("write global catalog");
    std::fs::write(
        project.join(".hcom/agents.json"),
        r#"{"agents":{"wdt_main":{"dir":"."},"wdt_private":{"dir":"."}}}"#,
    )
    .expect("write project catalog");
    let overlay = h.root_path().join("hermes-agents.json");
    std::fs::write(
        &overlay,
        format!(
            r#"{{"imports":[{{"from":{},"agents":["wdt_main"]}}]}}"#,
            serde_json::to_string(&project.join(".hcom/agents.json")).unwrap()
        ),
    )
    .expect("write overlay catalog");

    let output = h
        .cmd()
        .env("HCOM_AGENT_CATALOGS", &overlay)
        .args(["agent", "list", "--names"])
        .output()
        .expect("run hcom agent list");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.lines().any(|line| line == "global_main"),
        "stdout={stdout}"
    );
    assert!(
        stdout.lines().any(|line| line == "wdt_main"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("wdt_private"), "stdout={stdout}");
}

#[test]
fn agent_list_all_includes_agents_hidden_by_recursive_selective_imports() {
    let h = Hcom::new();
    let leaf = h.path().join("leaf-agents.json");
    std::fs::write(
        &leaf,
        r#"{"agents":{
            "leaf_public":{"dir":"/tmp","cli":"claude"},
            "leaf_private":{"dir":"/tmp","cli":"gemini",
                "tools":{"gemini":{"model":"gemini-2.5-pro"}}}
        }}"#,
    )
    .expect("write leaf catalog");
    let middle = h.path().join("middle-agents.json");
    std::fs::write(
        &middle,
        format!(
            r#"{{"imports":[{{"from":{},"agents":["leaf_public"]}}],
                "agents":{{
                    "middle_public":{{"dir":"/tmp","cli":"codex"}},
                    "middle_private":{{"dir":"/tmp","cli":"claude"}}
                }}}}"#,
            serde_json::to_string(&leaf).unwrap()
        ),
    )
    .expect("write middle catalog");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["middle_public"]}}]}}"#,
            serde_json::to_string(&middle).unwrap()
        ),
    )
    .expect("write root catalog");

    let (code, visible, stderr) = h.run(["agent", "list", "--names", "--no-project"]);
    assert_eq!(code, 0, "stdout={visible} stderr={stderr}");
    assert_eq!(visible.trim(), "middle_public");

    let (code, all_names, stderr) = h.run(["agent", "list", "--all", "--names", "--no-project"]);
    assert_eq!(code, 0, "stdout={all_names} stderr={stderr}");
    assert_eq!(
        all_names.lines().collect::<Vec<_>>(),
        [
            "leaf_private",
            "leaf_public",
            "middle_private",
            "middle_public"
        ]
    );

    let (code, table, stderr) = h.run(["agent", "list", "--all", "--for-humans", "--no-project"]);
    assert_eq!(code, 0, "stdout={table} stderr={stderr}");
    assert!(table.starts_with("NAME"));
    assert!(table.contains("MODEL"));
    assert!(table.lines().any(|line| line.starts_with("leaf_private ")));
    assert!(table.contains("gemini-2.5-pro"));

    let (code, json, stderr) = h.run(["agent", "list", "--all", "--json", "--no-project"]);
    assert_eq!(code, 0, "stdout={json} stderr={stderr}");
    let entries: Vec<serde_json::Value> = serde_json::from_str(&json).expect("list JSON");
    let leaf_private = entries
        .iter()
        .find(|entry| entry["name"] == "leaf_private")
        .expect("hidden leaf agent in JSON");
    assert_eq!(leaf_private["cli"], "gemini");
    assert_eq!(leaf_private["model"], "gemini-2.5-pro");
    assert!(
        leaf_private["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("import:"))
    );
}

#[test]
fn agent_list_local_includes_only_project_agents_and_their_imports() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"global_agent":{"dir":"/tmp"}}}"#,
    )
    .expect("write global catalog");

    let project = h.root_path().join("project");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project catalog dir");
    let imported = project.join("included.json");
    std::fs::write(
        &imported,
        r#"{"agents":{
            "included":{"dir":"/tmp","groups":["local_group"]},
            "hidden":{"dir":"/tmp","groups":["hidden_group"]}
        }}"#,
    )
    .expect("write imported catalog");
    std::fs::write(
        project.join(".hcom/agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["included"]}}],
                "agents":{{"direct":{{"dir":".","groups":["local_group"]}}}}}}"#,
            serde_json::to_string(&imported).unwrap()
        ),
    )
    .expect("write project catalog");

    let run = |args: &[&str]| {
        let output = h
            .cmd()
            .current_dir(&project)
            .args(args)
            .output()
            .expect("list local agents");
        (
            output.status.code().unwrap_or(1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    let (code, names, stderr) = run(&["agent", "list", "--local", "--names"]);
    assert_eq!(code, 0, "stdout={names} stderr={stderr}");
    assert_eq!(names.lines().collect::<Vec<_>>(), ["direct", "included"]);

    let (code, groups, stderr) = run(&["agent", "list", "--local", "--groups"]);
    assert_eq!(code, 0, "stdout={groups} stderr={stderr}");
    assert_eq!(groups.trim(), "@hidden_group\n@local_group");

    let (code, all, stderr) = run(&["agent", "list", "--local", "--all", "--names"]);
    assert_eq!(code, 0, "stdout={all} stderr={stderr}");
    assert_eq!(
        all.lines().collect::<Vec<_>>(),
        ["direct", "hidden", "included"]
    );
}

#[test]
fn imported_agent_config_overrides_global_defaults() {
    let h = Hcom::new();
    let project = h.root_path().join("wdt");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project dir");
    std::fs::write(
        project.join(".hcom/agents.json"),
        r#"{"defaults":{"cli":"claude"},"agents":{"wdt_main":{"dir":".","cli":"claude"}}}"#,
    )
    .expect("write project catalog");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"defaults":{{"cli":"codex"}},"imports":[{{"from":{},"agents":["wdt_main"]}}]}}"#,
            serde_json::to_string(&project.join(".hcom/agents.json")).unwrap()
        ),
    )
    .expect("write global catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "wdt_main"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.lines().any(|line| line == "cli:       claude"),
        "stdout={stdout}"
    );
    let sources = stdout
        .lines()
        .find_map(|line| line.strip_prefix("sources:   "))
        .expect("catalog sources in show output");
    for source in sources.split(", ") {
        assert!(
            std::path::Path::new(source).starts_with(h.root_path()),
            "fixture leaked catalog source outside {}: {source}\nstdout={stdout}",
            h.root_path().display()
        );
    }
}

#[test]
fn agent_list_for_agents_shows_only_names_and_descriptions() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "described":{"dir":"/tmp","cli":"codex","description":"backend API service, Go"},
            "plain":{"dir":"/tmp"}
        }}"#,
    )
    .expect("write catalog");

    let (code, brief, stderr) = h.run(["agent", "list", "--for-agents", "--no-project"]);
    assert_eq!(code, 0, "stdout={brief} stderr={stderr}");
    assert_eq!(
        brief.lines().collect::<Vec<_>>(),
        ["described  backend API service, Go", "plain      -"]
    );
    assert!(!brief.contains("NAME"), "stdout={brief}");
    assert!(!brief.contains("codex"), "stdout={brief}");
    assert!(!brief.contains("/tmp"), "stdout={brief}");

    // Test output is not a terminal, so the default matches --for-agents.
    let (code, default, stderr) = h.run(["agent", "list", "--no-project"]);
    assert_eq!(code, 0, "stdout={default} stderr={stderr}");
    assert_eq!(default, brief);

    let (code, json, stderr) = h.run(["agent", "list", "--json", "--no-project"]);
    assert_eq!(code, 0, "stdout={json} stderr={stderr}");
    let entries: Vec<serde_json::Value> = serde_json::from_str(&json).expect("list JSON");
    assert_eq!(entries[0]["description"], "backend API service, Go");
    assert!(entries[1]["description"].is_null(), "stdout={json}");

    let (code, show, stderr) = h.run(["agent", "show", "described", "--no-project"]);
    assert_eq!(code, 0, "stdout={show} stderr={stderr}");
    assert!(
        show.lines()
            .any(|line| line == "description: backend API service, Go"),
        "stdout={show}"
    );
}

#[test]
fn agent_unknown_name_suggests_a_close_match() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"wdt_main":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let (code, _stdout, stderr) = h.run(["agent", "wdt_mian"]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("unknown agent 'wdt_mian'"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains("did you mean 'wdt_main'"),
        "stderr={stderr}"
    );
}

/// A selective import hides a project's other agents on purpose. Someone
/// outside must not learn from an error that they exist, where they live, or
/// how to pull them in — the scope note says only that scope is directional.
#[test]
fn agent_unknown_name_keeps_an_import_filtered_entry_private() {
    let h = Hcom::new();
    let project = h.root_path().join("proj");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project catalog dir");
    std::fs::write(
        project.join(".hcom").join("agents.json"),
        r#"{"defaults":{"cli":"codex"},"agents":{"p_main":{"dir":"/tmp"},"p_two":{"dir":"/tmp"}}}"#,
    )
    .expect("write project catalog");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["p_main"]}}],"agents":{{"glob":{{"dir":"/tmp","cli":"codex"}}}}}}"#,
            serde_json::to_string(&project.join(".hcom").join("agents.json").to_string_lossy())
                .expect("encode path")
        ),
    )
    .expect("write global catalog");

    let (code, _stdout, stderr) = h.run(["agent", "show", "p_two"]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(stderr.contains("unknown agent 'p_two'"), "stderr={stderr}");
    assert!(
        stderr.contains("Catalog scope depends on the directory"),
        "stderr={stderr}"
    );
    assert!(
        !stderr.contains(
            &project
                .join(".hcom")
                .join("agents.json")
                .display()
                .to_string()
        ),
        "leaked the hidden catalog's path: stderr={stderr}"
    );
    assert!(
        !stderr.contains("is defined in"),
        "leaked that the agent exists: stderr={stderr}"
    );

    let (code, stdout, stderr) = h.run(["agent", "show", "p_main"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
}

/// A name no reachable catalog defines still needs the scope note: the user
/// cannot tell "misspelled" from "the catalog holding it is not in scope here".
#[test]
fn agent_unknown_name_explains_catalog_scope() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"glob":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let (code, _stdout, stderr) = h.run(["agent", "show", "absent_one"]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("Catalog scope depends on the directory"),
        "stderr={stderr}"
    );
}

#[test]
fn agent_cli_overrides_apply_to_projects_and_select_target_profile() {
    let h = Hcom::new();
    let project = h.root_path().join("project/.hcom");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{
        "cli_overrides":{"codex":"claude","claude":"gemini"},
        "imports":[{"from":"../project/.hcom/agents.json"}],
        "agents":{"global":{"cli":"codex"},"unchanged":{"cli":"gemini"}}
    }"#,
    )
    .unwrap();
    std::fs::write(
        project.join("agents.json"),
        r#"{
        "agents":{"solo":{"cli":"codex","groups":["review"],
            "tools":{"codex":{"model":"source-model","args":["--source-flag"]},
                     "claude":{"model":"target-model","args":["--target-flag"]}}}}
    }"#,
    )
    .unwrap();
    let catalog = project.join("agents.json");
    let catalog = catalog.to_str().unwrap();
    for args in [
        vec!["agent", "solo", "--catalog", catalog, "--dry-run"],
        vec!["agent", "solo", "--dry-run"],
        vec!["agent", "@review", "--catalog", catalog, "--dry-run"],
    ] {
        let (code, stdout, stderr) = h.run(args);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
        assert!(stdout.contains("claude --as solo"), "stdout={stdout}");
        assert!(stdout.contains("target-model"), "stdout={stdout}");
        assert!(stdout.contains("--target-flag"), "stdout={stdout}");
        assert!(!stdout.contains("source-model"), "stdout={stdout}");
        assert!(!stdout.contains("--source-flag"), "stdout={stdout}");
    }
    let (code, stdout, stderr) = h.run([
        "agent",
        "solo",
        "--catalog",
        catalog,
        "--cli",
        "codex",
        "--dry-run",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
    assert!(stdout.contains("source-model"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["agent", "list", "--catalog", catalog, "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let entries: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let solo = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "solo")
        .unwrap();
    assert_eq!(solo["cli"], "claude");
    assert_eq!(solo["model"], "target-model");
    for (name, tool) in [("global", "claude"), ("unchanged", "gemini")] {
        let (code, stdout, stderr) = h.run(["agent", name, "--dry-run"]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(&format!("{tool} --as {name}")),
            "stdout={stdout}"
        );
    }
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"imports":[{"from":"../project/.hcom/agents.json"}]}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
}

#[test]
fn agent_cli_overrides_support_aliases_and_default_cli() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{
        "cli_overrides":{"agy":"claude","claude":"codex"},
        "agents":{"alias":{"cli":"antigravity"},"default":{}}
    }"#,
    )
    .unwrap();
    for (name, tool) in [("alias", "claude"), ("default", "codex")] {
        let (code, stdout, stderr) = h.run(["agent", name, "--dry-run"]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(&format!("{tool} --as {name}")),
            "stdout={stdout}"
        );
    }
    let (code, stdout, stderr) = h.run(["agent", "default", "--tool", "claude", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("claude --as default"), "stdout={stdout}");
}

/// An override onto `agy` must pick the `agy` tool profile: resolving it to
/// `antigravity` dropped the profile and launched without its permission args.
#[test]
fn agent_cli_override_to_agy_keeps_agy_tool_profile() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{
        "cli_overrides":{"codex":"agy"},
        "defaults":{"tools":{"agy":{"args":["--dangerously-skip-permissions"]}}},
        "agents":{"reviewer":{"cli":"codex"}}
    }"#,
    )
    .unwrap();
    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("--dangerously-skip-permissions"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("agy --as reviewer"), "stdout={stdout}");
}

#[test]
fn agent_cli_overrides_reject_invalid_tools_and_non_global_catalogs() {
    let h = Hcom::new();
    for overrides in [
        r#"{"codex":"typo"}"#,
        r#"{"typo":"claude"}"#,
        r#"{"adhoc":"claude"}"#,
        r#"{"codex":"adhoc"}"#,
        r#"{"agy":"claude","antigravity":"codex"}"#,
    ] {
        std::fs::write(
            h.path().join("agents.json"),
            format!(r#"{{"cli_overrides":{overrides},"agents":{{"solo":{{}}}}}}"#),
        )
        .unwrap();
        let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
        assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
        assert!(stderr.contains("cli_overrides"), "stderr={stderr}");
    }
    std::fs::write(h.path().join("agents.json"), "{}").unwrap();
    let local = h.root_path().join("local.json");
    std::fs::write(
        &local,
        r#"{"cli_overrides":{"codex":"claude"},"agents":{"solo":{}}}"#,
    )
    .unwrap();
    let (code, stdout, stderr) = h.run([
        "agent",
        "solo",
        "--catalog",
        local.to_str().unwrap(),
        "--dry-run",
    ]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("cli_overrides is only supported in the global catalog"),
        "stderr={stderr}"
    );
}

#[test]
fn agent_dry_run_renders_the_hcom_command_without_launching() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"defaults":{"cli":"claude"},
            "agents":{"solo":{"dir":"/tmp","cli":"codex","terminal":"wezterm-tab",
                              "env":{"AWS_PROFILE":"wdt"},"args":["--from-catalog"]}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--model", "gpt-5", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("AWS_PROFILE=wdt"), "stdout={stdout}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
    assert!(stdout.contains("--dir /tmp"), "stdout={stdout}");
    assert!(stdout.contains("--terminal wezterm-tab"), "stdout={stdout}");
    assert!(stdout.contains("--model gpt-5"), "stdout={stdout}");
    assert!(stdout.contains("--from-catalog"), "stdout={stdout}");

    // Nothing was launched.
    let (code, list, _stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(list.trim(), "[]", "dry-run must not create an instance");
}

#[test]
fn agent_dry_run_exports_the_catalog_name_separately_from_the_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"codex","terminal":"wezterm-tab"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev2", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("HCOM_AGENT_NAME=reviewer"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("codex --as rev2"), "stdout={stdout}");
}

#[test]
fn agent_dry_run_inherits_specified_env_vars() {
    let h = Hcom::new();
    let test_var = "CUSTOM_SSH_AUTH_SOCK";
    let test_val = "/tmp/custom_ssh_agent.sock";
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"defaults":{{"cli":"claude","inherit_env":["{test_var}"]}},
                "agents":{{"worker":{{"dir":"/tmp","cli":"codex"}}}}}}"#
        ),
    )
    .expect("write catalog");

    h.set_launch_env(test_var, test_val);

    let (code, stdout, stderr) = h.run(["agent", "worker", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&format!("{test_var}={test_val}")),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["agent", "show", "worker"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&format!("inherit_env: {test_var}")),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains(&format!("{test_var}={test_val}")),
        "stdout={stdout}"
    );
}

#[test]
#[cfg(unix)]
fn claude_agent_start_links_private_skills_without_prompt_manifest() {
    let h = Hcom::new();
    let workspace = h.root_path().join("workspace");
    let bundle = h.path().join("agents/solo");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(bundle.join("skills/review")).unwrap();
    std::fs::write(bundle.join("SOUL.md"), "Agent notes.").unwrap();
    std::fs::write(
        bundle.join("skills/review/SKILL.md"),
        "---\nname: review\ndescription: Review changes\n---\n",
    )
    .unwrap();
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"agents":{{"solo":{{"dir":{},"cli":"claude","terminal_command":"sh -c true {{script}}"}}}}}}"#,
            serde_json::to_string(&workspace).unwrap()
        ),
    )
    .unwrap();

    let native_skills = bundle.join(".claude/skills");
    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(&format!("--add-dir {}", bundle.display())));
    assert!(!stdout.contains("# Available agent skills"));
    assert!(
        !native_skills.exists(),
        "dry-run must not change the bundle"
    );
    let (code, stdout, stderr) = h.run(["agent", "solo"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert_eq!(
        std::fs::read_link(&native_skills).unwrap(),
        std::path::Path::new("../skills")
    );
    assert_eq!(
        std::fs::read_to_string(native_skills.join("review/SKILL.md")).unwrap(),
        "---\nname: review\ndescription: Review changes\n---\n"
    );
    let (code, stdout, stderr) = h.run(["agent", "solo"]);
    assert_eq!(code, 0, "repeated start: stdout={stdout} stderr={stderr}");
    assert_eq!(
        std::fs::read_link(&native_skills).unwrap(),
        std::path::Path::new("../skills")
    );
}

#[test]
fn agent_catalog_system_prompt_file_renders_and_reaches_spawn_script() {
    let h = Hcom::new();
    let prompt_path = h.path().join("SYSTEM_PROMPT.md");
    std::fs::write(&prompt_path, "Shared instructions from Markdown.\n")
        .expect("write system prompt");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"system_prompt_file":"SYSTEM_PROMPT.md",
            "agents":{"solo":{"dir":"/tmp","cli":"codex",
                "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Shared instructions from Markdown."),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["agent", "solo"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let launch_dir = h.path().join(".tmp/launch");
    let launch_files = std::fs::read_dir(&launch_dir)
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        launch_files.contains("Shared instructions from Markdown."),
        "catalog prompt did not reach spawn script: {launch_files}"
    );
}

#[test]
fn agent_catalog_system_prompt_precedes_initial_prompt_separator() {
    // `hcom handoff` relaunches with --hcom-prompt; the system prompt flag must
    // stay before `--`, or Claude reads it as part of the initial prompt.
    let h = Hcom::new();
    std::fs::write(h.path().join("SYSTEM_PROMPT.md"), "Catalog rules.\n")
        .expect("write system prompt");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"system_prompt_file":"SYSTEM_PROMPT.md",
            "agents":{"solo":{"dir":"/tmp","cli":"claude",
                "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--hcom-prompt", "Continue."]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let script = std::fs::read_dir(h.path().join(".tmp/launch"))
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .find(|text| text.contains("Continue."))
        .expect("launch script with the initial prompt");
    let flag = script.find("--append-system-prompt").expect(&script);
    let separator = script.find(" -- ").expect(&script);
    assert!(flag < separator, "system prompt after `--`: {script}");
}

#[test]
fn launch_scripts_export_catalog_agent_name_and_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"claude",
                "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");
    let launch_dir = h.path().join(".tmp/launch");
    let launch_files = || {
        std::fs::read_dir(&launch_dir)
            .expect("read launch directory")
            .filter_map(Result::ok)
            .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev2"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let scripts = launch_files();
    assert!(scripts.contains("HCOM_AGENT_NAME=reviewer"), "{scripts}");
    assert!(scripts.contains("HCOM_INSTANCE_NAME=rev2"), "{scripts}");
    std::fs::remove_dir_all(&launch_dir).expect("clear launch directory");

    // Outside the catalog the agent name is the instance name, even when the
    // caller is itself an agent whose own name must not leak into the child.
    let output = h
        .cmd()
        .env("HCOM_AGENT_NAME", "reviewer")
        .args([
            "claude",
            "--as",
            "solo",
            "--terminal",
            "sh -c true {script}",
            "--no-run-here",
            "--go",
        ])
        .output()
        .expect("run hcom");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let scripts = launch_files();
    assert!(scripts.contains("HCOM_AGENT_NAME=solo"), "{scripts}");
    assert!(scripts.contains("HCOM_INSTANCE_NAME=solo"), "{scripts}");
}

#[test]
fn handoff_relaunches_the_same_catalog_agent_under_its_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"claude",
                "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");
    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev2"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let scripts = std::fs::read_dir(h.path().join(".tmp/launch"))
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let process_id = scripts
        .split("HCOM_PROCESS_ID=")
        .nth(1)
        .and_then(|rest| {
            rest.split(|c: char| c.is_whitespace() || c == '\'' || c == ';')
                .find(|s| !s.is_empty())
        })
        .expect("process id in launch script")
        .to_string();

    let (code, stdout, stderr) = h.run_as_process(
        &process_id,
        [
            "handoff",
            "--dry-run",
            "--cli",
            "codex",
            "Continue step 2 of PLAN.md",
        ],
    );
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("kill rev2"), "stdout={stdout}");
    assert!(
        stdout.contains("HCOM_AGENT_NAME=reviewer"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("codex --as rev2"), "stdout={stdout}");
    assert!(
        stdout.contains("Continue step 2 of PLAN.md"),
        "stdout={stdout}"
    );

    // Only a real handoff that passed validation logs the life action.
    let handoffs = || -> i64 {
        rusqlite::Connection::open(h.hcom_dir.join("hcom.db"))
            .expect("open hcom db")
            .query_row(
                "SELECT COUNT(*) FROM events WHERE type = 'life' AND instance = 'rev2'
                   AND json_extract(data, '$.action') = 'handoff'",
                [],
                |row| row.get(0),
            )
            .expect("count handoff events")
    };
    assert_eq!(handoffs(), 0, "dry-run must not log a handoff");
    let catalog = h.path().join("agents.json");
    let parked = h.path().join("agents.json.parked");
    std::fs::rename(&catalog, &parked).expect("park catalog");
    let (code, _stdout, stderr) = h.run_as_process(&process_id, ["handoff", "next"]);
    std::fs::rename(&parked, &catalog).expect("restore catalog");
    assert_eq!(code, 1, "unknown agent fails validation: {stderr}");
    assert_eq!(handoffs(), 0, "failed validation must not log a handoff");
    let (code, stdout, stderr) = h.run_as_process(&process_id, ["handoff", "next"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert_eq!(handoffs(), 1);
    let (code, stdout, stderr) = h.run(["events", "--action", "handoff", "--all"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("\"action\":\"handoff\""), "stdout={stdout}");

    // An agent launched outside the catalog has no catalog name to relaunch.
    std::fs::remove_dir_all(h.path().join(".tmp/launch")).expect("clear launch directory");
    let (code, stdout, stderr) = h.run([
        "claude",
        "--as",
        "solo",
        "--terminal",
        "sh -c true {script}",
        "--no-run-here",
        "--go",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let script = std::fs::read_dir(h.path().join(".tmp/launch"))
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let solo_process_id = script
        .split("HCOM_PROCESS_ID=")
        .nth(1)
        .and_then(|rest| {
            rest.split(|c: char| c.is_whitespace() || c == '\'' || c == ';')
                .find(|s| !s.is_empty())
        })
        .expect("process id in launch script")
        .to_string();
    let (code, _stdout, stderr) =
        h.run_as_process(&solo_process_id, ["handoff", "--dry-run", "next"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("'solo' has no recorded agent name"),
        "stderr={stderr}"
    );

    // Without a recorded name, the agent's own HCOM_AGENT_NAME is used, but
    // only when the environment belongs to the instance being handed off.
    let handoff_with_env = |instance: &str| {
        let output = h
            .cmd()
            .env("HCOM_PROCESS_ID", &solo_process_id)
            .env("HCOM_INSTANCE_NAME", instance)
            .env("HCOM_AGENT_NAME", "reviewer")
            .args(["handoff", "--dry-run", "next"])
            .output()
            .expect("run handoff");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    let (code, stdout, stderr) = handoff_with_env("solo");
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("kill solo"), "stdout={stdout}");
    assert!(
        stdout.contains("HCOM_AGENT_NAME=reviewer"),
        "stdout={stdout}"
    );
    let (code, _stdout, stderr) = handoff_with_env("rev2");
    assert_eq!(code, 1);
    assert!(
        stderr.contains("'solo' has no recorded agent name"),
        "stderr={stderr}"
    );

    // A caller without an agent identity has nothing to hand off.
    let (code, _stdout, stderr) = h.run(["handoff", "--dry-run", "next"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("must be run by an hcom agent"),
        "stderr={stderr}"
    );
}

#[cfg(unix)]
#[test]
fn handoff_after_kill_and_resume_keeps_the_catalog_agent_of_an_as_instance() {
    let h = Hcom::new();
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"claude",
                "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");
    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev2"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let mut agent = idle_kill_sleeper(&h);
    make_idle_kill_row(&conn, "rev2", "listening", agent.id());
    let (code, stdout, stderr) = h.run(["kill", "rev2"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    agent.wait().expect("killed agent exits");
    let snapshot_agent: Option<String> = conn
        .query_row(
            "SELECT json_extract(data, '$.snapshot.agent_name') FROM events
             WHERE type = 'life' AND instance = 'rev2'
               AND json_extract(data, '$.action') = 'stopped'
             ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("stopped event for rev2");
    assert_eq!(snapshot_agent.as_deref(), Some("reviewer"));

    std::fs::remove_dir_all(h.path().join(".tmp/launch")).expect("clear launch directory");
    let (code, stdout, stderr) = h.run(["r", "rev2"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let scripts = std::fs::read_dir(h.path().join(".tmp/launch"))
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(scripts.contains("HCOM_AGENT_NAME=reviewer"), "{scripts}");
    let process_id = scripts
        .split("HCOM_PROCESS_ID=")
        .nth(1)
        .and_then(|rest| {
            rest.split(|c: char| c.is_whitespace() || c == '\'' || c == ';')
                .find(|s| !s.is_empty())
        })
        .expect("process id in launch script")
        .to_string();
    let (code, stdout, stderr) = h.run_as_process(&process_id, ["handoff", "--dry-run", "next"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("kill rev2"), "stdout={stdout}");
    assert!(
        stdout.contains("HCOM_AGENT_NAME=reviewer"),
        "stdout={stdout}"
    );

    // A plain restart of a running agent is not a handoff.
    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev3"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let mut agent = idle_kill_sleeper(&h);
    make_idle_kill_row(&conn, "rev3", "listening", agent.id());
    let (code, stdout, stderr) = h.run(["agent", "reviewer", "--as", "rev3", "--restart"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    agent.wait().expect("restarted agent exits");
    let handoffs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE type = 'life'
               AND json_extract(data, '$.action') = 'handoff'",
            [],
            |row| row.get(0),
        )
        .expect("count handoff events");
    assert_eq!(handoffs, 0);
}

#[test]
fn agent_catalog_inline_system_prompt_overrides_file() {
    let h = Hcom::new();
    std::fs::write(h.path().join("SYSTEM_PROMPT.md"), "Instructions from file.")
        .expect("write system prompt");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"system_prompt_file":"SYSTEM_PROMPT.md",
            "defaults":{"system_prompt":"Inline instructions."},
            "agents":{"solo":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "solo"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Inline instructions."), "stdout={stdout}");
    assert!(
        !stdout.contains("Instructions from file."),
        "stdout={stdout}"
    );

    std::fs::write(
        h.path().join("agents.json"),
        r#"{"system_prompt_file":"SYSTEM_PROMPT.md",
            "defaults":{"system_prompt":""},
            "agents":{"solo":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("clear inline system prompt");
    let (code, stdout, stderr) = h.run(["agent", "show", "solo"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stdout.contains("--hcom-system-prompt"), "stdout={stdout}");
}

#[test]
fn agent_catalog_imported_system_prompt_file_resolves_beside_source() {
    let h = Hcom::new();
    let source_dir = h.root_path().join("shared-catalog");
    std::fs::create_dir_all(&source_dir).expect("create source catalog directory");
    std::fs::write(
        source_dir.join("SYSTEM_PROMPT.md"),
        "Imported instructions.",
    )
    .expect("write imported system prompt");
    let source_catalog = source_dir.join("agents.json");
    std::fs::write(
        &source_catalog,
        r#"{"system_prompt_file":"SYSTEM_PROMPT.md","agents":{"shared":{"cli":"codex"}}}"#,
    )
    .expect("write imported catalog");
    std::fs::write(
        h.path().join("agents.json"),
        serde_json::json!({"imports": [{"from": source_catalog, "agents": ["shared"]}]})
            .to_string(),
    )
    .expect("write global catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "shared"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Imported instructions."), "stdout={stdout}");
}

#[test]
fn agent_catalog_system_prompt_file_reports_read_error() {
    let h = Hcom::new();
    let catalog_path = h.path().join("agents.json");
    std::fs::write(
        &catalog_path,
        r#"{"system_prompt_file":"missing.md","agents":{"solo":{}}}"#,
    )
    .expect("write catalog");

    let (code, _stdout, stderr) = h.run(["agent", "show", "solo"]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains(&catalog_path.display().to_string()),
        "stderr={stderr}"
    );
    assert!(stderr.contains("missing.md"), "stderr={stderr}");
    assert!(
        stderr.contains("cannot read system prompt"),
        "stderr={stderr}"
    );

    std::fs::write(h.path().join("missing.md"), [0xff, 0xfe])
        .expect("write non-UTF-8 system prompt");
    let (code, _stdout, stderr) = h.run(["agent", "show", "solo"]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("cannot read system prompt"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("valid UTF-8"), "stderr={stderr}");
}

#[test]
fn agent_agy_external_bundle_dry_run_adds_bundle_to_workspace() {
    let h = Hcom::new();
    let workspace = h.root_path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    for name in ["agy_agent", "antigravity_agent"] {
        let bundle = h.path().join("agents").join(name);
        std::fs::create_dir_all(&bundle).expect("create bundle");
        std::fs::write(bundle.join("SOUL.md"), "Editable agent memory.")
            .expect("write bundle instructions");
    }
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"agents":{{
                "agy_agent":{{"dir":{},"cli":"agy"}},
                "antigravity_agent":{{"dir":{},"cli":"antigravity"}}
            }}}}"#,
            serde_json::to_string(&workspace).unwrap(),
            serde_json::to_string(&workspace).unwrap()
        ),
    )
    .expect("write catalog");

    for name in ["agy_agent", "antigravity_agent"] {
        let bundle = h.path().join("agents").join(name);
        let (code, stdout, stderr) = h.run(["agent", name, "--dry-run"]);
        assert_eq!(code, 0, "name={name} stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(&format!("--add-dir {}", bundle.display())),
            "name={name} stdout={stdout}"
        );
    }
}

#[test]
#[cfg(unix)]
fn agent_agy_policy_cwd_uses_canonical_launch_directory() {
    let h = Hcom::new();
    h.set_launch_env("DIPPY_POLICY_CWD", "/parent-policy");
    let workspace = h.root_path().join("workspace");
    let alias = h.root_path().join("workspace-alias");
    std::fs::create_dir_all(&workspace).unwrap();
    std::os::unix::fs::symlink(&workspace, &alias).unwrap();
    let bundle = h.path().join("agents/knowledge");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("SOUL.md"), "Knowledge agent.").unwrap();
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"agents":{{"knowledge":{{"dir":{},"cli":"agy","env":{{"DIPPY_POLICY_CWD":"/wrong"}}}}}}}}"#,
            serde_json::to_string(&alias).unwrap()
        ),
    )
    .unwrap();

    let (code, stdout, stderr) = h.run(["agent", "knowledge", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&format!("DIPPY_POLICY_CWD={}", workspace.display())),
        "stdout={stdout}"
    );
    assert!(stdout.contains(&format!("--add-dir {}", bundle.display())));
    assert!(!stdout.contains("DIPPY_POLICY_CWD=/wrong"));
    let other = h.root_path().join("other-workspace");
    std::fs::create_dir_all(&other).unwrap();
    let (code, stdout, stderr) = h.run([
        "agent",
        "knowledge",
        "--dir",
        other.to_str().unwrap(),
        "--dry-run",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(&format!("--dir {}", other.display())));
    assert!(stdout.contains(&format!("DIPPY_POLICY_CWD={}", workspace.display())));

    // The generated launcher script exports the value to AGY; AGY hook
    // subprocesses inherit this process environment. A plain child launch
    // cannot inherit the parent agent's policy scope.
    let (code, _script, stderr) = h.run(["agent", "knowledge", "--terminal", "print"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let launch_dir = h.path().join(".tmp/launch");
    let knowledge_env_path = std::fs::read_dir(&launch_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("antigravity_knowledge_") && name.ends_with(".env")
                })
        })
        .expect("knowledge launcher environment file");
    let script = std::fs::read_to_string(&knowledge_env_path).unwrap();
    assert!(
        script.contains(&format!("DIPPY_POLICY_CWD={}", workspace.display())),
        "launcher env path={}",
        knowledge_env_path.display()
    );
    assert!(!script.contains("/parent-policy"));
    assert!(!script.contains("DIPPY_POLICY_CWD=/wrong"));
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).unwrap();
    let launch_context: String = conn
        .query_row(
            "SELECT launch_context FROM instances WHERE name='knowledge'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let launch_context: serde_json::Value = serde_json::from_str(&launch_context).unwrap();
    assert_eq!(
        launch_context["dippy_policy_cwd"].as_str(),
        workspace.to_str()
    );
    let hook = std::process::Command::new("bash")
        .args([
            "-c",
            "source \"$1\"; sh -c 'printf %s \"$DIPPY_POLICY_CWD\"'",
            "_",
        ])
        .arg(&knowledge_env_path)
        .env_remove("DIPPY_POLICY_CWD")
        .output()
        .unwrap();
    assert!(hook.status.success());
    assert_eq!(hook.stdout, workspace.to_string_lossy().as_bytes());

    let (code, _script, stderr) = h.run(["agy", "--terminal", "print", "--no-run-here"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let child_env_paths = std::fs::read_dir(&launch_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "env") && path != &knowledge_env_path
        })
        .collect::<Vec<_>>();
    assert!(!child_env_paths.is_empty());
    for path in child_env_paths {
        let child_env = std::fs::read_to_string(&path).unwrap();
        assert!(
            !child_env.contains("DIPPY_POLICY_CWD"),
            "path={}",
            path.display()
        );
    }
}

#[test]
fn herdr_placement_belongs_to_one_launch_and_never_reaches_a_plain_child() {
    let h = Hcom::new();
    // The caller is itself a placed agent.
    let run_placed = |args: &[&str]| -> (i32, String) {
        let out = h
            .cmd()
            .env("HCOM_HERDR_WORKSPACE", "parent-space")
            .env("HCOM_HERDR_TAB", "parent-tab")
            .args(args)
            .output()
            .expect("spawn hcom");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"coach":{"cli":"claude","env":{"HCOM_HERDR_WORKSPACE":"child-space","HCOM_HERDR_TAB":"coach"}}}}"#,
    )
    .unwrap();
    let launch_dir = h.path().join(".tmp/launch");
    // The runner script exports the launch's HCOM_* values.
    let env_files = || -> Vec<std::path::PathBuf> {
        std::fs::read_dir(&launch_dir)
            .map(|dir| {
                dir.map(|entry| entry.unwrap().path())
                    .filter(|path| path.extension().is_some_and(|ext| ext == "sh"))
                    .collect()
            })
            .unwrap_or_default()
    };

    // Catalog launch: its own placement reaches the launch and its record.
    let (code, stderr) = run_placed(&["agent", "coach", "--terminal", "print"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let catalog_env = env_files();
    assert_eq!(catalog_env.len(), 1, "files={catalog_env:?}");
    let script = std::fs::read_to_string(&catalog_env[0]).unwrap();
    assert!(script.contains("child-space"), "env={script}");
    assert!(!script.contains("parent-space") && !script.contains("parent-tab"));
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).unwrap();
    let launch_context: String = conn
        .query_row(
            "SELECT launch_context FROM instances WHERE name='coach'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let launch_context: serde_json::Value = serde_json::from_str(&launch_context).unwrap();
    assert_eq!(launch_context["herdr_workspace"], "child-space");
    assert_eq!(launch_context["herdr_tab"], "coach");

    // Plain nested launch: the caller's placement is dropped.
    let (code, stderr) = run_placed(&["claude", "--terminal", "print", "--no-run-here"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let child_env: Vec<_> = env_files()
        .into_iter()
        .filter(|path| !catalog_env.contains(path))
        .collect();
    assert_eq!(child_env.len(), 1, "files={child_env:?}");
    let script = std::fs::read_to_string(&child_env[0]).unwrap();
    assert!(
        !script.contains("HCOM_HERDR_WORKSPACE") && !script.contains("HCOM_HERDR_TAB"),
        "env={script}"
    );
    let contexts: Vec<String> = conn
        .prepare("SELECT COALESCE(launch_context, '') FROM instances WHERE name != 'coach'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(
        contexts.iter().all(|ctx| !ctx.contains("herdr_")),
        "contexts={contexts:?}"
    );
}

#[test]
fn agent_agy_policy_cwd_survives_tracked_resume_and_cli_switch() {
    let h = Hcom::new();
    h.set_launch_env("DIPPY_POLICY_CWD", "/ambient-policy");
    let workspace = h.root_path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"cli_overrides":{{"codex":"claude"}},"agents":{{"knowledge":{{"dir":{},"cli":"codex","env":{{"DIPPY_POLICY_CWD":"/wrong"}}}}}}}}"#,
            serde_json::to_string(&workspace).unwrap()
        ),
    )
    .unwrap();

    let (code, stdout, stderr) = h.run(["agent", "knowledge", "--cli", "agy", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(&format!("DIPPY_POLICY_CWD={}", workspace.display())));
    let (code, stdout, stderr) = h.run(["agent", "knowledge", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stdout.contains("DIPPY_POLICY_CWD"), "stdout={stdout}");
    assert!(stdout.contains("claude --as knowledge"), "stdout={stdout}");

    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).unwrap();
    conn.execute(
        "INSERT INTO instances (name, session_id, tool, directory, status, status_context, created_at) \
         VALUES ('knowledge', 'agy-session', 'antigravity', ?1, 'inactive', 'exit:0', 1)",
        rusqlite::params![workspace.to_string_lossy().as_ref()],
    )
    .unwrap();
    let snapshot = serde_json::json!({
        "action": "stopped",
        "snapshot": {
            "tool": "antigravity",
            "session_id": "agy-session",
            "launch_args": "[]",
            "directory": workspace,
            "dippy_policy_cwd": workspace,
            "tag": "",
            "background": 0,
            "last_event_id": 0
        }
    });
    conn.execute(
        "INSERT INTO events (timestamp, type, instance, data) \
         VALUES ('2026-01-01T00:00:00Z', 'life', 'knowledge', ?1)",
        rusqlite::params![snapshot.to_string()],
    )
    .unwrap();
    drop(conn);

    let (code, _stdout, stderr) = h.run(["r", "knowledge", "--terminal", "print", "--no-run-here"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let launch_dir = h.path().join(".tmp/launch");
    let resume_env = std::fs::read_dir(&launch_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "env"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .find(|content| content.contains("DIPPY_POLICY_CWD"))
        .expect("tracked resume launcher environment");
    assert!(
        resume_env.contains(&format!("DIPPY_POLICY_CWD={}", workspace.display())),
        "resume launcher has wrong policy cwd"
    );
    assert!(!resume_env.contains("/ambient-policy"));

    let (code, _stdout, stderr) = h.run(["f", "knowledge", "--dry-run"]);
    assert_ne!(code, 0, "AGY fork is unsupported");
    assert!(stderr.contains("fork"), "stderr={stderr}");
}

#[test]
fn agent_group_dry_run_includes_agents_hidden_by_selective_import() {
    let h = Hcom::new();
    let imported = h.path().join("project-agents.json");
    std::fs::write(
        &imported,
        r#"{"agents":{
            "public":{"dir":"/tmp","cli":"claude","groups":["crew"]},
            "private":{"dir":"/tmp","cli":"gemini","groups":["crew"]}
        }}"#,
    )
    .expect("write imported catalog");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["public"]}}]}}"#,
            serde_json::to_string(&imported).unwrap()
        ),
    )
    .expect("write catalog");

    let (code, visible, stderr) = h.run(["agent", "list", "--names", "--no-project"]);
    assert_eq!(code, 0, "stdout={visible} stderr={stderr}");
    assert_eq!(visible.trim(), "public");

    let (code, groups, stderr) = h.run(["agent", "list", "--groups", "--no-project"]);
    assert_eq!(code, 0, "stdout={groups} stderr={stderr}");
    assert_eq!(groups.trim(), "@crew");

    let (code, members, stderr) = h.run(["agent", "list", "@crew", "--names", "--no-project"]);
    assert_eq!(code, 0, "stdout={members} stderr={stderr}");
    assert_eq!(members.trim(), "private\npublic");

    let (code, json, stderr) = h.run(["agent", "list", "@crew", "--json", "--no-project"]);
    assert_eq!(code, 0, "stdout={json} stderr={stderr}");
    let listed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON listing");
    assert_eq!(listed.as_array().map(Vec::len), Some(2));

    let (code, stdout, stderr) = h.run([
        "agent",
        "@crew",
        "--cli",
        "codex",
        "--dry-run",
        "--no-project",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as private"), "stdout={stdout}");
    assert!(stdout.contains("codex --as public"), "stdout={stdout}");
    assert!(
        stdout.find("--as private") < stdout.find("--as public"),
        "members must launch in name order: {stdout}"
    );
    assert!(
        stdout.contains("group '@crew': 2 succeeded, 0 failed (2 total)"),
        "stdout={stdout}"
    );
}

fn agent_names(stdout: &str) -> Vec<(String, String)> {
    let rows: serde_json::Value = serde_json::from_str(stdout).expect("agent list JSON");
    rows.as_array()
        .expect("agent list array")
        .iter()
        .map(|r| {
            (
                r["name"].as_str().unwrap_or_default().to_string(),
                r["source"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

#[test]
fn agent_global_sidecar_merges_into_the_global_catalog() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"cli_overrides":{"codex":"claude"},
            "defaults":{"model":"base-model"},
            "agents":{
            "a":{"cli":"claude","dir":"/tmp","model":"m1","env":{"A":"1"},"args":["--x"]},
            "b":{"cli":"claude","dir":"/tmp"},
            "gone":{"cli":"claude","dir":"/tmp"},
            "mapped":{"cli":"codex","dir":"/tmp"}}}"#,
    )
    .expect("write catalog");
    std::fs::write(
        h.path().join("agents.local.json"),
        r#"{"cli_overrides":{"gemini":"codex"},
            "defaults":{"reasoning":"high"},
            "agents":{
            "a":{"model":"m2","env":{"B":"2"},"args":["--y"]},
            "added":{"cli":"gemini","dir":"/tmp"},
            "gone":{"disabled":true}}}"#,
    )
    .expect("write sidecar");

    let (code, stdout, stderr) = h.run(["agent", "show", "a"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("model:     m2"), "stdout={stdout}");
    assert!(stdout.contains("reasoning: high"), "stdout={stdout}");
    assert!(stdout.contains("A=1 B=2"), "stdout={stdout}");
    assert!(stdout.contains("args:      --x --y"), "stdout={stdout}");
    let base = h.path().join("agents.json");
    let sidecar = h.path().join("agents.local.json");
    assert!(
        stdout.contains(&format!(
            "sources:   {}, {}",
            base.display(),
            sidecar.display()
        )),
        "stdout={stdout}"
    );
    let (code, stdout, stderr) = h.run(["agent", "show", "b"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("model:     base-model"), "stdout={stdout}");
    assert!(stdout.contains("reasoning: high"), "stdout={stdout}");

    // cli_overrides merge per source: the base maps codex, the sidecar gemini.
    let (code, stdout, stderr) = h.run(["agent", "show", "mapped"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("cli:       claude"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["agent", "show", "added"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("cli:       codex"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["agent", "list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let names = agent_names(&stdout);
    assert!(names.contains(&("a".into(), "global".into())), "{names:?}");
    assert!(
        names.contains(&("added".into(), "global".into())),
        "{names:?}"
    );
    assert!(!names.iter().any(|(n, _)| n == "gone"), "{names:?}");
    let (code, stdout, stderr) = h.run(["agent", "list", "--all", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(r#""name":"gone""#) || stdout.contains(r#""name": "gone""#));

    // Sidecar errors name the sidecar.
    for bad in [r#"{"agents":"#, r#"{"imports":[{"from":"other.json"}]}"#] {
        std::fs::write(&sidecar, bad).expect("write bad sidecar");
        let (code, stdout, stderr) = h.run(["agent", "show", "a"]);
        assert_ne!(code, 0, "stdout={stdout}");
        assert!(
            stderr.contains(&sidecar.display().to_string()),
            "stderr={stderr}"
        );
    }
}

#[test]
fn agent_global_sidecar_without_base_catalog_is_loaded() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.local.json"),
        r#"{"agents":{"only":{"cli":"claude","dir":"/tmp"}}}"#,
    )
    .expect("write sidecar");
    let (code, stdout, stderr) = h.run(["agent", "show", "only"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&h.path().join("agents.local.json").display().to_string()),
        "stdout={stdout}"
    );
}

#[test]
fn agent_sidecars_of_extra_imported_and_project_catalogs_keep_their_scope() {
    let h = Hcom::new();
    let root = h.root_path();
    let shared = root.join("shared");
    let extra = root.join("extra");
    let project = root.join("proj");
    std::fs::create_dir_all(&shared).expect("create shared dir");
    std::fs::create_dir_all(&extra).expect("create extra dir");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project dir");
    std::fs::write(
        shared.join("agents.json"),
        r#"{"agents":{"s":{"cli":"claude","dir":"/tmp","model":"s1"},
                      "hidden":{"cli":"claude","dir":"/tmp"}}}"#,
    )
    .expect("write shared catalog");
    std::fs::write(
        shared.join("agents.local.json"),
        r#"{"agents":{"s":{"model":"s2"},"sneaky":{"cli":"claude","dir":"/tmp"}}}"#,
    )
    .expect("write shared sidecar");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["s"]}}]}}"#,
            serde_json::to_string(&shared.join("agents.json")).unwrap()
        ),
    )
    .expect("write global catalog");
    std::fs::write(
        extra.join("team.json"),
        r#"{"agents":{"e":{"cli":"claude","dir":"/tmp","model":"e1"}}}"#,
    )
    .expect("write extra catalog");
    std::fs::write(
        extra.join("team.local.json"),
        r#"{"agents":{"e":{"model":"e2"}}}"#,
    )
    .expect("write extra sidecar");
    std::fs::write(
        project.join(".hcom/agents.json"),
        r#"{"agents":{"p":{"cli":"claude","dir":".","model":"p1"}}}"#,
    )
    .expect("write project catalog");
    std::fs::write(
        project.join(".hcom/agents.local.json"),
        r#"{"agents":{"p":{"model":"p2"},"padd":{"cli":"claude","dir":"."}}}"#,
    )
    .expect("write project sidecar");

    let run_in = |dir: &std::path::Path, args: &[&str]| {
        let output = h
            .cmd()
            .current_dir(dir)
            .env("HCOM_AGENT_CATALOGS", extra.join("team.json"))
            .args(args)
            .output()
            .expect("run hcom");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    for (dir, name, model) in [
        (root, "s", "s2"),
        (root, "e", "e2"),
        (project.as_path(), "p", "p2"),
    ] {
        let (code, stdout, stderr) = run_in(dir, &["agent", "show", name]);
        assert_eq!(code, 0, "{name}: stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(&format!("model:     {model}")),
            "{name}: stdout={stdout}"
        );
    }
    let (code, stdout, stderr) = run_in(project.as_path(), &["agent", "show", "padd"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    // Outside the project and the import filter, sidecar additions stay private.
    let (code, stdout, stderr) = run_in(root, &["agent", "list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let names = agent_names(&stdout);
    for private in ["sneaky", "hidden", "padd", "p"] {
        assert!(!names.iter().any(|(n, _)| n == private), "{names:?}");
    }
    for private in ["sneaky", "padd"] {
        let (code, _stdout, stderr) = run_in(root, &["agent", "show", private]);
        assert_ne!(code, 0, "{private} must not resolve: {stderr}");
    }

    // cli_overrides stay global-only, also in a sidecar.
    std::fs::write(
        project.join(".hcom/agents.local.json"),
        r#"{"cli_overrides":{"codex":"claude"}}"#,
    )
    .expect("write project sidecar with cli_overrides");
    let (code, _stdout, stderr) = run_in(project.as_path(), &["agent", "show", "p"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains(
            &project
                .join(".hcom/agents.local.json")
                .display()
                .to_string()
        ),
        "stderr={stderr}"
    );
}

#[test]
fn kill_by_group_and_at_prefix() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "worker_a":{"dir":"/tmp","cli":"claude","groups":["workers"]},
            "worker_b":{"dir":"/tmp","cli":"gemini","groups":["workers"]}
        }}"#,
    )
    .expect("write catalog");

    // Initially nothing running, kill @workers fails with no active agents
    let (code, stdout, stderr) = h.run(["kill", "@workers"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("No active agents in group '@workers'"),
        "stderr={stderr}"
    );

    // --json reports the failure under the selector as written.
    let (code, stdout, stderr) = h.run(["kill", "@workers", "--json"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("kill --json");
    assert_eq!(report["ok"], false, "stdout={stdout}");
    assert_eq!(
        report["results"][0]["target"], "@workers",
        "stdout={stdout}"
    );
    assert_eq!(report["results"][0]["kind"], "error", "stdout={stdout}");

    // Bare group name without @ suggests @workers
    let (code, stdout, stderr) = h.run(["kill", "workers"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("No agent named 'workers'")
            && stderr.contains("\nDid you mean @workers? Groups require @"),
        "stderr={stderr}"
    );

    // Populate active instances for worker_a and worker_b
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances (name, status, status_time, created_at, tool, pid) \
         VALUES ('worker_a', 'listening', ?1, ?1, 'claude', 999991)",
        rusqlite::params![now],
    )
    .expect("insert worker_a");
    conn.execute(
        "INSERT INTO instances (name, status, status_time, created_at, tool, pid) \
         VALUES ('worker_b', 'listening', ?1, ?1, 'gemini', 999992)",
        rusqlite::params![now],
    )
    .expect("insert worker_b");

    // @ prefix is strictly for groups; @worker_a is not a group so it fails
    let (code, stdout, stderr) = h.run(["kill", "@worker_a"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("unknown or empty agent group '@worker_a'"),
        "stderr={stderr}"
    );

    // Single agent kill works by plain agent name
    let (code, stdout, stderr) = h.run(["kill", "worker_a"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("worker_a"), "stdout={stdout}");

    // Group kill works and kills remaining member worker_b
    let (code, stdout, stderr) = h.run(["kill", "@workers"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Killed 1 (group:@workers)"),
        "stdout={stdout}"
    );

    // Now group is completely stopped
    let (code, stdout, stderr) = h.run(["kill", "@workers"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("No active agents in group '@workers'"),
        "stderr={stderr}"
    );
}

#[test]
fn kill_by_tool_kills_only_instances_of_that_cli() {
    let h = Hcom::new();

    let (code, stdout, stderr) = h.run(["kill", "tool:nosuch"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("Unknown tool 'nosuch'. Valid tools: claude,"),
        "stderr={stderr}"
    );

    let (code, stdout, stderr) = h.run(["kill", "tool:agy"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("No agents with tool 'antigravity'"),
        "stderr={stderr}"
    );

    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    for (name, tool, pid) in [
        ("agy_a", "antigravity", 999981),
        ("agy_b", "antigravity", 999982),
        ("cla_a", "claude", 999983),
    ] {
        conn.execute(
            "INSERT INTO instances (name, status, status_time, created_at, tool, pid) \
             VALUES (?1, 'listening', ?2, ?2, ?3, ?4)",
            rusqlite::params![name, now, tool, pid],
        )
        .expect("insert instance");
    }

    let (code, stdout, stderr) = h.run(["kill", "tool:agy"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("agy_a")
            && stdout.contains("agy_b")
            && !stdout.contains("cla_a")
            && stdout.contains("Killed 2 (tool:antigravity)"),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("cla_a") && !stdout.contains("agy_a"),
        "stdout={stdout}"
    );
}

#[test]
fn kill_self_kills_only_the_calling_instance() {
    let h = Hcom::new();
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    for (name, pid) in [("sel_a", 999971), ("sel_b", 999972), ("self", 999973)] {
        conn.execute(
            "INSERT INTO instances (name, status, status_time, created_at, tool, pid) \
             VALUES (?1, 'listening', ?2, ?2, 'claude', ?3)",
            rusqlite::params![name, now, pid],
        )
        .expect("insert instance");
    }

    let (code, stdout, stderr) = h.run(["kill", "SELF"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("Cannot determine identity for SELF"),
        "stderr={stderr}"
    );

    let (code, stdout, stderr) = h.run(["kill", "SELF", "--name", "sel_a"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("sel_a") && !stdout.contains("sel_b"),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["kill", "self"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    // Session environment (process binding) without --name.
    let bound = h.start_with_process_id("pid-self-bound");
    conn.execute(
        "UPDATE instances SET pid = 999974 WHERE name = ?1",
        rusqlite::params![bound],
    )
    .expect("set pid");
    let (code, stdout, stderr) = h.run_as_process("pid-self-bound", ["kill", "SELF"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&bound) && !stdout.contains("sel_b"),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("sel_b")
            && !stdout.contains("sel_a")
            && !stdout.contains("\"self\"")
            && !stdout.contains(&format!("\"{bound}\"")),
        "stdout={stdout}"
    );
}

#[test]
fn agent_zsh_completions_add_names_and_groups_separately() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["agent", "completions", "zsh"]);

    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("  compadd -a names\n"), "stdout={stdout}");
    assert!(stdout.contains("  compadd -a groups\n"), "stdout={stdout}");
    assert!(
        !stdout.contains("compadd -a names -a groups"),
        "stdout={stdout}"
    );
}

#[test]
fn completions_command_generates_valid_scripts_for_all_shells() {
    let h = Hcom::new();

    // Bash
    let (code, stdout, stderr) = h.run(["completions", "bash"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("_hcom()"), "stdout={stdout}");
    assert!(stdout.contains("complete -F _hcom hcom"), "stdout={stdout}");
    assert!(stdout.contains("agent"), "stdout={stdout}");
    assert!(stdout.contains("kill"), "stdout={stdout}");

    // Singular alias 'completion'
    let (code, stdout_alias, stderr) = h.run(["completion", "bash"]);
    assert_eq!(code, 0, "stdout={stdout_alias} stderr={stderr}");
    assert_eq!(stdout, stdout_alias);

    // Zsh
    let (code, stdout, stderr) = h.run(["completions", "zsh"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("#compdef hcom"), "stdout={stdout}");
    assert!(stdout.contains("compdef _hcom hcom"), "stdout={stdout}");

    // Fish
    let (code, stdout, stderr) = h.run(["completions", "fish"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("complete -c hcom"), "stdout={stdout}");

    // Help
    let (code, stdout, stderr) = h.run(["completions", "--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Usage:"), "stdout={stdout}");
    assert!(stdout.contains("hcom completions"), "stdout={stdout}");
}

#[test]
fn agent_group_rejects_single_instance_flags_and_terminal_here() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","groups":["crew"]}}}"#,
    )
    .expect("write catalog");

    for flag in ["--as", "--attach"] {
        let args = if flag == "--as" {
            vec!["agent", "@crew", flag, "alias", "--dry-run"]
        } else {
            vec!["agent", "@crew", flag, "--dry-run"]
        };
        let (code, _stdout, stderr) = h.run(args);
        assert_ne!(code, 0, "flag={flag}");
        assert!(stderr.contains(flag), "stderr={stderr}");
    }

    let (code, _stdout, stderr) = h.run(["agent", "@crew", "--terminal", "here", "--dry-run"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("current terminal"), "stderr={stderr}");
}

#[test]
fn agent_group_continues_after_a_member_fails() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "a_broken":{"dir":"/tmp","cli":"not-a-real-tool","groups":["crew"],
                "terminal_command":"sh -c true {script}"},
            "z_working":{"dir":"/tmp","cli":"codex","groups":["crew"],
                "terminal_command":"sh -c true {script}"}
        }}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "@crew", "--no-project"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("agent 'a_broken' failed"),
        "stderr={stderr}"
    );
    assert!(
        stdout.contains("group '@crew': 1 succeeded, 1 failed (2 total)"),
        "stdout={stdout}"
    );
}

#[test]
fn agent_disabled_entry_is_treated_as_undefined() {
    let h = Hcom::new();
    let project = h.root_path().join("weather-app");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::create_dir_all(project.join(".hcom")).expect("create project .hcom");
    std::fs::write(
        project.join(".hcom/agents.json"),
        r#"{"agents":{"loc_off":{},"loc_on":{"disabled":false}}}"#,
    )
    .expect("write project catalog");
    // Defaults disable every agent; an entry's false re-enables it.
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"defaults":{"disabled":true,"cli":"codex","terminal_command":"sh -c true {script}"},
            "agents":{
                "worker_on":{"dir":"/tmp","groups":["crew"],"disabled":false},
                "worker_off":{"dir":"/tmp","groups":["crew","hidden"]},
                "reviewer":{"roaming":true}
            }}"#,
    )
    .expect("write catalog");
    let run_in = |dir: &std::path::Path, args: &[&str]| {
        let output = h
            .cmd()
            .current_dir(dir)
            .args(args)
            .output()
            .expect("run hcom");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    // Launch, resume, show and send autostart answer exactly as for an undefined name.
    for (disabled, undefined_name, args) in [
        (
            "worker_off",
            "worker_ofx",
            vec!["agent", "worker_off", "--dry-run"],
        ),
        (
            "worker_off",
            "worker_ofx",
            vec!["agent", "worker_off", "--resume", "--dry-run"],
        ),
        (
            "worker_off",
            "worker_ofx",
            vec!["agent", "show", "worker_off"],
        ),
        (
            "worker_off",
            "worker_ofx",
            vec!["send", "--from", "bigboss", "@worker_off", "--", "hi"],
        ),
        (
            "reviewer",
            "reviewex",
            vec!["send", "--from", "bigboss", "@reviewer", "--", "hi"],
        ),
    ] {
        let undefined_args: Vec<String> = args
            .iter()
            .map(|arg| arg.replace(disabled, undefined_name))
            .collect();
        let off = run_in(&project, &args);
        let undefined = h
            .cmd()
            .current_dir(&project)
            .args(&undefined_args)
            .output()
            .expect("run hcom");
        assert_ne!(off.0, 0, "{args:?}: stdout={} stderr={}", off.1, off.2);
        assert_eq!(Some(off.0), undefined.status.code(), "{args:?}");
        assert_eq!(
            off.2.replace(disabled, undefined_name),
            String::from_utf8_lossy(&undefined.stderr),
            "{args:?}"
        );
    }
    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(
        instances.trim(),
        "[]",
        "no roaming or catalog instance started"
    );

    // Group launch and listings skip the disabled entry.
    let (code, stdout, stderr) = h.run(["agent", "@crew", "--dry-run", "--no-project"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("--as worker_on") && !stdout.contains("worker_off"),
        "stdout={stdout}"
    );
    let (code, stdout, stderr) = h.run(["agent", "@hidden", "--dry-run", "--no-project"]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("unknown or empty agent group '@hidden'"),
        "stderr={stderr}"
    );
    for args in [
        vec!["agent", "list", "--names"],
        vec!["agent", "list", "--all", "--names"],
        vec!["agent", "list", "--all", "@crew", "--names"],
    ] {
        let (code, stdout, stderr) = run_in(&project, &args);
        assert_eq!(code, 0, "{args:?}: stderr={stderr}");
        assert_eq!(
            stdout,
            if args.contains(&"@crew") {
                "worker_on\n"
            } else {
                "loc_on\nworker_on\n"
            },
            "{args:?}"
        );
    }
    let (code, stdout, stderr) = run_in(&project, &["agent", "list", "--all", "--groups"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "@crew\n");
    let (code, stdout, stderr) = run_in(&project, &["agent", "list", "--local", "--names"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(stdout, "loc_on\n");

    // Only --all shows it, marked, in every output mode.
    let (code, stdout, stderr) = run_in(&project, &["agent", "list", "--for-agents"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(!stdout.contains("worker_off"), "stdout={stdout}");
    let (code, stdout, stderr) = run_in(&project, &["agent", "list", "--all", "--for-agents"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let line = |name: &str| {
        stdout
            .lines()
            .find(|line| line.starts_with(name))
            .unwrap_or_else(|| panic!("{name} missing: {stdout}"))
            .to_string()
    };
    assert!(line("worker_off").contains("(disabled)"), "stdout={stdout}");
    assert!(!line("worker_on").contains("disabled"), "stdout={stdout}");
    let (code, table, stderr) = run_in(&project, &["agent", "list", "--all", "--for-humans"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        table
            .lines()
            .any(|line| line.starts_with("worker_off") && line.ends_with("disabled")),
        "table={table}"
    );
    for (args, expected) in [
        (
            vec!["agent", "list", "--all", "--json"],
            vec![
                ("loc_off", true),
                ("loc_on", false),
                ("reviewer", true),
                ("worker_off", true),
                ("worker_on", false),
            ],
        ),
        (
            vec!["agent", "list", "--all", "--local", "--json"],
            vec![("loc_off", true), ("loc_on", false)],
        ),
    ] {
        let (code, stdout, stderr) = run_in(&project, &args);
        assert_eq!(code, 0, "{args:?}: stderr={stderr}");
        let listed: serde_json::Value = serde_json::from_str(&stdout).expect("list JSON");
        let listed: Vec<(String, bool)> = listed
            .as_array()
            .expect("array")
            .iter()
            .map(|item| {
                (
                    item["name"].as_str().unwrap().to_string(),
                    item["disabled"].as_bool().unwrap(),
                )
            })
            .collect();
        let expected: Vec<(String, bool)> = expected
            .into_iter()
            .map(|(name, disabled)| (name.to_string(), disabled))
            .collect();
        assert_eq!(listed, expected, "{args:?}");
    }
}

#[test]
fn running_instance_of_a_disabled_agent_is_still_messageable_and_killable() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"worker_off":{"dir":"/tmp","cli":"claude","disabled":true}}}"#,
    )
    .expect("write catalog");
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances (name, status, status_time, created_at, tool, pid) \
         VALUES ('worker_off', 'listening', ?1, ?1, 'claude', 999991)",
        rusqlite::params![now],
    )
    .expect("insert worker_off");

    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@worker_off", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1", "--full"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(
        event["data"]["delivered_to"],
        serde_json::json!(["worker_off"])
    );

    let (code, stdout, stderr) = h.run(["kill", "worker_off"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("worker_off"), "stdout={stdout}");
}

#[test]
fn agent_as_uses_catalog_config_with_a_distinct_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"codex","terminal":"wezterm-tab"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--as", "solo_two", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as solo_two"), "stdout={stdout}");
    assert!(stdout.contains("--dir /tmp"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["agent", "show", "solo", "--as", "solo_two"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("name:      solo_two"), "stdout={stdout}");
    assert!(stdout.contains("codex --as solo_two"), "stdout={stdout}");
}

#[test]
fn agent_start_mode_uses_catalog_and_cli_override() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"defaults":{"resume":true},
            "agents":{"solo":{"dir":"/tmp","cli":"codex","terminal":"wezterm-tab"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "solo"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(" r solo "), "stdout={stdout}");
    assert!(stdout.contains("--go"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--clean", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
    assert!(!stdout.contains(" r solo "), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["agent", "show", "solo", "--clean"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("start:     clean"), "stdout={stdout}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
}

#[test]
fn agent_continue_without_previous_session_fails() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let (code, _stdout, stderr) = h.run(["agent", "solo", "--continue"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("cannot continue 'solo'"), "stderr={stderr}");
}

#[test]
fn agent_continue_dry_run_builds_handoff_prompt_from_previous_session() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let transcript_dir = tempfile::tempdir().unwrap();
    let transcript_path = transcript_dir.path().join("rollout.jsonl");
    let content = [
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-03-27T10:00:00Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Implement OAuth flow"}]
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-03-27T10:00:01Z",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Working on auth.rs"}]
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-03-27T10:00:02Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Add unit tests"}]
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-03-27T10:00:03Z",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Tests added"}]
            }
        }),
    ]
    .iter()
    .map(serde_json::Value::to_string)
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&transcript_path, content).unwrap();

    let _ = h.run(["list", "--json"]);
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    conn.execute(
        "INSERT INTO instances (name, created_at, transcript_path, tool, status, status_context) VALUES ('solo', 1000, ?1, 'codex', 'inactive', 'exit:0')",
        [transcript_path.to_str().unwrap()],
    )
    .expect("insert instance");

    let (code, stdout, stderr) = h.run([
        "agent",
        "solo",
        "--cli",
        "claude",
        "--continue",
        "--dry-run",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("claude --as solo"), "stdout={stdout}");
    assert!(stdout.contains("--hcom-prompt"), "stdout={stdout}");
    assert!(stdout.contains("Implement OAuth flow"), "stdout={stdout}");
    assert!(stdout.contains("Add unit tests"), "stdout={stdout}");

    std::fs::write(
        h.path().join("agents.json"),
        r#"{"cli_overrides":{"codex":"claude"},"agents":{"solo":{"dir":"/tmp","cli":"codex","resume":true}}}"#,
    ).unwrap();
    let (code, stdout, stderr) = h.run(["agent", "solo", "--restart", "--continue", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("claude --as solo"), "stdout={stdout}");
    assert!(stdout.contains("Implement OAuth flow"), "stdout={stdout}");
    assert!(stdout.contains("Previous tool: `codex`"), "stdout={stdout}");

    let (code, show_out, stderr) = h.run(["agent", "show", "solo", "--continue"]);
    assert_eq!(code, 0, "stdout={show_out} stderr={stderr}");
    assert!(
        show_out.contains("start:     continue"),
        "stdout={show_out}"
    );

    let (code, stdout_last, stderr) = h.run([
        "agent",
        "solo",
        "--cli",
        "claude",
        "--continue",
        "--last",
        "1",
        "--dry-run",
        "--hcom-prompt",
        "finish now",
    ]);
    assert_eq!(code, 0, "stdout={stdout_last} stderr={stderr}");
    assert!(
        stdout_last.contains("Recent Activity (last 1 exchanges)"),
        "stdout={stdout_last}"
    );
    assert!(stdout_last.contains("finish now"), "stdout={stdout_last}");
}

#[test]
fn agent_show_materializes_roaming_agent_for_the_current_project() {
    let h = Hcom::new();
    let project = h.root_path().join("forecast-service");
    let nested = project.join("src/jobs");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::create_dir_all(&nested).expect("create nested working directory");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"}}}"#,
    )
    .expect("write catalog");

    let output = h
        .cmd()
        .current_dir(&nested)
        .args(["agent", "show", "reviewer"])
        .output()
        .expect("show roaming agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    let instance = roaming_instance("reviewer_forecast_service", &project);
    assert!(
        stdout.contains(&format!("name:      {instance}")),
        "stdout={stdout}"
    );
    assert!(stdout.contains("roaming:   true"));
    assert!(
        stdout.contains(&format!("dir:       {}", project.display())),
        "stdout={stdout}"
    );
    assert!(stdout.contains(&format!("codex --as {instance}")));
}

/// The instance name hcom gives a roaming agent: `<prefix>_<3-digit number from the root path hash>`.
fn roaming_instance(prefix: &str, root: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let root = std::fs::canonicalize(root).expect("canonical project root");
    let digest = Sha256::digest(root.as_os_str().as_encoded_bytes());
    let value = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    format!("{prefix}_{}", value % 900 + 100)
}

#[test]
fn roaming_agent_rejects_catalog_static_placement() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"dir":"/tmp"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "reviewer"]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("roaming agent 'reviewer' cannot set dir"),
        "stderr={stderr}"
    );
}

#[test]
fn agent_dry_run_selects_the_effective_cli_tool_profile() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"claude","args":["--common"],
            "tools":{"claude":{"model":"sonnet","args":["--agent","reviewer"]},
                     "codex":{"model":"gpt-5","args":["--sandbox","workspace-write"]}}}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--cli", "codex", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("codex --as solo"), "stdout={stdout}");
    assert!(stdout.contains("--model gpt-5"), "stdout={stdout}");
    assert!(stdout.contains("--common"), "stdout={stdout}");
    assert!(
        stdout.contains("--sandbox workspace-write"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("--agent reviewer"), "stdout={stdout}");
}

#[test]
fn agent_show_uses_configured_herdr_placement_instead_of_parent_placement() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"defaults":{"cli":"codex","session":"child-space"},"agents":{"solo":{"dir":"/tmp"}}}"#,
    )
    .expect("write catalog");

    let mut cmd = h.cmd();
    cmd.env("HCOM_TERMINAL", "herdr")
        .env("HCOM_HERDR_WORKSPACE", "parent-space")
        .env("HCOM_HERDR_TAB", "parent-tab")
        .args(["agent", "show", "solo"]);
    let output = cmd.output().expect("run agent show");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("terminal:  preset herdr (hcom config default)"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("HCOM_HERDR_WORKSPACE=child-space"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("HCOM_HERDR_TAB=solo"), "stdout={stdout}");
    assert!(!stdout.contains("parent-space"), "stdout={stdout}");
    assert!(!stdout.contains("parent-tab"), "stdout={stdout}");
}

#[test]
fn agent_with_terminal_preset_opens_its_own_surface() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "solo":{"dir":"/tmp","cli":"codex","terminal":"tmux","session":"work"},
            "inplace":{"dir":"/tmp","cli":"codex","terminal":"here"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("--terminal tmux --no-run-here --go"),
        "stdout={stdout}"
    );
    assert!(
        stderr.contains("session 'work' ignored: terminal preset 'tmux' is not herdr"),
        "stderr={stderr}"
    );

    let (code, stdout, stderr) = h.run(["agent", "inplace", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("--terminal here --go"), "stdout={stdout}");
    assert!(!stdout.contains("--no-run-here"), "stdout={stdout}");
}

#[test]
fn agent_catalog_rejects_removed_pre_field() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"codex","pre":"echo ready"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.contains("unknown field `pre`"), "stderr={stderr}");
}

#[cfg(unix)]
#[test]
fn kill_by_group_matches_members_running_under_another_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "worker_a":{"dir":"/tmp","cli":"claude","groups":["workers"]},
            "worker_b":{"dir":"/tmp","cli":"gemini","groups":["workers"]}
        }}"#,
    )
    .expect("write catalog");
    // Make sure the schema exists before inserting rows directly.
    let (code, stdout, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    // worker_a runs as `rev2` (`hcom agent worker_a --as rev2`).
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances (name, status, status_time, created_at, tool, pid, launch_context) \
         VALUES ('rev2', 'listening', ?1, ?1, 'claude', 999991, '{\"agent_name\":\"worker_a\"}')",
        rusqlite::params![now],
    )
    .expect("insert rev2");

    // worker_b survives only as a tracked orphan process named `rev3`.
    let mut child = Command::new("sh")
        .args(["-c", "exec sleep 60"])
        .process_group(0)
        .spawn()
        .expect("spawn orphan process group");
    let pid = child.id();
    h.track_cleanup_pid(i64::from(pid));
    std::fs::create_dir_all(h.hcom_dir.join(".tmp")).expect("create .tmp");
    std::fs::write(
        h.hcom_dir.join(".tmp/launched_pids.json"),
        format!(
            r#"{{"{pid}":{{"tool":"gemini","names":["rev3"],"launched_at":0,"agent_name":"worker_b"}}}}"#
        ),
    )
    .expect("write pidfile");

    let (code, stdout, stderr) = h.run(["kill", "@workers"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("'rev2'"), "stdout={stdout}");
    assert!(
        stdout.contains("leftover process of stopped agent 'rev3'"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Killed 2 (group:@workers)"),
        "stdout={stdout}"
    );
    let status = child.wait().expect("reap orphan");
    assert!(!status.success(), "orphan should be terminated: {status}");
}

#[test]
fn send_help_explains_autostart_acknowledgement() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["send", "--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("first event's acknowledgement"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("queued/pending"), "stdout={stdout}");
    assert!(stdout.contains("roaming"), "stdout={stdout}");
    assert!(
        stdout.contains("<name>_<project>_<hash>"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("Running agents route first"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("catalog group 'review'"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["--help"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("send         Send message (@name, @tag-, @group)"),
        "stdout={stdout}"
    );
}

#[test]
fn agent_cli_overrides_apply_to_message_autostart() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"cli_overrides":{"codex":"claude"},"agents":{"reviewer":{"dir":"/tmp","cli":"codex",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "--intent",
        "request",
        "--",
        "review this",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Queued; delivery pending:"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("Sent to:"), "stdout={stdout}");
    assert!(stdout.contains("reviewer"), "stdout={stdout}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "review this");

    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let reviewer = instances
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == "reviewer"))
        .expect("autostarted reviewer instance");
    assert_eq!(reviewer["unread_count"], 1);
    assert_eq!(reviewer["tool"], "claude");
}

#[test]
fn targeted_send_starts_catalog_agent_and_reports_unacknowledged_message_pending() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"codex",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "--intent",
        "request",
        "--",
        "review this",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Queued; delivery pending:"),
        "stdout={stdout}"
    );
    assert!(!stdout.contains("Sent to:"), "stdout={stdout}");
    assert!(stdout.contains("reviewer"), "stdout={stdout}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "review this");

    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let reviewer = instances
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == "reviewer"))
        .expect("autostarted reviewer instance");
    assert_eq!(reviewer["unread_count"], 1);
}

#[test]
fn targeted_send_to_catalog_group_starts_only_in_scope_members() {
    let h = Hcom::new();
    let imported = h.path().join("project-agents.json");
    std::fs::write(
        &imported,
        r#"{"agents":{
            "public":{"dir":"/tmp","cli":"codex","groups":["crew"],
                "terminal_command":"sh -c true {script}"},
            "private":{"dir":"/tmp","cli":"codex","groups":["crew"],
                "terminal_command":"sh -c true {script}"}
        }}"#,
    )
    .expect("write imported catalog");
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"imports":[{{"from":{},"agents":["public"]}}]}}"#,
            serde_json::to_string(&imported).unwrap()
        ),
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@crew",
        "--intent",
        "request",
        "--",
        "check this",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("public"), "stdout={stdout}");
    assert!(
        !stdout.contains("private") && !stderr.contains("private"),
        "stdout={stdout} stderr={stderr}"
    );

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "check this");
    let names = |h: &Hcom| {
        let (code, instances, stderr) = h.run(["list", "--json"]);
        assert_eq!(code, 0, "instances={instances} stderr={stderr}");
        let instances: serde_json::Value =
            serde_json::from_str(&instances).expect("instance list JSON");
        let mut names: Vec<String> = instances
            .as_array()
            .expect("instance array")
            .iter()
            .filter_map(|item| item["name"].as_str().map(str::to_string))
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&h), vec!["public"]);

    // A catalog agent named like the group keeps precedence.
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "team":{"dir":"/tmp","cli":"codex","terminal_command":"sh -c true {script}"},
            "member":{"dir":"/tmp","cli":"codex","groups":["team"],
                "terminal_command":"sh -c true {script}"}
        }}"#,
    )
    .expect("write catalog");
    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@team", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert_eq!(names(&h), vec!["public", "team"]);

    // So does a running instance named like the group.
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{
            "member":{"dir":"/tmp","cli":"codex","groups":["lead"],
                "terminal_command":"sh -c true {script}"}
        }}"#,
    )
    .expect("write catalog");
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances (name, status, status_time, created_at, tool) \
         VALUES ('lead', 'listening', ?1, ?1, 'claude')",
        rusqlite::params![now],
    )
    .expect("insert lead");
    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@lead", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert_eq!(names(&h), vec!["lead", "public", "team"]);
}

#[test]
fn targeted_send_with_spawn_as_starts_catalog_agent_under_custom_instance_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"codex",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "--spawn-as",
        "reviewer_fix",
        "--intent",
        "request",
        "--",
        "review this bug",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Queued; delivery pending:"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("reviewer_fix"), "stdout={stdout}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "review this bug");

    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let clone = instances
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == "reviewer_fix"))
        .expect("autostarted reviewer_fix instance");
    assert_eq!(clone["unread_count"], 1);

    // Sending again with --as alias to the already running clone does not relaunch
    let (code2, stdout2, stderr2) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "--as",
        "reviewer_fix",
        "--",
        "second message to clone",
    ]);
    assert_eq!(code2, 0, "stdout={stdout2} stderr={stderr2}");
    assert!(stdout2.contains("reviewer_fix"), "stdout={stdout2}");
}

#[test]
fn targeted_send_with_spawn_as_validates_target_and_name() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"dir":"/tmp","cli":"codex",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    // Invalid base name
    let (code, _stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "--spawn-as",
        "Bad-Name!",
        "--",
        "msg",
    ]);
    assert_ne!(code, 0, "invalid name must fail");
    assert!(stderr.contains("Invalid instance name"), "stderr={stderr}");

    // Multiple targets with spawn-as
    let (code, _stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@reviewer",
        "@other",
        "--spawn-as",
        "clone_agent",
        "--",
        "msg",
    ]);
    assert_ne!(code, 0, "multiple targets must fail");
    assert!(
        stderr.contains("exactly one target agent"),
        "stderr={stderr}"
    );

    // Broadcast with spawn-as
    let (code, _stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "--spawn-as",
        "clone_agent",
        "--",
        "msg",
    ]);
    assert_ne!(code, 0, "broadcast with spawn-as must fail");
    assert!(
        stderr.contains("exactly one target agent"),
        "stderr={stderr}"
    );

    // Non-catalog target with spawn-as
    let (code, _stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@nonexistent",
        "--spawn-as",
        "clone_agent",
        "--",
        "msg",
    ]);
    assert_ne!(code, 0, "nonexistent catalog agent must fail");
    assert!(stderr.contains("Unknown catalog agent"), "stderr={stderr}");
}

#[test]
fn targeted_send_materializes_roaming_agent_in_the_senders_git_root() {
    let h = Hcom::new();
    let project = h.root_path().join("weather-app");
    let nested = project.join("services/api");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::create_dir_all(&nested).expect("create nested working directory");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex",
            "env":{"DIPPY_CONFIG_ONLY":"/tmp/roaming-senior.dippy"},
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let output = h
        .cmd()
        .current_dir(&nested)
        .args([
            "send",
            "--from",
            "bigboss",
            "@reviewer",
            "--intent",
            "request",
            "--",
            "review this",
        ])
        .output()
        .expect("send to roaming agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    let instance = roaming_instance("reviewer_weather_app", &project);
    assert!(stdout.contains(&instance), "stdout={stdout}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1", "--full"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "review this");
    assert_eq!(event["data"]["mentions"], serde_json::json!([instance]));
    assert_eq!(event["data"]["delivered_to"], serde_json::json!([instance]));

    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let reviewer = instances
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == instance.as_str()))
        .expect("materialized reviewer instance");
    assert_eq!(reviewer["directory"], project.to_string_lossy().as_ref());

    let launch_dir = h.path().join(".tmp/launch");
    let launch_files = std::fs::read_dir(&launch_dir)
        .expect("read launch directory")
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        launch_files.contains("DIPPY_CONFIG_ONLY")
            && launch_files.contains("/tmp/roaming-senior.dippy"),
        "catalog env did not reach launched process: {launch_files}"
    );
}

#[test]
fn roaming_send_from_an_agent_uses_its_recorded_directory_not_process_cwd() {
    let h = Hcom::new();
    let project = h.root_path().join("agent-project");
    let other = h.root_path().join("unrelated-cwd");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::create_dir_all(&other).expect("create unrelated directory");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let sender = h.start_with_process_id("roaming-sender-process");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    conn.execute(
        "UPDATE instances SET directory = ?1 WHERE name = ?2",
        rusqlite::params![project.to_string_lossy().as_ref(), sender],
    )
    .expect("move sender context to project");
    let instance = roaming_instance("reviewer_agent_project", &project);
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at) \
         VALUES (?1, 'listening', ?2, 1000.0)",
        [instance.as_str(), project.to_string_lossy().as_ref()],
    )
    .expect("insert materialized reviewer");

    let output = h
        .cmd()
        .current_dir(&other)
        .env("HCOM_PROCESS_ID", "roaming-sender-process")
        .args(["send", "@reviewer", "--", "inspect"])
        .output()
        .expect("send from registered agent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(&instance), "stdout={stdout}");
    assert!(
        !stdout.contains("reviewer_unrelated_cwd"),
        "stdout={stdout}"
    );
}

#[test]
fn roaming_send_from_same_basename_roots_addresses_distinct_instances() {
    let h = Hcom::new();
    let first = h.root_path().join("one/app");
    let second = h.root_path().join("two/app");
    std::fs::create_dir_all(first.join(".git")).expect("create first git marker");
    std::fs::create_dir_all(second.join(".git")).expect("create second git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let first_instance = roaming_instance("reviewer_app", &first);
    let second_instance = roaming_instance("reviewer_app", &second);
    assert_ne!(first_instance, second_instance);
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    for (instance, root) in [(&first_instance, &first), (&second_instance, &second)] {
        conn.execute(
            "INSERT INTO instances (name, status, directory, created_at) \
             VALUES (?1, 'listening', ?2, 1000.0)",
            [instance.as_str(), root.to_string_lossy().as_ref()],
        )
        .expect("insert project reviewer");
    }

    for (root, own, other) in [
        (&first, &first_instance, &second_instance),
        (&second, &second_instance, &first_instance),
    ] {
        let output = h
            .cmd()
            .current_dir(root)
            .args(["send", "--from", "bigboss", "@reviewer", "--", "inspect"])
            .output()
            .expect("send from project");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
        assert!(
            stdout.contains(own.as_str()) && !stdout.contains(other.as_str()),
            "stdout={stdout}"
        );
        assert!(
            stdout.contains(&format!(
                "Note: @reviewer is a roaming agent; in this project it is {own} (use that name)."
            )),
            "stdout={stdout}"
        );
        assert!(!stderr.contains("roaming"), "stderr={stderr}");
    }
}

#[test]
fn roaming_send_autostarts_own_project_instance_while_another_project_runs_one() {
    let h = Hcom::new();
    let here = h.root_path().join("cca-manager");
    let elsewhere = h.root_path().join("proweb");
    std::fs::create_dir_all(here.join(".git")).expect("create first git marker");
    std::fs::create_dir_all(elsewhere.join(".git")).expect("create second git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");
    let own = roaming_instance("reviewer_cca_manager", &here);
    let other = roaming_instance("reviewer_proweb", &elsewhere);
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at) \
         VALUES (?1, 'listening', ?2, 1000.0)",
        [other.as_str(), elsewhere.to_string_lossy().as_ref()],
    )
    .expect("insert other project reviewer");

    let output = h
        .cmd()
        .current_dir(&here)
        .args(["send", "--from", "bigboss", "@reviewer", "--", "inspect"])
        .output()
        .expect("send from project");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains(&format!(
            "Note: @reviewer is a roaming agent; in this project it is {own} (use that name)."
        )),
        "stdout={stdout}"
    );
    assert!(!stderr.contains("roaming"), "stderr={stderr}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1", "--full"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["delivered_to"], serde_json::json!([own]));
    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let started = instances
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == own.as_str()))
        .expect("own project reviewer started");
    assert_eq!(started["directory"], here.to_string_lossy().as_ref());
}

#[test]
fn listings_mark_roaming_instances_with_their_catalog_agent() {
    let h = Hcom::new();
    let project = h.root_path().join("app");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"},
            "auditor":{"roaming":true,"cli":"codex"},"solo":{"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let instance = roaming_instance("reviewer_app", &project);
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    for (name, agent) in [
        (instance.as_str(), "reviewer"),
        ("helper", "solo"),
        ("plain", ""),
    ] {
        conn.execute(
            "INSERT INTO instances \
             (name, status, status_time, created_at, tool, directory, launch_context) \
             VALUES (?1, 'listening', ?2, ?2, 'codex', ?3, ?4)",
            rusqlite::params![
                name,
                now,
                project.to_string_lossy(),
                format!(r#"{{"agent_name":"{agent}"}}"#),
            ],
        )
        .expect("insert instance");
    }

    let (code, stdout, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let line = |name: &str| {
        stdout
            .lines()
            .find(|line| line.contains(&format!(" {name}")))
            .unwrap_or_else(|| panic!("no line for {name}: {stdout}"))
            .to_string()
    };
    assert!(
        line(&instance).contains(&format!("{instance} ↻reviewer ")),
        "stdout={stdout}"
    );
    assert!(!line("helper").contains('↻'), "stdout={stdout}");
    assert!(!line("plain").contains('↻'), "stdout={stdout}");
    // The badge widens the name column (padded by at least two spaces):
    // status text stays aligned.
    let status_column = |line: String, name: &str| {
        let gap = line.find(name).unwrap() + line[line.find(name).unwrap()..].find("  ").unwrap();
        let status = gap + line[gap..].find(|c: char| c != ' ').unwrap();
        line[..status].chars().count()
    };
    assert_eq!(
        status_column(line(&instance), &instance),
        status_column(line("helper"), "helper"),
        "stdout={stdout}"
    );

    let (code, stdout, stderr) = h.run(["agent", "list", "--for-humans"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let row = |name: &str| {
        stdout
            .lines()
            .find(|line| line.starts_with(&format!("{name} ")))
            .unwrap_or_else(|| panic!("no row for {name}: {stdout}"))
            .to_string()
    };
    assert!(
        row("reviewer").contains(&format!("● 1 roaming: {instance}")),
        "stdout={stdout}"
    );
    assert!(row("reviewer").contains("<project>"), "stdout={stdout}");
    assert!(row("auditor").contains("○ -"), "stdout={stdout}");
    assert!(row("solo").contains("○ -"), "stdout={stdout}");

    let (code, stdout, stderr) = h.run(["agent", "list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let agents: serde_json::Value = serde_json::from_str(&stdout).expect("agent list JSON");
    let entry = |name: &str| {
        agents
            .as_array()
            .and_then(|items| items.iter().find(|item| item["name"] == name))
            .cloned()
            .unwrap_or_else(|| panic!("no entry for {name}: {stdout}"))
    };
    assert_eq!(
        entry("reviewer")["instances"],
        serde_json::json!([instance])
    );
    assert_eq!(entry("auditor")["instances"], serde_json::json!([]));
    assert!(entry("solo").get("instances").is_none(), "stdout={stdout}");
    assert!(entry("reviewer")["status"].is_null(), "stdout={stdout}");

    // An exact running `reviewer` does not hide the roaming instances, and a
    // dead roaming row (exited, no process) does not count as running.
    let auditor = roaming_instance("auditor_app", &project);
    for (name, agent, status, context) in [
        ("reviewer", "reviewer", "listening", ""),
        (auditor.as_str(), "auditor", "inactive", "exit:closed"),
    ] {
        conn.execute(
            "INSERT INTO instances (name, status, status_context, status_time, created_at, \
             tool, directory, launch_context) VALUES (?1, ?2, ?3, ?4, ?4, 'codex', ?5, ?6)",
            rusqlite::params![
                name,
                status,
                context,
                now,
                project.to_string_lossy(),
                format!(r#"{{"agent_name":"{agent}"}}"#),
            ],
        )
        .expect("insert instance");
    }
    let (code, stdout, stderr) = h.run(["agent", "list", "--for-humans"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let row = |name: &str| {
        stdout
            .lines()
            .find(|line| line.starts_with(&format!("{name} ")))
            .unwrap_or_else(|| panic!("no row for {name}: {stdout}"))
            .to_string()
    };
    assert!(
        row("reviewer").contains(&format!("● 1 roaming: {instance}")),
        "stdout={stdout}"
    );
    assert!(row("auditor").contains("○ -"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["agent", "list", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let agents: serde_json::Value = serde_json::from_str(&stdout).expect("agent list JSON");
    let auditor_entry = agents
        .as_array()
        .and_then(|items| items.iter().find(|item| item["name"] == "auditor"))
        .expect("auditor entry");
    assert_eq!(auditor_entry["instances"], serde_json::json!([]));
    let dead_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM instances WHERE name = ?1",
            [auditor.as_str()],
            |row| row.get(0),
        )
        .expect("count dead row");
    assert_eq!(dead_rows, 1, "the dead roaming row must still exist");
}

#[test]
fn send_by_exact_or_non_roaming_name_prints_no_roaming_note() {
    let h = Hcom::new();
    let project = h.root_path().join("app");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"},"solo":{"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let instance = roaming_instance("reviewer_app", &project);
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    for name in [instance.as_str(), "solo"] {
        conn.execute(
            "INSERT INTO instances (name, status, directory, created_at) \
             VALUES (?1, 'listening', ?2, 1000.0)",
            [name, project.to_string_lossy().as_ref()],
        )
        .expect("insert instance");
    }

    for target in [format!("@{instance}"), "@solo".to_string()] {
        let output = h
            .cmd()
            .current_dir(&project)
            .args([
                "send",
                "--from",
                "bigboss",
                target.as_str(),
                "--",
                "inspect",
            ])
            .output()
            .expect("send");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
        assert!(!stdout.contains("Note:"), "target={target} stdout={stdout}");
    }
}

#[test]
fn agent_launch_of_a_roaming_agent_notes_the_project_instance() {
    let h = Hcom::new();
    let project = h.root_path().join("forecast-service");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let instance = roaming_instance("reviewer_forecast_service", &project);

    // --resume has nothing to resume here and fails as before, but takes the
    // same launch path, so it still prints the note.
    for (args, succeeds) in [
        (vec!["agent", "reviewer", "--dry-run"], true),
        (vec!["agent", "reviewer", "--resume", "--dry-run"], false),
    ] {
        let output = h
            .cmd()
            .current_dir(&project)
            .args(&args)
            .output()
            .expect("launch roaming agent");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.success(),
            succeeds,
            "stdout={stdout} stderr={stderr}"
        );
        assert!(
            stdout.contains(&format!(
                "Note: reviewer is a roaming agent; in this project it runs as {instance}."
            )),
            "args={args:?} stdout={stdout}"
        );
        assert!(!stderr.contains("roaming"), "stderr={stderr}");
    }
}

#[test]
fn roaming_send_rejects_an_instance_name_recorded_for_another_root() {
    let h = Hcom::new();
    let first = h.root_path().join("one/app");
    let second = h.root_path().join("two/app");
    std::fs::create_dir_all(first.join(".git")).expect("create first git marker");
    std::fs::create_dir_all(second.join(".git")).expect("create second git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex"}}}"#,
    )
    .expect("write catalog");
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at) \
         VALUES (?1, 'stopped', ?2, 1000.0)",
        [
            roaming_instance("reviewer_app", &second).as_str(),
            first.to_string_lossy().as_ref(),
        ],
    )
    .expect("insert first project reviewer");

    let output = h
        .cmd()
        .current_dir(&second)
        .args(["send", "--from", "bigboss", "@reviewer", "--", "inspect"])
        .output()
        .expect("send from colliding project");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("roaming instance name collides with another project root"),
        "stderr={stderr}"
    );
}

#[test]
fn roaming_slug_collision_blocks_only_sends_that_address_that_agent() {
    let h = Hcom::new();
    let first = h.root_path().join("one/app");
    let second = h.root_path().join("two/app");
    std::fs::create_dir_all(first.join(".git")).expect("create first git marker");
    std::fs::create_dir_all(second.join(".git")).expect("create second git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex","tag":"review"}}}"#,
    )
    .expect("write catalog");
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    let taken = roaming_instance("reviewer_app", &second);
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at, tag) \
         VALUES (?1, 'stopped', ?2, 1000.0, 'review'), \
                ('nova', 'listening', ?2, 1000.0, 'astro')",
        [taken.as_str(), first.to_string_lossy().as_ref()],
    )
    .expect("insert instances");
    let taken_target = format!("@{taken}");

    let send = |args: &[&str]| {
        let output = h
            .cmd()
            .current_dir(&second)
            .args(args)
            .output()
            .expect("send from colliding project");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    let (ok, stdout, stderr) = send(&["send", "--from", "bigboss", "@nova", "--", "unrelated"]);
    assert!(ok, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("nova"), "stdout={stdout}");

    let (ok, stdout, stderr) = send(&["send", "--from", "bigboss", "@astro-", "--", "tagged"]);
    assert!(ok, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("nova"), "stdout={stdout}");

    for args in [
        &["send", "--from", "bigboss", "@reviewer inline"][..],
        &["send", "--from", "bigboss", "@review-", "--", "tagged"][..],
        &[
            "send",
            "--from",
            "bigboss",
            "@review-reviewer",
            "--",
            "full name",
        ][..],
        &["send", "--from", "bigboss", &taken_target, "--", "instance"][..],
    ] {
        let (ok, stdout, stderr) = send(args);
        assert!(!ok, "args={args:?} stdout={stdout} stderr={stderr}");
        assert!(
            stderr.contains("roaming instance name collides with another project root"),
            "args={args:?} stderr={stderr}"
        );
    }
}

#[test]
fn send_to_a_running_agent_does_not_read_the_catalog() {
    let h = Hcom::new();
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    conn.execute(
        "INSERT INTO instances (name, status, created_at, tag) \
         VALUES ('nova', 'listening', 1000.0, 'astro')",
        [],
    )
    .expect("insert running agent");
    std::fs::write(h.path().join("agents.json"), "{not json").expect("write broken catalog");

    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@nova", "--", "hello"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("nova"), "stdout={stdout}");
    assert!(!stderr.contains("catalog"), "stderr={stderr}");

    let (code, stdout, stderr) = h.run([
        "send", "--from", "bigboss", "--thread", "empty", "--", "hello",
    ]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.contains("has no members"), "stderr={stderr}");
    assert!(!stderr.contains("catalog"), "stderr={stderr}");

    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@astro-", "--", "hello"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("nova"), "stdout={stdout}");
    assert!(
        stderr.contains("Warning: cannot load agent catalog"),
        "stderr={stderr}"
    );

    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@ghost", "--", "hello"]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.contains("@ghost"), "stderr={stderr}");
}

#[test]
fn catalog_autostart_failure_does_not_write_message() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"broken":{"dir":"/tmp","cli":"not-a-real-tool",
            "terminal_command":"sh -c true {script}"}}}"#,
    )
    .expect("write catalog");

    let (code, _stdout, stderr) = h.run([
        "send",
        "--from",
        "bigboss",
        "@broken",
        "--",
        "must not be queued",
    ]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("could not start catalog agent 'broken'"),
        "stderr={stderr}"
    );

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    assert!(
        events.trim().is_empty(),
        "failed send wrote message: {events}"
    );
}

#[test]
fn roaming_slug_collision_blocks_thread_only_send_to_that_agent() {
    let h = Hcom::new();
    let first = h.root_path().join("one/app");
    let second = h.root_path().join("two/app");
    std::fs::create_dir_all(first.join(".git")).expect("create first git marker");
    std::fs::create_dir_all(second.join(".git")).expect("create second git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"reviewer":{"roaming":true,"cli":"codex","tag":"review"}}}"#,
    )
    .expect("write catalog");
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let conn = rusqlite::Connection::open(h.path().join("hcom.db")).expect("open hcom db");
    let taken = roaming_instance("reviewer_app", &second);
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at, tag) \
         VALUES (?1, 'stopped', ?2, 1000.0, 'review')",
        [taken.as_str(), first.to_string_lossy().as_ref()],
    )
    .expect("insert stopped roaming instance");
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)",
        [
            format!("events_sub:thread-review-{taken}"),
            format!(
                r#"{{"caller":"{taken}","thread_name":"review","auto_thread_member":true,"delivery_only":true,"sql":"0","created":1.0,"last_id":0,"once":false}}"#
            ),
        ],
    )
    .expect("insert thread membership");

    let output = h
        .cmd()
        .current_dir(&second)
        .args([
            "send", "--from", "bigboss", "--thread", "review", "--", "continue",
        ])
        .output()
        .expect("thread send from colliding project");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("roaming instance name collides with another project root"),
        "stderr={stderr}"
    );
}

/// A `start --as` rebind inherits the previous cursor, so a message queued for
/// the dead session arrives as normal unread and `listen --missed` must not
/// repeat it. (A fresh launch, which does miss it, is covered by the
/// `missed_messages_*` unit test.)
#[test]
fn listen_missed_returns_what_a_fresh_launch_skipped() {
    let h = Hcom::new();
    let (code, stdout, _) = h.run(["listen", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--missed"), "help={stdout}");

    let name = h.start_with_process_id("missed-first");
    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "gateway-test",
        &format!("@{name}"),
        "--intent",
        "request",
        "--",
        "lost task",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let (code, stdout, stderr) = h.run(["stop", &name]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let (code, stdout, stderr) = h.run_as_process("missed-second", ["start", "--as", &name]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    // Drain normal delivery so only the --missed path can print the message.
    h.run(["listen", "--name", &name, "--timeout", "1", "--json"]);

    let (code, stdout, stderr) = h.run(["listen", "--missed", "--name", &name]);
    assert_eq!(code, 1, "no launch environment: stdout={stdout}");
    assert!(stderr.contains("hcom-launched session"), "stderr={stderr}");

    let (code, _, stderr) = h.run(["listen", "--missed", "--from", "nova", "--name", &name]);
    assert_eq!(code, 1, "stderr={stderr}");
    assert!(stderr.contains("event filters"), "stderr={stderr}");

    // Fixture, not a real launch: the `start --as` identity above stands in
    // for the new session, and these two variables are what the launcher
    // would set for a fresh launch that began reading after the message.
    let (_, last, _) = h.run(["events", "--last", "1"]);
    let start = serde_json::from_str::<serde_json::Value>(last.trim()).expect("event")["id"]
        .as_i64()
        .expect("id");
    let mut command = h.cmd();
    command
        .env("HCOM_INSTANCE_NAME", &name)
        .env("HCOM_LAUNCH_EVENT_ID", start.to_string())
        .args(["listen", "--missed", "--name", &name]);
    let output = command.output().expect("run listen --missed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "stdout={stdout}");
    assert!(stdout.contains("[request #"), "stdout={stdout}");
    assert!(stdout.contains("lost task"), "stdout={stdout}");
}

#[test]
#[cfg(unix)]
fn orca_kill_closes_the_exact_persisted_terminal_handle() {
    use std::os::unix::fs::PermissionsExt;

    let h = Hcom::new();
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");

    let fake_bin = h.root_path().join("fake-orca-bin");
    std::fs::create_dir_all(&fake_bin).expect("create fake Orca bin");
    let capture = h.root_path().join("orca-close-argv");
    let fake_orca = fake_bin.join("orca");
    std::fs::write(
        &fake_orca,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$ORCA_CAPTURE_PATH\"\nprintf '%s\\n' '{\"id\":\"cli:terminal:close\",\"ok\":true,\"result\":{\"close\":{\"handle\":\"term:runtime:42\",\"tabId\":\"tab:1\",\"ptyKilled\":true}}}'\n",
    )
    .expect("write fake Orca CLI");
    let mut permissions = std::fs::metadata(&fake_orca)
        .expect("stat fake Orca CLI")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_orca, permissions).expect("chmod fake Orca CLI");

    let mut child = Command::new("sh")
        .args(["-c", "sleep 60"])
        .process_group(0)
        .spawn()
        .expect("spawn managed process group");
    let pid = i64::from(child.id());
    h.track_cleanup_pid(pid);
    let reaper = std::thread::spawn(move || child.wait().expect("reap managed process"));

    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances \
         (name, status, status_time, created_at, tool, background, pid, \
          terminal_preset_effective, launch_context) \
         VALUES ('orca-agent', 'active', ?1, ?1, 'codex', 0, ?2, 'orca', ?3)",
        rusqlite::params![
            now,
            pid,
            r#"{"process_id":"proc-orca","pane_id":"term:runtime:42","terminal_id":"term:runtime:42","terminal_preset_effective":"orca"}"#
        ],
    )
    .expect("insert Orca instance fixture");

    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![fake_bin];
    path_entries.extend(std::env::split_paths(&inherited_path));
    let mut cmd = h.cmd();
    cmd.env(
        "PATH",
        std::env::join_paths(path_entries).expect("join fake Orca PATH"),
    )
    .env("ORCA_CAPTURE_PATH", &capture)
    .args(["kill", "orca-agent"]);
    let output = cmd.output().expect("run hcom kill");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("closed orca pane"), "stdout={stdout}");
    assert_eq!(
        std::fs::read_to_string(capture).expect("read captured Orca argv"),
        "terminal\nclose\n--terminal\nterm:runtime:42\n--json\n"
    );
    reaper.join().expect("join managed process reaper");
}

#[test]
fn terminal_help_exposes_orca_preset() {
    let h = Hcom::new();
    let (code, stdout, stderr) = h.run(["config", "terminal", "--info"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("orca"), "stdout={stdout}");
    assert!(stdout.contains("result.terminal.handle"), "stdout={stdout}");
}

#[test]
#[cfg(unix)]
fn orca_direct_launch_uses_local_workspace_and_structured_agent_intent() {
    use std::os::unix::fs::PermissionsExt;

    let h = Hcom::new();
    let fake_bin = h.root_path().join("fake-orca-launch-bin");
    std::fs::create_dir_all(&fake_bin).expect("create fake Orca launch bin");
    let capture = h.root_path().join("orca-create-argv");
    let fake_orca = fake_bin.join("orca");
    std::fs::write(
        &fake_orca,
        r#"#!/bin/sh
if [ "$1" = "status" ]; then
  printf '%s\n' '{"id":"cli:status","ok":true,"result":{"runtime":{"reachable":true,"capabilities":["terminal.create-interactive-agent.v1","terminal.create-folder-workspace.v1"]}}}'
  exit 0
fi
printf '%s\n' "$@" > "$ORCA_CAPTURE_PATH"
printf '%s\n' '{"id":"request-launch","ok":true,"result":{"terminal":{"handle":"term:runtime:launch","worktreeId":"workspace:1","title":"orca_direct"}}}'
"#,
    )
    .expect("write fake Orca launch CLI");
    // Claude's per-run hooks need no CLI preflight, so a stub binary suffices.
    let fake_claude = fake_bin.join("claude");
    std::fs::write(
        &fake_claude,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.0 (Claude Code)'; exit 0; fi\nexit 0\n",
    )
    .expect("write fake Claude CLI");
    for executable in [&fake_orca, &fake_claude] {
        let mut permissions = std::fs::metadata(executable)
            .expect("stat fake executable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(executable, permissions).expect("chmod fake executable");
    }
    let launch_dir = h.workspace.join("project with space-žluťoučký");
    std::fs::create_dir_all(&launch_dir).expect("create Unicode launch directory");
    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![fake_bin];
    path_entries.extend(std::env::split_paths(&inherited_path));
    let mut cmd = h.cmd();
    cmd.env(
        "PATH",
        std::env::join_paths(path_entries).expect("join fake launch PATH"),
    )
    .env("ORCA_CAPTURE_PATH", &capture)
    .env("HCOM_SUBAGENT_TIMEOUT", "1")
    .args([
        "claude",
        "--terminal",
        "orca",
        "--dir",
        launch_dir.to_str().expect("UTF-8 launch dir"),
        "--as",
        "orca_direct",
        "--no-run-here",
    ]);
    let output = cmd.output().expect("run Orca direct launch");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        matches!(output.status.code(), Some(0..=2)),
        "stdout={stdout} stderr={stderr}"
    );
    let captured = std::fs::read_to_string(&capture).expect("read captured create argv");
    let argv: Vec<&str> = captured.lines().collect();
    assert_eq!(&argv[..3], ["terminal", "create", "--worktree"]);
    assert_eq!(
        argv[3],
        format!("path:{}", launch_dir.canonicalize().unwrap().display())
    );
    assert_eq!(argv[4], "--ensure-folder-workspace");
    assert!(
        argv.windows(2)
            .any(|pair| pair == ["--title", "orca_direct"])
    );
    assert!(
        argv.windows(2)
            .any(|pair| pair == ["--interactive-agent", "claude"])
    );
    assert_eq!(argv.last().copied(), Some("--json"));
}

#[test]
#[cfg(unix)]
fn orca_invalid_create_envelope_closes_the_returned_handle() {
    use std::os::unix::fs::PermissionsExt;

    let h = Hcom::new();
    let fake_bin = h.root_path().join("fake-orca-cleanup-bin");
    std::fs::create_dir_all(&fake_bin).expect("create fake Orca cleanup bin");
    let capture = h.root_path().join("orca-cleanup-calls");
    let fake_orca = fake_bin.join("orca");
    std::fs::write(
        &fake_orca,
        r#"#!/bin/sh
if [ "$1" = "status" ]; then
  printf '%s\n' '{"id":"cli:status","ok":true,"result":{"runtime":{"reachable":true,"capabilities":["terminal.create-interactive-agent.v1","terminal.create-folder-workspace.v1"]}}}'
  exit 0
fi
printf '%s\n' "$*" >> "$ORCA_CAPTURE_PATH"
if [ "$1 $2" = "terminal create" ]; then
  printf '%s\n' '{"id":"request-cleanup","ok":false,"result":{"terminal":{"handle":"term:cleanup:42","worktreeId":"workspace:1"}}}'
  exit 0
fi
printf '%s\n' '{"id":"cli:terminal:close","ok":true,"result":{"close":{"handle":"term:cleanup:42","tabId":"tab:1","ptyKilled":true}}}'
"#,
    )
    .expect("write fake Orca cleanup CLI");
    let fake_claude = fake_bin.join("claude");
    std::fs::write(
        &fake_claude,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.0 (Claude Code)'; fi\nexit 0\n",
    )
    .expect("write fake Claude CLI");
    for executable in [&fake_orca, &fake_claude] {
        let mut permissions = std::fs::metadata(executable)
            .expect("stat fake executable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(executable, permissions).expect("chmod fake executable");
    }
    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![fake_bin];
    path_entries.extend(std::env::split_paths(&inherited_path));
    let mut cmd = h.cmd();
    cmd.env(
        "PATH",
        std::env::join_paths(path_entries).expect("join fake cleanup PATH"),
    )
    .env("ORCA_CAPTURE_PATH", &capture)
    .args([
        "claude",
        "--terminal",
        "orca",
        "--as",
        "orca_cleanup",
        "--no-run-here",
    ]);
    let output = cmd.output().expect("run invalid Orca launch");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("unexpected terminal-create response envelope"),
        "stderr={stderr}"
    );
    let calls = std::fs::read_to_string(capture).expect("read Orca cleanup calls");
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("terminal create "))
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "terminal close --terminal term:cleanup:42 --json"),
        "calls={calls}"
    );
}

#[test]
fn config_reports_file_parse_errors_in_get_and_list_modes() {
    let h = Hcom::new();
    std::fs::write(h.hcom_dir.join("config.toml"), "timeout = [broken\n").unwrap();
    for args in [
        vec!["config", "timeout"],
        vec!["config", "terminal"],
        vec!["config", "dev_root"],
        vec!["config"],
        vec!["config", "--json"],
    ] {
        let (code, stdout, stderr) = h.run(args);
        assert_eq!(code, 0, "{stderr}");
        assert!(
            stderr.contains("Invalid config.toml"),
            "stdout={stdout} stderr={stderr}"
        );
    }
}

/// A live process group for an idle-kill target: killing the row signals it.
#[cfg(unix)]
fn idle_kill_sleeper(h: &Hcom) -> std::process::Child {
    let child = Command::new("sleep")
        .arg("120")
        .process_group(0)
        .spawn()
        .expect("spawn sleep");
    h.track_cleanup_pid(i64::from(child.id()));
    child
}

/// Make `name` a headless (no pane) agent that has listened for 100 s, owned
/// by `pid`, with every message read.
#[cfg(unix)]
fn make_idle_kill_row(conn: &rusqlite::Connection, name: &str, status: &str, pid: u32) {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances (name, created_at) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING",
        rusqlite::params![name, now as f64],
    )
    .expect("insert instance");
    conn.execute(
        "UPDATE instances SET status = ?2, status_context = '', status_time = ?3, pid = ?4,
             background = 1, session_id = COALESCE(session_id, 'sess-' || ?1),
             last_event_id = (SELECT COALESCE(MAX(id), 0) FROM events)
         WHERE name = ?1",
        rusqlite::params![name, status, now - 100, pid],
    )
    .expect("make instance idle");
}

/// `(by, reason)` of the newest stopped event for `name`.
#[cfg(unix)]
fn idle_kill_stopped_by(conn: &rusqlite::Connection, name: &str) -> Option<(String, String)> {
    conn.query_row(
        "SELECT json_extract(data, '$.by'), json_extract(data, '$.reason') FROM events
         WHERE type = 'life' AND instance = ?1 AND json_extract(data, '$.action') = 'stopped'
         ORDER BY id DESC LIMIT 1",
        rusqlite::params![name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .ok()
}

#[cfg(unix)]
fn wait_for_idle_kill(h: &Hcom, child: &mut std::process::Child, drive_with_list: bool) {
    h.eventually(
        "idle_kill to end the agent process",
        Duration::from_secs(25),
        || {
            if drive_with_list {
                h.run(["list"]);
            }
            Ok(child.try_wait().map_err(|e| e.to_string())?.map(|_| ()))
        },
    );
}

#[cfg(unix)]
#[test]
fn idle_kill_set_by_config_kills_idle_agent_and_spares_guarded_ones() {
    let h = Hcom::new();
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let mut idle = idle_kill_sleeper(&h);
    let mut blocked = idle_kill_sleeper(&h);
    let mut unread = idle_kill_sleeper(&h);
    let mut waiting = idle_kill_sleeper(&h);
    let mut peer = idle_kill_sleeper(&h);
    make_idle_kill_row(&conn, "peer", "listening", peer.id());
    make_idle_kill_row(&conn, "waiting", "listening", waiting.id());

    // waiting asks peer and gets no reply.
    let (code, stdout, stderr) = h.run([
        "send", "@peer", "--intent", "request", "--name", "waiting", "--", "status?",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    // Named like the kill-all target: only this row may die, not every agent.
    make_idle_kill_row(&conn, "all", "listening", idle.id());
    make_idle_kill_row(&conn, "blocked", "blocked", blocked.id());
    make_idle_kill_row(&conn, "unread", "listening", unread.id());
    make_idle_kill_row(&conn, "waiting", "listening", waiting.id());
    // Arrives after the row was marked read, so it stays unread.
    let (code, stdout, stderr) = h.run(["send", "--from", "bigboss", "@unread", "--", "hi"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    for name in ["all", "blocked", "unread", "waiting"] {
        let (code, stdout, stderr) = h.run(["config", "-i", name, "idle_kill", "1"]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
        assert!(stdout.contains("Set idle_kill"), "stdout={stdout}");
    }
    let (code, stdout, _) = h.run(["config", "-i", "blocked", "idle_kill"]);
    assert_eq!((code, stdout.trim()), (0, "1"));

    wait_for_idle_kill(&h, &mut idle, true);
    h.eventually(
        "stopped event for the idle agent",
        Duration::from_secs(10),
        || Ok(idle_kill_stopped_by(&conn, "all")),
    );
    assert_eq!(
        idle_kill_stopped_by(&conn, "all"),
        Some(("idle_kill".to_string(), "killed".to_string()))
    );

    // The same sweeps evaluated the guarded agents; give any stray kill time to land.
    std::thread::sleep(Duration::from_secs(2));
    for (name, child) in [
        ("blocked", &mut blocked),
        ("unread", &mut unread),
        ("waiting", &mut waiting),
        ("peer", &mut peer),
    ] {
        assert!(
            child.try_wait().expect("try_wait").is_none(),
            "{name} was killed"
        );
        assert_eq!(idle_kill_stopped_by(&conn, name), None, "{name}");
    }

    // 0 clears; invalid values are rejected.
    let (code, stdout, stderr) = h.run(["config", "-i", "blocked", "idle_kill", "0"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("Cleared idle_kill"), "stdout={stdout}");
    for bad in ["abc", "-5", "1.5"] {
        let (code, stdout, stderr) = h.run(["config", "-i", "blocked", "idle_kill", bad]);
        assert_eq!(code, 1, "{bad}: stdout={stdout} stderr={stderr}");
        assert!(
            stderr.contains("idle_kill must be a non-negative integer"),
            "{bad}: stderr={stderr}"
        );
    }
}

#[cfg(unix)]
#[test]
fn idle_kill_is_enforced_by_a_lone_listen_loop_enabled_after_it_started() {
    let h = Hcom::new();
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let name = h.start_listening_with_process_id("pid-idle-cadence");
    let mut agent = idle_kill_sleeper(&h);
    conn.execute(
        "UPDATE instances SET pid = ?2, background = 1 WHERE name = ?1",
        rusqlite::params![name, agent.id()],
    )
    .expect("bind agent process");

    // The listen loop is already in its long wait when the first key appears.
    let started = Instant::now();
    let (code, stdout, stderr) = h.run(["config", "-i", &name, "idle_kill", "1"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    // No other hcom command runs from here on: only the listen loop sweeps.
    wait_for_idle_kill(&h, &mut agent, false);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(14),
        "idle_kill=1 took {elapsed:?}"
    );
    h.eventually(
        "stopped event for the listening agent",
        Duration::from_secs(10),
        || Ok(idle_kill_stopped_by(&conn, &name)),
    );
    assert_eq!(
        idle_kill_stopped_by(&conn, &name),
        Some(("idle_kill".to_string(), "killed".to_string()))
    );
}

#[cfg(unix)]
#[test]
fn idle_killed_catalog_agents_restart_on_send_and_resume() {
    let h = Hcom::new();
    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let project = h.root_path().join("proj");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"defaults":{"idle_kill":1},"agents":{
            "reviewer":{"roaming":true,"cli":"claude","terminal_command":"sh -c true {script}"},
            "keeper":{"dir":"/tmp","cli":"claude","terminal_command":"sh -c true {script}"},
            "never":{"dir":"/tmp","idle_kill":false}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "show", "keeper"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("idle_kill: 1s"), "stdout={stdout}");
    let (code, stdout, stderr) = h.run(["agent", "show", "never"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(!stdout.contains("idle_kill"), "stdout={stdout}");

    let send = |text: &str| {
        let output = h
            .cmd()
            .current_dir(&project)
            .args(["send", "--from", "bigboss", "@reviewer", "--", text])
            .output()
            .expect("send to roaming agent");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "stdout={stdout} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        stdout
    };
    let instance = roaming_instance("reviewer_proj", &project);
    let stdout = send("first");
    assert!(stdout.contains(instance.as_str()), "stdout={stdout}");
    let (code, stdout, _) = h.run(["config", "-i", instance.as_str(), "idle_kill"]);
    assert_eq!((code, stdout.trim()), (0, "1"));

    let mut roaming = idle_kill_sleeper(&h);
    make_idle_kill_row(&conn, instance.as_str(), "listening", roaming.id());
    wait_for_idle_kill(&h, &mut roaming, true);
    h.eventually(
        "stopped event for the roaming agent",
        Duration::from_secs(10),
        || Ok(idle_kill_stopped_by(&conn, instance.as_str())),
    );

    // The next send autostarts it again, with its idle_kill.
    send("second");
    let (code, stdout, stderr) = h.run(["config", "-i", instance.as_str(), "idle_kill"]);
    assert_eq!(
        (code, stdout.trim()),
        (0, "1"),
        "stdout={stdout} stderr={stderr}"
    );

    // A persistent agent killed for idling resumes with hcom r.
    let (code, stdout, stderr) = h.run(["agent", "keeper"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let mut keeper = idle_kill_sleeper(&h);
    make_idle_kill_row(&conn, "keeper", "listening", keeper.id());
    wait_for_idle_kill(&h, &mut keeper, true);
    h.eventually(
        "stopped event for the persistent agent",
        Duration::from_secs(10),
        || Ok(idle_kill_stopped_by(&conn, "keeper")),
    );
    let (code, stdout, stderr) = h.run(["r", "keeper"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
}

/// Spawn `hcom events --follow <args>` and forward its stdout lines over a
/// channel, so tests can wait for each line with a timeout.
fn spawn_follow(
    h: &Hcom,
    args: &[&str],
) -> (std::process::Child, std::sync::mpsc::Receiver<String>) {
    use std::io::BufRead;
    let mut child = h
        .cmd()
        .args(["events", "--follow"])
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn events --follow");
    let stdout = child.stdout.take().expect("follow stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
        {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    (child, rx)
}

fn next_follow_line(rx: &std::sync::mpsc::Receiver<String>) -> serde_json::Value {
    let line = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("events --follow produced no line within 10s");
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("follow line is not JSON ({e}): {line}"))
}

#[test]
fn events_follow_streams_new_matches_and_resumes_from_cursor() {
    let h = Hcom::new();
    let agent = h.start();
    let send = |text: &str| {
        let (code, _, stderr) = h.run(["send", "-b", &format!("@{agent}"), "--", text]);
        assert_eq!(code, 0, "send failed: {stderr}");
    };

    send("first");
    // --last 1 replays the existing message; receiving it proves the follower
    // has fixed its cursor, so the next send is guaranteed to stream.
    let (mut follower, rx) = spawn_follow(&h, &["--type", "message", "--last", "1"]);
    let first = next_follow_line(&rx);
    assert_eq!(first["data"]["text"], "first", "backlog line: {first}");
    send("second");
    let second = next_follow_line(&rx);
    assert_eq!(second["data"]["text"], "second", "streamed line: {second}");
    assert!(second["id"].as_i64() > first["id"].as_i64());
    let _ = follower.kill();
    let _ = follower.wait();

    // A saved cursor replays everything after it, in id order.
    let cursor = first["id"].as_i64().expect("event id").to_string();
    let (mut resumed, rx) = spawn_follow(&h, &["--type", "message", "--after-id", &cursor]);
    let replayed = next_follow_line(&rx);
    assert_eq!(replayed["id"], second["id"], "resumed line: {replayed}");
    let _ = resumed.kill();
    let _ = resumed.wait();
}

#[cfg(unix)]
#[test]
fn events_follow_exits_when_reader_closes() {
    let h = Hcom::new();
    let mut child = h
        .cmd()
        .args(["events", "--follow"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn events --follow");
    // Close the read end without any event arriving: an idle follower must
    // notice on its own instead of waiting for a write to fail.
    drop(child.stdout.take());
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("events --follow kept running after its reader closed");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "follow exit status: {status}");
}

#[test]
fn events_follow_rejects_conflicting_flags() {
    let h = Hcom::new();
    for (args, expected) in [
        (
            vec!["events", "--after-id", "1"],
            "--after-id requires --follow",
        ),
        (
            vec!["events", "--follow", "--wait", "5"],
            "--follow cannot be combined with --wait",
        ),
        (
            vec!["events", "--follow", "--after-id", "1", "--last", "2"],
            "--last and --after-id are mutually exclusive",
        ),
        (
            vec!["events", "--follow", "--after-id", "999999"],
            "is past the newest event",
        ),
    ] {
        let (code, _, stderr) = h.run(&args);
        assert_eq!(code, 1, "{args:?}: stderr={stderr}");
        assert!(stderr.contains(expected), "{args:?}: stderr={stderr}");
    }
}

#[test]
fn events_follow_exits_on_query_error() {
    let h = Hcom::new();
    // An unbound `?` passes prepare but fails at query time; follow must
    // report it instead of polling forever.
    let (code, _, stderr) = h.run(["events", "--follow", "--sql", "type = ?"]);
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(stderr.contains("SQL"), "stderr={stderr}");
}

#[test]
fn transcript_path_reads_a_file_without_an_instance() {
    let h = Hcom::new();
    let item = |role: &str, kind: &str, text: &str| {
        serde_json::json!({"type": "response_item", "timestamp": "2026-03-27T10:00:00Z",
            "payload": {"type": "message", "role": role, "content": [{"type": kind, "text": text}]}})
        .to_string()
    };
    let rollout = h.path().join("rollout-old-session.jsonl");
    let lines = [
        item("user", "input_text", "first prompt"),
        item("assistant", "output_text", "first answer"),
        item("user", "input_text", "second prompt"),
        item("assistant", "output_text", "second answer"),
    ];
    std::fs::write(&rollout, lines.join("\n")).expect("write rollout");
    let rollout = rollout.to_str().expect("utf-8 path");

    // A positional range still works with --path; no instance row is needed.
    let (code, stdout, stderr) = h.run(["transcript", "--path", rollout, "2", "--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let exchanges: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(
        exchanges.as_array().map(Vec::len),
        Some(1),
        "stdout={stdout}"
    );
    assert_eq!(exchanges[0]["position"], 2);
    assert_eq!(exchanges[0]["user"], "second prompt");
    assert_eq!(exchanges[0]["action"], "second answer");

    // An agent name and --path are mutually exclusive.
    let (code, _, stderr) = h.run(["transcript", "luna", "--path", rollout]);
    assert_eq!(code, 1);
    assert!(stderr.contains("takes no agent name"), "stderr={stderr}");

    // Subcommands read the database, so --path must not be silently ignored by them.
    for subcmd in [&["timeline"][..], &["search", "answer"][..]] {
        let mut argv = vec!["transcript", "--path", rollout];
        argv.extend_from_slice(subcmd);
        let (code, _, stderr) = h.run(argv);
        assert_eq!(code, 1, "{subcmd:?}");
        assert!(stderr.contains("does not combine"), "stderr={stderr}");
    }

    // A shared session database needs the agent's session, so it is refused.
    let db = h.path().join("opencode.db");
    std::fs::write(&db, b"").expect("write db");
    let (code, _, stderr) = h.run(["transcript", "--path", db.to_str().expect("utf-8 path")]);
    assert_eq!(code, 1);
    assert!(stderr.contains("holds many sessions"), "stderr={stderr}");

    // An unknown format is an error, not a guessed parser.
    let unknown = h.path().join("notes.txt");
    std::fs::write(&unknown, b"x").expect("write file");
    let (code, _, stderr) = h.run([
        "transcript",
        "--path",
        unknown.to_str().expect("utf-8 path"),
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Unable to determine transcript parser"),
        "stderr={stderr}"
    );
}

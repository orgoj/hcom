//! Hermetic CLI smoke tests: invoke the `hcom` binary in a temp HCOM_DIR and
//! assert exit codes + stdout shape.

mod support;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};
use support::{Hcom, parse_hcom_marker};

#[test]
fn fixture_drop_terminates_registered_process_group() {
    #[cfg(unix)]
    let mut child = Command::new("sh")
        .args(["-c", "sleep 60"])
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
    assert!(stdout.contains("<name>_<project>"), "stdout={stdout}");
}

#[test]
fn transcript_without_model_history_points_to_transport_events() {
    let h = Hcom::new();
    let agent = h.start();

    let (code, stdout, stderr) = h.run(["transcript", &agent]);
    assert_ne!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains(&format!("No model transcript is registered for {agent}.")),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains(&format!("hcom events --participant {agent} --type message")),
        "stderr={stderr}"
    );
    assert!(
        !stderr.contains("no messages have been exchanged"),
        "stderr={stderr}"
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
fn list_json_empty() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("list json");
    let arr = v.as_array().expect("list returns array");
    assert!(arr.is_empty(), "expected empty list, got {stdout}");
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
fn manual_ack_listen_redelivers_until_strict_ack() {
    let h = Hcom::new();
    let recipient = h.start_with_process_id("manual-ack-recipient");
    let (code, stdout, stderr) = h.run([
        "send",
        "--from",
        "gateway-test",
        &format!("@{recipient}"),
        "--intent",
        "request",
        "--thread",
        "bridge-test",
        "--",
        "deliver reliably",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    let receive = || {
        let (code, stdout, stderr) = h.run([
            "listen",
            "--name",
            &recipient,
            "--timeout",
            "1",
            "--json",
            "--manual-ack",
        ]);
        assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
        serde_json::from_str::<serde_json::Value>(stdout.trim()).expect("manual ack envelope")
    };
    let first = receive();
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["from"], "gateway-test");
    assert_eq!(first["to"], recipient);
    assert_eq!(first["text"], "deliver reliably");
    assert_eq!(first["intent"], "request");
    assert_eq!(first["thread"], "bridge-test");
    assert!(first["timestamp"].is_string());
    let event_id = first["event_id"].as_i64().expect("event id");

    let second = receive();
    assert_eq!(second["event_id"].as_i64(), Some(event_id));

    let (code, _stdout, stderr) = h.run(["ack", &(event_id + 1).to_string(), "--name", &recipient]);
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(stderr.contains("first unread event"), "stderr={stderr}");

    let (code, stdout, stderr) = h.run(["ack", &event_id.to_string(), "--name", &recipient]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");

    let (code, stdout, stderr) = h.run([
        "listen",
        "--name",
        &recipient,
        "--timeout",
        "1",
        "--json",
        "--manual-ack",
    ]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.trim().is_empty(),
        "acked event redelivered: {stdout}"
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
fn config_unknown_key_is_not_set() {
    let h = Hcom::new();
    let (code, stdout, _stderr) = h.run(["config", "no_such_key"]);
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
fn list_reconciles_a_reused_tracked_pid_without_signalling_it() {
    let h = Hcom::new();
    let (code, _, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stderr={stderr}");

    let conn = rusqlite::Connection::open(h.hcom_dir.join("hcom.db")).expect("open hcom db");
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO instances \
         (name, status, status_time, created_at, tool, background, pid, launch_context) \
         VALUES ('reused', 'active', ?1, ?1, 'codex', 1, ?2, ?3)",
        rusqlite::params![
            now,
            std::process::id() as i64,
            r#"{"process_identity":"different-incarnation"}"#
        ],
    )
    .expect("insert reused-pid fixture");

    let (code, stdout, stderr) = h.run(["list"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let remains: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM instances WHERE name = 'reused'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remains, 0, "stdout={stdout} stderr={stderr}");
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
    assert!(first_context.contains(&format!("[hcom:{me}]")));

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
    assert!(repeated_context.contains(&format!("[hcom:{me}]")));

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
    let (send_code, _, send_stderr) = h.run([
        "send",
        &format!("@{me}"),
        "--name",
        &me,
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
            .is_some_and(|text| text.contains(&format!("[hcom:{me}]"))),
        "pi-start should return bootstrap with the hcom marker: {start}"
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
        None,
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

    // 3. pi-beforetool records tool status and allows the tool call.
    let before_tool = run_argv_hook(
        &h,
        "pi-beforetool",
        None,
        &[
            "--name",
            &me,
            "--tool",
            "bash",
            "--input-json",
            r#"{"command":"echo hello"}"#,
        ],
    );
    assert_eq!(before_tool, serde_json::json!({ "decision": "allow" }));
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

    let check = h.run(["pi-read", "--name", &me, "--check"]);
    assert_eq!(check.0, 0, "pi-read --check stderr={}", check.2);
    assert_eq!(check.1.trim(), "true");

    let read = h.run(["pi-read", "--name", &me]);
    assert_eq!(read.0, 0, "pi-read stderr={}", read.2);
    let messages: serde_json::Value = serde_json::from_str(&read.1).expect("pi-read json");
    assert!(
        messages
            .as_array()
            .is_some_and(|items| items.iter().any(|m| m["message"] == "ping")),
        "pi-read should return pending ping: {messages}"
    );

    let ack = run_argv_hook(&h, "pi-read", None, &["--name", &me, "--ack"]);
    assert_eq!(ack["acked"].as_u64(), Some(1));
    let check = h.run(["pi-read", "--name", &me, "--check"]);
    assert_eq!(check.0, 0, "pi-read --check after ack stderr={}", check.2);
    assert_eq!(check.1.trim(), "false");

    // 5. pi-stop finalizes the session.
    let stop = run_argv_hook(&h, "pi-stop", None, &["--name", &me, "--reason", "done"]);
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
    assert!(
        stdout.contains("agy and antigravity use --add-dir"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("DIPPY_POLICY_CWD"), "stdout={stdout}");
    assert!(stdout.contains("--as <name>"), "stdout={stdout}");
    assert!(stdout.contains("@<group>"), "stdout={stdout}");
    assert!(stdout.contains("\"groups\""), "stdout={stdout}");
    assert!(stdout.contains("\"roaming\": true"), "stdout={stdout}");
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
        "5. the matching tools.<effective-cli> profile",
        "6. command-line flags",
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

    let (code, _script, stderr) = h.run(["agy", "--terminal", "print"]);
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
fn agent_agy_policy_cwd_survives_tracked_resume_and_cli_switch() {
    let h = Hcom::new();
    h.set_launch_env("DIPPY_POLICY_CWD", "/ambient-policy");
    let workspace = h.root_path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        h.path().join("agents.json"),
        format!(
            r#"{{"agents":{{"knowledge":{{"dir":{},"cli":"codex","env":{{"DIPPY_POLICY_CWD":"/wrong"}}}}}}}}"#,
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

    let (code, _stdout, stderr) = h.run(["r", "knowledge", "--terminal", "print"]);
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

    // Bare group name without @ suggests @workers
    let (code, stdout, stderr) = h.run(["kill", "workers"]);
    assert_eq!(code, 1, "stdout={stdout} stderr={stderr}");
    assert!(
        stderr.contains("Agent 'workers' not found\nDid you mean @workers? Groups require @"),
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
    assert!(stdout.contains("reviewer_weather_app"), "stdout={stdout}");

    let (code, events, stderr) = h.run(["events", "--type", "message", "--last", "1", "--full"]);
    assert_eq!(code, 0, "events={events} stderr={stderr}");
    let event: serde_json::Value = serde_json::from_str(events.trim()).expect("message event JSON");
    assert_eq!(event["data"]["text"], "review this");
    assert_eq!(
        event["data"]["mentions"],
        serde_json::json!(["reviewer_weather_app"])
    );
    assert_eq!(
        event["data"]["delivered_to"],
        serde_json::json!(["reviewer_weather_app"])
    );

    let (code, instances, stderr) = h.run(["list", "--json"]);
    assert_eq!(code, 0, "instances={instances} stderr={stderr}");
    let instances: serde_json::Value =
        serde_json::from_str(&instances).expect("instance list JSON");
    let reviewer = instances
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["name"] == "reviewer_weather_app")
        })
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
    assert!(stdout.contains("name:      reviewer_forecast_service"));
    assert!(stdout.contains("roaming:   true"));
    assert!(
        stdout.contains(&format!("dir:       {}", project.display())),
        "stdout={stdout}"
    );
    assert!(stdout.contains("codex --as reviewer_forecast_service"));
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
    conn.execute(
        "INSERT INTO instances (name, status, directory, created_at) \
         VALUES ('reviewer_agent_project', 'listening', ?1, 1000.0)",
        [project.to_string_lossy().as_ref()],
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
    assert!(stdout.contains("reviewer_agent_project"), "stdout={stdout}");
    assert!(
        !stdout.contains("reviewer_unrelated_cwd"),
        "stdout={stdout}"
    );
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
fn roaming_send_rejects_same_slug_for_a_different_project_root() {
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
         VALUES ('reviewer_app', 'stopped', ?1, 1000.0)",
        [first.to_string_lossy().as_ref()],
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
        stderr.contains("project directory basenames must be unique"),
        "stderr={stderr}"
    );
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
printf '%s\n' '{"id":"request-launch","ok":true,"result":{"terminal":{"handle":"term:runtime:launch","worktreeId":"workspace:1","title":"orca-direct"}}}'
"#,
    )
    .expect("write fake Orca launch CLI");
    let fake_codex = fake_bin.join("codex");
    std::fs::write(
        &fake_codex,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex-cli 0.131.0'; exit 0; fi\nexit 0\n",
    )
    .expect("write fake Codex CLI");
    for executable in [&fake_orca, &fake_codex] {
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
        "codex",
        "--terminal",
        "orca",
        "--dir",
        launch_dir.to_str().expect("UTF-8 launch dir"),
        "--as",
        "orca-direct",
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
            .any(|pair| pair == ["--title", "orca-direct"])
    );
    assert!(
        argv.windows(2)
            .any(|pair| pair == ["--interactive-agent", "codex"])
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
    let fake_codex = fake_bin.join("codex");
    std::fs::write(
        &fake_codex,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex-cli 0.131.0'; fi\nexit 0\n",
    )
    .expect("write fake Codex CLI");
    for executable in [&fake_orca, &fake_codex] {
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
    .args(["codex", "--terminal", "orca", "--as", "orca-cleanup"]);
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
fn agent_session_uses_tmux_window_with_terminal_here() {
    let h = Hcom::new();
    std::fs::write(
        h.path().join("agents.json"),
        r#"{"agents":{"solo":{"dir":"/tmp","cli":"codex","terminal":"tmux","session":"work","window":"main",
                              "pre":"echo ready"}}}"#,
    )
    .expect("write catalog");

    let (code, stdout, stderr) = h.run(["agent", "solo", "--dry-run"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    if stderr.contains("tmux not found") {
        // No tmux on this machine: the session must degrade to a plain launch.
        assert!(!stdout.contains("tmux "), "stdout={stdout}");
        return;
    }
    assert!(stdout.contains("-s work"), "stdout={stdout}");
    assert!(stdout.contains("-n main"), "stdout={stdout}");
    assert!(stdout.contains("echo ready &&"), "stdout={stdout}");
    assert!(stdout.contains("--terminal here"), "stdout={stdout}");
    assert!(stdout.contains(r#"exec "${SHELL:-"#), "stdout={stdout}");
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
    // Composite oracle: first establish send_ok and pending mid-wait.
    // In GREEN, pending=true implies cursor unchanged, preview_count <= 1 (no duplicate preview),
    // and expected final code. Endpoint registration is diagnostic-only and not required.
    // In RED, premature_exit is Some(ExitStatus(0)), failing immediately on pending=false.
    let oracle_passed = send_ok
        && pending
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

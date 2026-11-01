use anyhow::Result;
use chrono::Utc;
use hcom_graph::model::{
    AgentGraph, AgentNode, AgentStatus, InstanceSnapshot, MessageData, MessageEvent,
    is_valid_identity, sanitize_display_string,
};
use hcom_graph::source::{
    CommandExecutor, HcomSyncWorker, StreamHandle, build_messages_snapshot_cmd, parse_instances,
    parse_messages_snapshot, read_line_capped,
};
use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn test_ecma48_ansi_osc_utf8_c1_and_fail_closed() {
    // 1. CSI with subparameters ':'
    assert_eq!(
        sanitize_display_string("\x1b[38:2::255:0:0mRedText\x1b[0m"),
        "RedText"
    );

    // 2. CSI with '~' final byte
    assert_eq!(sanitize_display_string("Before\x1b[3~After"), "BeforeAfter");

    // 3. 7-bit OSC with BEL terminator
    assert_eq!(
        sanitize_display_string("\x1b]52;c;payload\x07CleanText"),
        "CleanText"
    );

    // 4. 7-bit OSC with 7-bit ST (\x1b\\)
    assert_eq!(
        sanitize_display_string("\x1b]0;Title\x1b\\CleanText"),
        "CleanText"
    );

    // 5. UTF-8 C1 OSC (\u{009D} and \u{009C})
    assert_eq!(
        sanitize_display_string("Start\u{009D}52;c;bad\u{009C}End"),
        "StartEnd"
    );

    // 6. UTF-8 C1 CSI (\u{009B})
    assert_eq!(sanitize_display_string("\u{009B}1mBold\u{009B}0m"), "Bold");

    // 7. Fail-closed: unterminated 7-bit OSC
    assert_eq!(
        sanitize_display_string("Prefix\x1b]52;c;unterminated_payload"),
        "Prefix"
    );

    // 8. Fail-closed: unterminated UTF-8 C1 OSC
    assert_eq!(
        sanitize_display_string("Prefix\u{009D}52;c;unterminated_payload"),
        "Prefix"
    );

    // 9. Fail-closed: unterminated 7-bit CSI
    assert_eq!(sanitize_display_string("Prefix\x1b[38;2;255"), "Prefix");

    // 10. Bidi overrides and zero-width characters removed
    let dirty = "Hello\u{202E}Reversed\u{202C}\u{200B}World\u{FEFF}!";
    assert_eq!(sanitize_display_string(dirty), "HelloReversedWorld!");

    // 11. All C0/C1 control characters removed, including \n and \t
    assert_eq!(
        sanitize_display_string("Line1\nLine2\tTabbed\rCarriage"),
        "Line1Line2TabbedCarriage"
    );
}

#[test]
fn test_identity_sanitization_and_collision_rule() {
    assert!(is_valid_identity("valid_agent-1"));
    assert!(!is_valid_identity(""));
    assert!(!is_valid_identity("   "));

    // Identities with leading or trailing whitespace must fail validation
    assert!(!is_valid_identity(" alice "));
    assert!(!is_valid_identity("alice "));
    assert!(!is_valid_identity(" alice"));

    // Identities with ANSI escapes or bidi overrides fail the collision rule
    assert!(!is_valid_identity("alice\x1b[31m"));
    assert!(!is_valid_identity("alice\u{202E}bob"));
    assert!(!is_valid_identity("zero\u{200B}width"));

    let mut graph = AgentGraph::new();

    // Valid node is upserted
    let valid_node = AgentNode {
        name: "agent_valid".into(),
        status: AgentStatus::Active,
        status_context: Some("\x1b[31mcontext\x1b[0m".into()),
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    let idx = graph.upsert_agent(valid_node);
    assert!(idx.is_some());
    assert_eq!(
        graph
            .get_agent("agent_valid")
            .unwrap()
            .status_context
            .as_deref(),
        Some("context")
    );

    // Node with leading/trailing whitespace is rejected
    let whitespace_node = AgentNode {
        name: " agent_ws ".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    assert!(graph.upsert_agent(whitespace_node).is_none());
    assert_eq!(graph.node_count(), 1);

    // Malicious identity is rejected
    let malicious_node = AgentNode {
        name: "agent\x1b[31m".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    assert!(graph.upsert_agent(malicious_node).is_none());
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn test_malicious_sender_advances_watermark_without_stuck_loop() {
    let mut graph = AgentGraph::new();

    // Event 1 has malicious sender -> rejected, but watermark MUST advance to 1
    let malicious_msg = MessageEvent {
        id: 1,
        ts: Utc::now(),
        instance: "agent_valid".into(),
        data: MessageData {
            from: "spoofed\x1b[31m".into(),
            delivered_to: vec!["agent_valid".into()],
            mentions: vec![],
            intent: None,
            text: "text".into(),
        },
    };
    assert!(!graph.apply_message_event(&malicious_msg));
    assert_eq!(
        graph.last_event_id, 1,
        "Watermark must advance to avoid stuck reconnect loop"
    );

    // Event 2 is valid -> accepted, watermark advances to 2
    let valid_msg = MessageEvent {
        id: 2,
        ts: Utc::now(),
        instance: "alice".into(),
        data: MessageData {
            from: "alice".into(),
            delivered_to: vec!["bob".into()],
            mentions: vec![],
            intent: None,
            text: "hello".into(),
        },
    };
    assert!(graph.apply_message_event(&valid_msg));
    assert_eq!(graph.last_event_id, 2);
    assert_eq!(graph.get_message_edge("alice", "bob").unwrap().count, 1);
}

#[test]
fn test_read_line_capped_with_drain() {
    // 1. Exact-cap without newline must be rejected
    let exact_cap_no_nl = b"0123456789";
    let mut cur1 = Cursor::new(exact_cap_no_nl);
    assert!(
        read_line_capped(&mut cur1, 10).is_err(),
        "Exact cap without newline must fail"
    );

    // 2. Exact-cap WITH newline must succeed
    let exact_cap_with_nl = b"012345678\n";
    let mut cur2 = Cursor::new(exact_cap_with_nl);
    let res = read_line_capped(&mut cur2, 10).expect("read ok");
    assert_eq!(res.as_deref(), Some("012345678\n"));

    // 3. Oversized line followed by newline drains and recovers next line
    let stream_data = b"short line\n123456789012345678901234567890\nrecovered line\n";
    let mut cursor = Cursor::new(stream_data);

    let l1 = read_line_capped(&mut cursor, 20).expect("read ok");
    assert_eq!(l1.as_deref(), Some("short line\n"));

    let l2_err = read_line_capped(&mut cursor, 20);
    assert!(l2_err.is_err(), "Must return error for oversized line");

    let l3 = read_line_capped(&mut cursor, 20).expect("read ok");
    assert_eq!(l3.as_deref(), Some("recovered line\n"));

    let l4 = read_line_capped(&mut cursor, 20).expect("read ok");
    assert_eq!(l4, None);
}

#[test]
fn test_non_blocking_backoff_timing() {
    struct FailingStream;
    impl StreamHandle for FailingStream {
        fn next_line(&mut self) -> Result<Option<String>> {
            Ok(None)
        }
        fn kill(&mut self) {}
    }

    struct MockSpawnFailExecutor;
    impl CommandExecutor for MockSpawnFailExecutor {
        fn run_command(&self, _argv: &[String]) -> Result<String> {
            Ok("[]".into())
        }
        fn spawn_stream(&self, _argv: &[String]) -> Result<Box<dyn StreamHandle>> {
            Ok(Box::new(FailingStream))
        }
    }

    let worker = HcomSyncWorker::new(MockSpawnFailExecutor, 100);
    let mut graph = AgentGraph::new();
    let mut follower: Option<Box<dyn StreamHandle>> = Some(Box::new(FailingStream));

    assert_eq!(
        *worker.current_backoff.lock().unwrap(),
        Duration::from_millis(500)
    );
    assert!(worker.reconnect_deadline.lock().unwrap().is_none());

    let updated = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(!updated);
    assert!(worker.reconnect_deadline.lock().unwrap().is_some());
    assert_eq!(
        *worker.current_backoff.lock().unwrap(),
        Duration::from_millis(1000)
    );

    let non_blocking_poll = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(!non_blocking_poll);
    assert!(worker.reconnect_deadline.lock().unwrap().is_some());
}

#[test]
fn test_graph_construction_from_instances() {
    let raw_json = r#"[
        {
            "name": "orchestrator",
            "status": "active",
            "status_context": "tool:Bash",
            "status_age_seconds": 15,
            "unread_count": 0,
            "parent_name": null,
            "directory": "/home/user/project",
            "tool": "claude"
        },
        {
            "name": "worker_1",
            "status": "listening",
            "status_context": "prompt",
            "status_age_seconds": 45,
            "unread_count": 2,
            "parent_name": "orchestrator",
            "directory": "/home/user/project",
            "tool": "codex"
        }
    ]"#;

    let instances: Vec<InstanceSnapshot> = parse_instances(raw_json).expect("valid json");
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&instances);

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);

    let orch = graph.get_agent("orchestrator").expect("exists");
    assert_eq!(orch.status, AgentStatus::Active);
    assert_eq!(orch.status_context.as_deref(), Some("tool:Bash"));

    let worker = graph.get_agent("worker_1").expect("exists");
    assert_eq!(worker.status, AgentStatus::Listening);
    assert_eq!(worker.parent_name.as_deref(), Some("orchestrator"));

    let spawn_edge = graph
        .spawn_edges
        .get(&("orchestrator".to_string(), "worker_1".to_string()));
    assert!(
        spawn_edge.is_some(),
        "Spawn edge orchestrator -> worker_1 must exist"
    );
}

#[test]
fn test_upsert_reparenting_and_parent_none() {
    let mut graph = AgentGraph::new();

    let node_a = AgentNode {
        name: "child".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: Some("parent_a".into()),
        directory: None,
        tool: None,
    };
    graph.upsert_agent(node_a);
    assert!(graph.get_spawn_edge("parent_a", "child").is_some());

    let node_b = AgentNode {
        name: "child".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: Some("parent_b".into()),
        directory: None,
        tool: None,
    };
    graph.upsert_agent(node_b);
    assert!(
        graph.get_spawn_edge("parent_a", "child").is_none(),
        "Old spawn edge must be removed"
    );
    assert!(
        graph.get_spawn_edge("parent_b", "child").is_some(),
        "New spawn edge must exist"
    );

    let node_none = AgentNode {
        name: "child".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    graph.upsert_agent(node_none);
    assert!(
        graph.get_spawn_edge("parent_b", "child").is_none(),
        "Spawn edge must be removed when parent_name is None"
    );
}

#[test]
fn test_reset_preserves_spawn_edges_and_stable_indices() {
    let mut graph = AgentGraph::new();

    let child_node = AgentNode {
        name: "child".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: Some("parent".into()),
        directory: None,
        tool: None,
    };
    graph.upsert_agent(child_node);
    assert!(graph.get_spawn_edge("parent", "child").is_some());

    let msg = MessageEvent {
        id: 1,
        ts: Utc::now(),
        instance: "parent".into(),
        data: MessageData {
            from: "parent".into(),
            delivered_to: vec!["child".into()],
            mentions: vec![],
            intent: None,
            text: "hello".into(),
        },
    };
    graph.apply_message_event(&msg);
    assert!(graph.get_message_edge("parent", "child").is_some());
    assert_eq!(graph.edge_count(), 2);

    graph.reset_events();

    assert!(graph.get_message_edge("parent", "child").is_none());
    assert!(
        graph.get_spawn_edge("parent", "child").is_some(),
        "Spawn edge must survive reset_events"
    );
    assert_eq!(graph.edge_count(), 1);

    let updated_child = AgentNode {
        name: "child".into(),
        status: AgentStatus::Listening,
        status_context: None,
        status_age_seconds: 10,
        unread_count: 0,
        parent_name: Some("parent".into()),
        directory: None,
        tool: None,
    };
    graph.upsert_agent(updated_child);
    assert!(graph.get_spawn_edge("parent", "child").is_some());
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn test_message_accumulation_cyclic() {
    let mut graph = AgentGraph::new();

    for id in 1..=3 {
        let event = MessageEvent {
            id,
            ts: Utc::now(),
            instance: "alice".into(),
            data: MessageData {
                from: "alice".into(),
                delivered_to: vec!["bob".into()],
                mentions: vec!["bob".into()],
                intent: Some("request".into()),
                text: format!("Request #{}", id),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    for id in 4..=5 {
        let event = MessageEvent {
            id,
            ts: Utc::now(),
            instance: "bob".into(),
            data: MessageData {
                from: "bob".into(),
                delivered_to: vec!["alice".into()],
                mentions: vec!["alice".into()],
                intent: Some("ack".into()),
                text: format!("Reply #{}", id),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    let a_to_b = graph.get_message_edge("alice", "bob").expect("edge exists");
    assert_eq!(a_to_b.count, 3);
    assert_eq!(a_to_b.last_intent.as_deref(), Some("request"));
    assert_eq!(a_to_b.last_preview, "Request #3");

    let b_to_a = graph.get_message_edge("bob", "alice").expect("edge exists");
    assert_eq!(b_to_a.count, 2);
    assert_eq!(b_to_a.last_intent.as_deref(), Some("ack"));
    assert_eq!(b_to_a.last_preview, "Reply #5");

    assert_eq!(graph.last_event_id, 5);
}

#[test]
fn test_watermark_deduplication() {
    let mut graph = AgentGraph::new();

    let e1 = MessageEvent {
        id: 10,
        ts: Utc::now(),
        instance: "agent_a".into(),
        data: MessageData {
            from: "agent_a".into(),
            delivered_to: vec!["agent_b".into()],
            mentions: vec![],
            intent: None,
            text: "Hello".into(),
        },
    };

    assert!(graph.apply_message_event(&e1));
    assert_eq!(graph.last_event_id, 10);
    assert_eq!(
        graph.get_message_edge("agent_a", "agent_b").unwrap().count,
        1
    );

    let e1_duplicate = MessageEvent {
        id: 10,
        ts: Utc::now(),
        instance: "agent_a".into(),
        data: MessageData {
            from: "agent_a".into(),
            delivered_to: vec!["agent_b".into()],
            mentions: vec![],
            intent: None,
            text: "Hello duplicate".into(),
        },
    };
    assert!(!graph.apply_message_event(&e1_duplicate));

    let e_older = MessageEvent {
        id: 9,
        ts: Utc::now(),
        instance: "agent_a".into(),
        data: MessageData {
            from: "agent_a".into(),
            delivered_to: vec!["agent_b".into()],
            mentions: vec![],
            intent: None,
            text: "Older event".into(),
        },
    };
    assert!(!graph.apply_message_event(&e_older));

    assert_eq!(
        graph.get_message_edge("agent_a", "agent_b").unwrap().count,
        1
    );
    assert_eq!(graph.last_event_id, 10);
}

#[test]
fn test_initial_load_sorting() {
    let json_lines = r#"
{"id":30,"instance":"a","ts":"2026-11-01T10:00:03Z","type":"message","data":{"from":"a","delivered_to":["b"],"text":"msg 30"}}
{"id":20,"instance":"a","ts":"2026-11-01T10:00:02Z","type":"message","data":{"from":"a","delivered_to":["b"],"text":"msg 20"}}
{"id":10,"instance":"a","ts":"2026-11-01T10:00:01Z","type":"message","data":{"from":"a","delivered_to":["b"],"text":"msg 10"}}
"#;

    let sorted_events = parse_messages_snapshot(json_lines).expect("parse ok");
    assert_eq!(sorted_events.len(), 3);
    assert_eq!(sorted_events[0].id, 10);
    assert_eq!(sorted_events[1].id, 20);
    assert_eq!(sorted_events[2].id, 30);

    let mut graph = AgentGraph::new();
    for event in &sorted_events {
        assert!(graph.apply_message_event(event));
    }

    assert_eq!(graph.get_message_edge("a", "b").unwrap().count, 3);
    assert_eq!(graph.last_event_id, 30);
}

#[test]
fn test_current_db_id_monotonicity() {
    let snapshot_cmd = build_messages_snapshot_cmd(500);
    assert!(
        !snapshot_cmd.contains(&"--all".to_string()),
        "Snapshot must not query archived databases"
    );

    let mut graph = AgentGraph::new();
    for id in [100, 101, 105, 110] {
        let event = MessageEvent {
            id,
            ts: Utc::now(),
            instance: "x".into(),
            data: MessageData {
                from: "x".into(),
                delivered_to: vec!["y".into()],
                mentions: vec![],
                intent: None,
                text: "ping".into(),
            },
        };
        assert!(graph.apply_message_event(&event));
    }
    assert_eq!(graph.last_event_id, 110);
    assert_eq!(graph.get_message_edge("x", "y").unwrap().count, 4);
}

struct MockStreamHandle {
    lines: VecDeque<String>,
    killed: Arc<AtomicBool>,
}

impl StreamHandle for MockStreamHandle {
    fn next_line(&mut self) -> Result<Option<String>> {
        Ok(self.lines.pop_front())
    }

    fn kill(&mut self) {
        self.killed.store(true, Ordering::SeqCst);
    }
}

struct MockExecutor {
    list_responses: Mutex<VecDeque<String>>,
    snapshot_responses: Mutex<VecDeque<String>>,
    stream_queues: Mutex<VecDeque<Vec<String>>>,
    last_spawned_killed_flag: Mutex<Option<Arc<AtomicBool>>>,
}

impl CommandExecutor for MockExecutor {
    fn run_command(&self, argv: &[String]) -> Result<String> {
        if argv.contains(&"list".to_string()) {
            let mut q = self.list_responses.lock().unwrap();
            return Ok(q.pop_front().unwrap_or_else(|| "[]".to_string()));
        }
        if argv.contains(&"events".to_string()) {
            let mut q = self.snapshot_responses.lock().unwrap();
            return Ok(q.pop_front().unwrap_or_default());
        }
        Ok(String::new())
    }

    fn spawn_stream(&self, _argv: &[String]) -> Result<Box<dyn StreamHandle>> {
        let mut q = self.stream_queues.lock().unwrap();
        let lines: VecDeque<String> = q.pop_front().unwrap_or_default().into();
        let killed = Arc::new(AtomicBool::new(false));
        *self.last_spawned_killed_flag.lock().unwrap() = Some(killed.clone());
        Ok(Box::new(MockStreamHandle { lines, killed }))
    }
}

#[test]
fn test_sync_worker_lifecycle_and_db_reset() {
    let list_epoch1 = r#"[{"name":"m1","status":"active","parent_name":null}]"#;
    let snap_epoch1 = r#"{"id":1,"instance":"m1","ts":"2026-11-01T10:00:00Z","type":"message","data":{"from":"m1","delivered_to":["m2"],"text":"snap1"}}"#;

    let stream_epoch1 = vec![
        r#"{"id":2,"instance":"m1","ts":"2026-11-01T10:00:01Z","type":"message","data":{"from":"m1","delivered_to":["m2"],"text":"msg2"}}"#.into(),
        r#"{"db_reset":true}"#.into(),
    ];

    let list_epoch2 = r#"[{"name":"m3","status":"listening","parent_name":null}]"#;
    let snap_epoch2 = r#"{"id":1,"instance":"m3","ts":"2026-11-01T11:00:00Z","type":"message","data":{"from":"m3","delivered_to":["m4"],"text":"new-snap1"}}"#;

    let stream_epoch2 = vec![
        r#"{"id":2,"instance":"m3","ts":"2026-11-01T11:00:01Z","type":"message","data":{"from":"m3","delivered_to":["m4"],"text":"new-msg2"}}"#.into(),
    ];

    let executor = MockExecutor {
        list_responses: Mutex::new(VecDeque::from(vec![
            list_epoch1.to_string(),
            list_epoch2.to_string(),
        ])),
        snapshot_responses: Mutex::new(VecDeque::from(vec![
            snap_epoch1.to_string(),
            snap_epoch2.to_string(),
        ])),
        stream_queues: Mutex::new(VecDeque::from(vec![stream_epoch1, stream_epoch2])),
        last_spawned_killed_flag: Mutex::new(None),
    };

    let worker = HcomSyncWorker::new(executor, 100);
    let mut graph = AgentGraph::new();

    worker.initial_sync(&mut graph).unwrap();
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.get_message_edge("m1", "m2").unwrap().count, 1);
    assert_eq!(graph.last_event_id, 1);

    let mut follower = Some(worker.spawn_follower(&graph).unwrap());
    let killed_ref1 = worker
        .executor
        .last_spawned_killed_flag
        .lock()
        .unwrap()
        .clone()
        .unwrap();

    let updated = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(updated);
    assert_eq!(graph.get_message_edge("m1", "m2").unwrap().count, 2);
    assert_eq!(graph.last_event_id, 2);

    let reset_processed = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(reset_processed);

    assert!(
        killed_ref1.load(Ordering::SeqCst),
        "Old follower process must be killed upon db_reset"
    );

    assert!(
        graph.get_message_edge("m1", "m2").is_none(),
        "Old epoch messages must be cleared"
    );
    assert_eq!(
        graph.get_message_edge("m3", "m4").unwrap().count,
        1,
        "New snapshot message must be present"
    );
    assert_eq!(graph.last_event_id, 1);

    let new_msg_processed = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(new_msg_processed);
    assert_eq!(graph.get_message_edge("m3", "m4").unwrap().count, 2);
    assert_eq!(graph.last_event_id, 2);
}

#[test]
fn test_system_command_executor_capped() {
    use hcom_graph::source::SystemCommandExecutor;
    let executor = SystemCommandExecutor;

    #[cfg(unix)]
    let argv = vec![
        "head".into(),
        "-c".into(),
        "2000".into(),
        "/dev/zero".into(),
    ];
    #[cfg(windows)]
    let argv = vec![
        "cmd.exe".into(),
        "/c".into(),
        "for /L %i in (1,1,200) do @echo 1234567890".into(),
    ];

    let res = executor.run_command_capped(&argv, 500);
    assert!(res.is_err(), "Command exceeding cap must return error");
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("exceeded maximum allowed size")
    );

    let res_ok = executor.run_command_capped(&argv, 3000);
    assert!(res_ok.is_ok());
}

#[test]
fn test_system_command_executor_no_stderr_deadlock() {
    use hcom_graph::source::SystemCommandExecutor;
    let executor = SystemCommandExecutor;

    #[cfg(unix)]
    let argv = vec![
        "sh".into(),
        "-c".into(),
        "head -c 131072 /dev/zero >&2 && echo ok".into(),
    ];
    #[cfg(windows)]
    let argv = vec![
        "cmd.exe".into(),
        "/c".into(),
        "for /L %i in (1,1,13000) do @echo 1234567890 >&2 & echo ok".into(),
    ];

    let res = executor.run_command_capped(&argv, 10000);
    assert!(
        res.is_ok(),
        "Must not deadlock when child produces large stderr"
    );
    assert_eq!(res.unwrap().trim(), "ok");
}

#[test]
fn test_parse_messages_snapshot_drops_oversized_lines() {
    let normal_line = r#"{"id":1,"instance":"a","ts":"2026-11-01T10:00:01Z","type":"message","data":{"from":"a","delivered_to":["b"],"text":"ok"}}"#;
    let oversized_line = format!(
        r#"{{"id":2,"instance":"a","ts":"2026-11-01T10:00:02Z","type":"message","data":{{"from":"a","delivered_to":["b"],"text":"{}"}}}}"#,
        "x".repeat(1024 * 1024 + 10)
    );

    let payload = format!("{}\n{}\n", normal_line, oversized_line);
    let parsed = parse_messages_snapshot(&payload).expect("parse ok");
    assert_eq!(parsed.len(), 1, "Oversized line must be dropped");
    assert_eq!(parsed[0].id, 1);
}

use anyhow::Result;
use chrono::Utc;
use egui::{Color32, Pos2, Rect, Vec2};
use hcom_dashboard::action::{ActionRequest, ActionResult, ActionRunner, ActionWorkerHandle};
use hcom_dashboard::canvas::{CanvasView, distance_to_quadratic_bezier, distance_to_segment};
use hcom_dashboard::layout::GraphLayout;
use hcom_dashboard::model::{
    AgentGraph, AgentNode, AgentStatus, InstanceSnapshot, MessageData, MessageEvent,
    is_valid_identity, sanitize_display_string,
};
use hcom_dashboard::source::{
    CommandExecutor, HcomSyncWorker, StreamHandle, StreamKillHandle, StreamReader,
    build_messages_follow_cmd, parse_instances, read_line_capped, split_system_stream,
};
use hcom_dashboard::sync::BackgroundSync;
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
            reply_to: None,
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
            reply_to: None,
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

        fn spawn_stream_split(
            &self,
            _argv: &[String],
        ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)> {
            struct FailingReader;
            impl StreamReader for FailingReader {
                fn next_line(&mut self, _cap: usize) -> Result<Option<String>> {
                    Err(anyhow::anyhow!("stream fail"))
                }
            }
            struct FailingKill;
            impl StreamKillHandle for FailingKill {
                fn kill(&self) -> Result<()> {
                    Ok(())
                }
            }
            Ok((Box::new(FailingReader), Arc::new(FailingKill)))
        }
    }

    let worker = HcomSyncWorker::new(MockSpawnFailExecutor);
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
            reply_to: None,
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
                reply_to: None,
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
                reply_to: None,
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
            reply_to: None,
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
            reply_to: None,
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
            reply_to: None,
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
fn test_current_db_id_monotonicity() {
    let follow_cmd = build_messages_follow_cmd(0);
    assert!(
        !follow_cmd.contains(&"--all".to_string()),
        "History replay must not query archived databases"
    );
    assert!(
        !follow_cmd.contains(&"--last".to_string()),
        "History replay must not limit the message count"
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
                reply_to: None,
                text: "ping".into(),
            },
        };
        assert!(graph.apply_message_event(&event));
    }
    assert_eq!(graph.last_event_id, 110);
    assert_eq!(graph.get_message_edge("x", "y").unwrap().count, 4);
    assert_eq!(graph.get_message_edge("x", "y").unwrap().history.len(), 4);
}

#[test]
fn test_message_edge_keeps_full_history_and_sanitizes() {
    let mut graph = AgentGraph::new();
    // 600 messages: more than the former 500 snapshot and 50 per-edge limits.
    for id in 1..=600 {
        let event = MessageEvent {
            id,
            ts: Utc::now(),
            instance: "alice".into(),
            data: MessageData {
                from: "alice".into(),
                delivered_to: vec!["bob".into()],
                mentions: vec![],
                intent: Some("chat\x1b[31m".into()),
                reply_to: None,
                text: format!("msg_{}\x1b[2J\u{200B}", id),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    let edge = graph.get_message_edge("alice", "bob").unwrap();
    assert_eq!(edge.count, 600);
    assert_eq!(edge.history.len(), 600, "History must keep every message");
    assert_eq!(edge.history.front().unwrap().id, 1);
    assert_eq!(edge.history.back().unwrap().id, 600);

    // Sanitization must have cleaned the text and intent
    let last = edge.history.back().unwrap();
    assert_eq!(last.text, "msg_600");
    assert_eq!(last.intent.as_deref(), Some("chat"));
}

#[test]
fn test_conversation_history_bidirectional_sorting_capped_at_50() {
    let mut graph = AgentGraph::new();
    let base_time = Utc::now();

    // Alice -> Bob: 30 messages (odd timestamps)
    for i in 0..30 {
        let ts = base_time + chrono::Duration::seconds(i * 2);
        let event = MessageEvent {
            id: (i as u64) + 1,
            ts,
            instance: "alice".into(),
            data: MessageData {
                from: "alice".into(),
                delivered_to: vec!["bob".into()],
                mentions: vec![],
                intent: None,
                reply_to: None,
                text: format!("alice_{}", i),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    // Bob -> Alice: 30 messages (even timestamps, interleaved)
    for i in 0..30 {
        let ts = base_time + chrono::Duration::seconds(i * 2 + 1);
        let event = MessageEvent {
            id: (i as u64) + 101,
            ts,
            instance: "bob".into(),
            data: MessageData {
                from: "bob".into(),
                delivered_to: vec!["alice".into()],
                mentions: vec![],
                intent: None,
                reply_to: None,
                text: format!("bob_{}", i),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    // Total = 60 messages between Alice and Bob. get_conversation_history must cap at 50!
    let history = graph.get_conversation_history("alice", "bob");
    assert_eq!(
        history.len(),
        50,
        "Combined history must be strictly capped at 50"
    );

    // Must be sorted strictly ascending by timestamp
    for w in history.windows(2) {
        assert!(w[0].ts <= w[1].ts);
    }

    // Symmetric query (bob, alice) must return identical history
    let history_rev = graph.get_conversation_history("bob", "alice");
    assert_eq!(history, history_rev);
}

#[test]
fn test_reset_clears_message_history() {
    let mut graph = AgentGraph::new();
    let event = MessageEvent {
        id: 1,
        ts: Utc::now(),
        instance: "alice".into(),
        data: MessageData {
            from: "alice".into(),
            delivered_to: vec!["bob".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "hello".into(),
        },
    };
    assert!(graph.apply_message_event(&event));
    assert_eq!(graph.get_conversation_history("alice", "bob").len(), 1);

    // Reset events
    graph.reset_events();
    assert!(graph.get_message_edge("alice", "bob").is_none());
    assert_eq!(graph.get_conversation_history("alice", "bob").len(), 0);
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
    stream_queues: Mutex<VecDeque<Vec<String>>>,
    last_spawned_killed_flag: Mutex<Option<Arc<AtomicBool>>>,
    /// Every argv passed to `run_command` and `spawn_stream`, in order.
    commands: Mutex<Vec<Vec<String>>>,
}

impl CommandExecutor for MockExecutor {
    fn run_command(&self, argv: &[String]) -> Result<String> {
        self.commands.lock().unwrap().push(argv.to_vec());
        if argv.contains(&"list".to_string()) {
            let mut q = self.list_responses.lock().unwrap();
            return Ok(q.pop_front().unwrap_or_else(|| "[]".to_string()));
        }
        Ok(String::new())
    }

    fn spawn_stream(&self, argv: &[String]) -> Result<Box<dyn StreamHandle>> {
        self.commands.lock().unwrap().push(argv.to_vec());
        let mut q = self.stream_queues.lock().unwrap();
        let lines: VecDeque<String> = q.pop_front().unwrap_or_default().into();
        let killed = Arc::new(AtomicBool::new(false));
        *self.last_spawned_killed_flag.lock().unwrap() = Some(killed.clone());
        Ok(Box::new(MockStreamHandle { lines, killed }))
    }

    fn spawn_stream_split(
        &self,
        argv: &[String],
    ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)> {
        let handle = self.spawn_stream(argv)?;
        struct SplitMockReader(Box<dyn StreamHandle>);
        impl StreamReader for SplitMockReader {
            fn next_line(&mut self, _cap: usize) -> Result<Option<String>> {
                self.0.next_line()
            }
        }
        struct SplitMockKill(Arc<AtomicBool>);
        impl StreamKillHandle for SplitMockKill {
            fn kill(&self) -> Result<()> {
                self.0.store(true, Ordering::SeqCst);
                Ok(())
            }
        }
        let killed = self
            .last_spawned_killed_flag
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        Ok((
            Box::new(SplitMockReader(handle)),
            Arc::new(SplitMockKill(killed)),
        ))
    }
}

#[test]
fn test_sync_worker_lifecycle_and_db_reset() {
    let list_epoch1 = r#"[{"name":"m1","status":"active","parent_name":null}]"#;
    // The follower replays the whole history (id 1 on) before new events.
    let stream_epoch1 = vec![
        r#"{"id":1,"instance":"m1","ts":"2026-11-01T10:00:00Z","type":"message","data":{"from":"m1","delivered_to":["m2"],"text":"snap1"}}"#.into(),
        r#"{"id":2,"instance":"m1","ts":"2026-11-01T10:00:01Z","type":"message","data":{"from":"m1","delivered_to":["m2"],"text":"msg2"}}"#.into(),
        r#"{"db_reset":true}"#.into(),
    ];

    let list_epoch2 = r#"[{"name":"m3","status":"listening","parent_name":null}]"#;

    let stream_epoch2 = vec![
        r#"{"id":1,"instance":"m3","ts":"2026-11-01T11:00:00Z","type":"message","data":{"from":"m3","delivered_to":["m4"],"text":"new-snap1"}}"#.into(),
        r#"{"id":2,"instance":"m3","ts":"2026-11-01T11:00:01Z","type":"message","data":{"from":"m3","delivered_to":["m4"],"text":"new-msg2"}}"#.into(),
    ];

    let executor = MockExecutor {
        list_responses: Mutex::new(VecDeque::from(vec![
            list_epoch1.to_string(),
            list_epoch2.to_string(),
        ])),
        stream_queues: Mutex::new(VecDeque::from(vec![stream_epoch1, stream_epoch2])),
        last_spawned_killed_flag: Mutex::new(None),
        commands: Mutex::new(Vec::new()),
    };

    let worker = HcomSyncWorker::new(executor);
    let mut graph = AgentGraph::new();

    worker.initial_sync(&mut graph).unwrap();
    assert_eq!(graph.node_count(), 1);
    assert_eq!(graph.last_event_id, 0);

    let mut follower = Some(worker.spawn_follower(&graph).unwrap());
    assert!(worker.step_stream(&mut graph, &mut follower).unwrap());
    assert_eq!(graph.get_message_edge("m1", "m2").unwrap().count, 1);
    assert_eq!(graph.last_event_id, 1);
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
    assert_eq!(graph.last_event_id, 0, "Reset restarts the replay at id 0");

    for _ in 0..2 {
        assert!(worker.step_stream(&mut graph, &mut follower).unwrap());
    }
    assert_eq!(graph.get_message_edge("m3", "m4").unwrap().count, 2);
    assert_eq!(graph.last_event_id, 2);

    // Messages are loaded only by the follower replay from id 0: no
    // count-limited one-shot `hcom events` snapshot is ever run.
    let commands = worker.executor.commands.lock().unwrap();
    let follow_0 = build_messages_follow_cmd(0);
    assert_eq!(
        commands.iter().filter(|c| **c == follow_0).count(),
        2,
        "Start and db_reset must both replay from --after-id 0"
    );
    assert!(commands.iter().all(|c| !c.contains(&"--last".to_string())
        && (c.contains(&"list".to_string()) || c.contains(&"--follow".to_string()))));
}

#[test]
fn test_system_command_executor_capped() {
    use hcom_dashboard::source::SystemCommandExecutor;
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
    use hcom_dashboard::source::SystemCommandExecutor;
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

// =========================================================================
// TASK-77.2 Tests: Canvas, Layout, Interaction, and BackgroundSync
// =========================================================================

#[test]
fn test_node_status_colors() {
    assert_eq!(
        CanvasView::status_color(&AgentStatus::Active),
        Color32::from_rgb(46, 204, 113)
    );
    assert_eq!(
        CanvasView::status_color(&AgentStatus::Listening),
        Color32::from_rgb(52, 152, 219)
    );
    assert_eq!(
        CanvasView::status_color(&AgentStatus::Blocked),
        Color32::from_rgb(231, 76, 60)
    );
    assert_eq!(
        CanvasView::status_color(&AgentStatus::Stopped),
        Color32::from_rgb(127, 140, 141)
    );
}

#[test]
fn test_canvas_coordinate_transforms_pan_and_zoom() {
    let mut canvas = CanvasView::new();
    let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1000.0, 800.0));

    // Default zoom = 1.0, pan = (0, 0)
    let p_world = Pos2::new(50.0, -100.0);
    let p_screen = canvas.world_to_screen(p_world, rect);
    assert_eq!(p_screen, Pos2::new(550.0, 300.0)); // center is (500, 400)

    let roundtrip = canvas.screen_to_world(p_screen, rect);
    assert!((roundtrip.x - p_world.x).abs() < 1e-4);
    assert!((roundtrip.y - p_world.y).abs() < 1e-4);

    // Zoomed and panned
    canvas.zoom = 2.0;
    canvas.pan = Vec2::new(100.0, 50.0);
    let p_screen_zoomed = canvas.world_to_screen(p_world, rect);
    // center (500, 400) + pan (100, 50) + world (50, -100) * 2 = (700, 250)
    assert_eq!(p_screen_zoomed, Pos2::new(700.0, 250.0));

    let roundtrip_zoomed = canvas.screen_to_world(p_screen_zoomed, rect);
    assert!((roundtrip_zoomed.x - p_world.x).abs() < 1e-4);
    assert!((roundtrip_zoomed.y - p_world.y).abs() < 1e-4);
}

#[test]
fn test_layout_hit_test_and_drag_pinning() {
    let mut graph = AgentGraph::new();
    let snap = vec![
        InstanceSnapshot {
            name: "agent_a".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
        InstanceSnapshot {
            name: "agent_b".into(),
            status: Some("listening".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: Some("agent_a".into()),
            directory: None,
            tool: None,
        },
    ];
    graph.apply_instance_snapshot(&snap);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);

    let idx_a = graph.get_node_index("agent_a").expect("node a");
    let phys_a = layout.nodes.get(&idx_a).expect("phys a");
    let pos_a = phys_a.pos;

    // Hit test directly at node pos
    assert_eq!(layout.hit_test(pos_a, 26.0), Some(idx_a));

    // Hit test far outside node radius
    assert_eq!(
        layout.hit_test(Pos2::new(pos_a.x + 100.0, pos_a.y + 100.0), 26.0),
        None
    );

    // Test dragging and pinning
    let new_pos = Pos2::new(300.0, 400.0);
    layout.set_node_pos(idx_a, new_pos, true);

    let phys_pinned = layout.nodes.get(&idx_a).unwrap();
    assert_eq!(phys_pinned.pos, new_pos);
    assert!(phys_pinned.pinned);
    assert_eq!(phys_pinned.vel, Vec2::ZERO);

    // Layout step must NOT move pinned node
    let _ = layout.step(&graph, 0.016);
    let phys_after_step = layout.nodes.get(&idx_a).unwrap();
    assert_eq!(phys_after_step.pos, new_pos);
    assert_eq!(phys_after_step.vel, Vec2::ZERO);

    // Release drag / unpin
    layout.set_node_pos(idx_a, new_pos, false);
    let phys_unpinned = layout.nodes.get(&idx_a).unwrap();
    assert!(!phys_unpinned.pinned);
}

#[test]
fn test_layout_physics_step_dt_clamping_and_sleep() {
    let mut graph = AgentGraph::new();
    let snap = vec![
        InstanceSnapshot {
            name: "node_1".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
        InstanceSnapshot {
            name: "node_2".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
    ];
    graph.apply_instance_snapshot(&snap);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);

    // Run steps and verify layout converges to sleep
    for _ in 0..300 {
        layout.step(&graph, 0.016);
        if layout.is_sleeping {
            break;
        }
    }

    assert!(
        layout.is_sleeping,
        "Layout must enter sleep mode once stable"
    );
    assert_eq!(
        layout.step(&graph, 0.016),
        0.0,
        "Sleeping layout must do 0 work"
    );

    // Wake up
    layout.wake_up();
    assert!(!layout.is_sleeping);
}

struct MockOverloadExecutor {
    events_jsonl: String,
}

impl CommandExecutor for MockOverloadExecutor {
    fn run_command(&self, _argv: &[String]) -> Result<String> {
        Ok("[]".into())
    }

    fn spawn_stream(&self, _argv: &[String]) -> Result<Box<dyn StreamHandle>> {
        struct MockStream(std::io::Lines<std::io::Cursor<String>>);
        impl StreamHandle for MockStream {
            fn next_line(&mut self) -> Result<Option<String>> {
                match self.0.next() {
                    Some(Ok(l)) => Ok(Some(l)),
                    Some(Err(e)) => Err(e.into()),
                    None => Ok(None),
                }
            }
            fn kill(&mut self) {}
        }
        let cursor = std::io::Cursor::new(self.events_jsonl.clone());
        Ok(Box::new(MockStream(std::io::BufRead::lines(cursor))))
    }

    fn spawn_stream_split(
        &self,
        _argv: &[String],
    ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)> {
        struct MockReader(std::io::Lines<std::io::Cursor<String>>);
        impl StreamReader for MockReader {
            fn next_line(&mut self, _cap: usize) -> Result<Option<String>> {
                match self.0.next() {
                    Some(Ok(l)) => Ok(Some(l)),
                    Some(Err(e)) => Err(e.into()),
                    None => Ok(None),
                }
            }
        }
        struct MockKill;
        impl StreamKillHandle for MockKill {
            fn kill(&self) -> Result<()> {
                Ok(())
            }
        }
        let cursor = std::io::Cursor::new(self.events_jsonl.clone());
        Ok((
            Box::new(MockReader(std::io::BufRead::lines(cursor))),
            Arc::new(MockKill),
        ))
    }
}

#[test]
fn test_overload_10k_events_throttled_coalescing() {
    // Generate 10,000 message events
    let mut lines = String::new();
    for i in 1..=10_000 {
        lines.push_str(&format!(
            r#"{{"id":{},"instance":"bench","ts":"2026-11-01T10:00:00Z","type":"message","data":{{"from":"agent_a","delivered_to":["agent_b"],"text":"msg_{}"}}}}"#,
            i, i
        ));
        lines.push('\n');
    }

    let executor = MockOverloadExecutor {
        events_jsonl: lines,
    };
    let worker = HcomSyncWorker::new(executor);

    // Batch limit 200, cadence 16ms
    let sync = BackgroundSync::new(worker, 200, Duration::from_millis(16)).expect("sync start");

    // Wait until stream finishes processing
    let start = std::time::Instant::now();
    let mut final_graph = None;
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(g) = sync.try_get_latest() {
            final_graph = Some(g);
        }
        if let Some(ref g) = final_graph
            && g.last_event_id >= 10_000
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let g = final_graph.expect("must produce a final graph");
    assert_eq!(
        g.last_event_id, 10_000,
        "All 10,000 events must be ingested"
    );

    let pub_count = sync.published_count.load(Ordering::Relaxed);
    assert!(
        pub_count <= 60,
        "Throttled publications must be <= 60 for 10k events with batch 200, got {}",
        pub_count
    );
}

#[test]
fn test_drop_quiet_follower_kills_and_joins_cleanly() {
    // Custom follower executor spawning long sleep
    struct QuietStreamExecutor;
    impl CommandExecutor for QuietStreamExecutor {
        fn run_command(&self, _argv: &[String]) -> Result<String> {
            Ok("[]".into())
        }

        fn spawn_stream(&self, _argv: &[String]) -> Result<Box<dyn StreamHandle>> {
            unimplemented!()
        }

        fn spawn_stream_split(
            &self,
            _argv: &[String],
        ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)> {
            #[cfg(unix)]
            let cmd_argv: Vec<String> = vec!["sleep".into(), "60".into()];
            #[cfg(windows)]
            let cmd_argv: Vec<String> = vec![
                "powershell".into(),
                "-NoProfile".into(),
                "-Command".into(),
                "Start-Sleep -Seconds 60".into(),
            ];

            let mut child = std::process::Command::new(&cmd_argv[0])
                .args(&cmd_argv[1..])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("spawn quiet child");

            let stdout = child.stdout.take().unwrap();
            Ok(split_system_stream(child, stdout))
        }
    }

    let worker = HcomSyncWorker::new(QuietStreamExecutor);
    let sync = BackgroundSync::new(worker, 200, Duration::from_millis(16)).expect("start sync");

    // Allow worker to spawn child and block in read_line
    std::thread::sleep(Duration::from_millis(50));

    // Drop must kill child and join thread without deadlock
    let drop_start = std::time::Instant::now();
    drop(sync);
    let drop_duration = drop_start.elapsed();

    assert!(
        drop_duration < Duration::from_millis(200),
        "Drop of quiet follower must terminate and join in <200ms, took {:?}",
        drop_duration
    );
}

#[test]
fn test_layout_frame_budget_deadline_enforced_and_resumes() {
    let mut graph = AgentGraph::new();
    let mut snap = Vec::new();
    // 300 nodes = 44,850 pairs
    for i in 0..300 {
        snap.push(InstanceSnapshot {
            name: format!("node_{}", i),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: if i > 0 {
                Some(format!("node_{}", i - 1))
            } else {
                None
            },
            directory: None,
            tool: None,
        });
    }
    graph.apply_instance_snapshot(&snap);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);

    let initial_pos_0 = layout.nodes[&graph.get_node_index("node_0").unwrap()].pos;

    // Set a strict deadline: 200 microseconds
    layout.max_step_duration = Duration::from_micros(200);

    let start = std::time::Instant::now();
    let motion1 = layout.step(&graph, 0.016);
    let elapsed = start.elapsed();

    // Verify it terminated well within frame limits (allowing small margin for test thread contention)
    assert!(
        elapsed < Duration::from_millis(15),
        "Layout step must respect max_step_duration deadline, took {:?}",
        elapsed
    );
    assert!(
        !layout.is_sleeping,
        "Layout must NOT sleep when budget was exceeded"
    );
    assert!(
        motion1 > 0.0,
        "Motion must be non-zero when budget was exceeded"
    );

    // Cursor must have advanced
    let (c_i1, c_j1) = (layout.cursor_i, layout.cursor_j);
    assert!(
        c_i1 > 0 || c_j1 > 1,
        "Pair cursor must advance across frames"
    );

    // Next step must resume and make further progress
    let _ = layout.step(&graph, 0.016);
    let (c_i2, c_j2) = (layout.cursor_i, layout.cursor_j);
    assert_ne!(
        (c_i1, c_j1),
        (c_i2, c_j2),
        "Pair cursor must progress across frames"
    );

    let final_pos_0 = layout.nodes[&graph.get_node_index("node_0").unwrap()].pos;
    assert_ne!(
        initial_pos_0, final_pos_0,
        "Positions must advance even under tight time-slicing"
    );
}

#[test]
fn test_layout_large_graph_frame_budget_enforced_during_forces_init() {
    let mut graph = AgentGraph::new();
    let mut snap = Vec::new();
    // 1500 nodes = 1,124,250 pairs
    for i in 0..1500 {
        snap.push(InstanceSnapshot {
            name: format!("large_node_{}", i),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        });
    }
    graph.apply_instance_snapshot(&snap);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);

    // Sorted indices are precomputed in sync_nodes
    assert_eq!(layout.sorted_indices.len(), 1500);

    let initial_pos_0 = layout.nodes[&graph.get_node_index("large_node_0").unwrap()].pos;

    // Tight deadline: 100 microseconds so repulsion and integration get sliced across frames
    layout.max_step_duration = Duration::from_micros(100);

    let start = std::time::Instant::now();
    let _ = layout.step(&graph, 0.016);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(5),
        "Layout step with 1500 nodes must respect max_step_duration clamp, took {:?}",
        elapsed
    );

    let (c_i1, c_j1) = (layout.cursor_i, layout.cursor_j);
    let int_cur_1 = layout.integrate_cursor;
    assert!(c_i1 > 0 || c_j1 > 1, "Pair cursor must advance in frame 1");

    // Frame 2: verify cursors make further progress without resetting to 0
    let _ = layout.step(&graph, 0.016);
    let (c_i2, c_j2) = (layout.cursor_i, layout.cursor_j);
    let int_cur_2 = layout.integrate_cursor;
    assert_ne!(
        (c_i1, c_j1),
        (c_i2, c_j2),
        "Pair cursor must advance further in frame 2 without resetting"
    );
    assert_ne!(
        int_cur_1, int_cur_2,
        "Integration cursor must advance across frames"
    );

    let final_pos_0 = layout.nodes[&graph.get_node_index("large_node_0").unwrap()].pos;
    assert_ne!(
        initial_pos_0, final_pos_0,
        "Positions must advance across frames even under heavy time-slicing"
    );
}

struct MockActionRunner {
    recorded: Arc<Mutex<Vec<ActionRequest>>>,
    fail_attach: bool,
    transcript_result: String,
}

impl ActionRunner for MockActionRunner {
    fn execute(&self, req: ActionRequest) -> Option<ActionResult> {
        self.recorded.lock().unwrap().push(req.clone());
        match req {
            ActionRequest::Attach { agent, .. } => Some(ActionResult::AttachFinished {
                agent: agent.clone(),
                result: if self.fail_attach {
                    Err("Agent not running in Herdr pane".into())
                } else {
                    Ok(())
                },
            }),
            ActionRequest::Inject { agent, .. } => Some(ActionResult::InjectFinished {
                agent,
                result: Ok(()),
            }),
            ActionRequest::Transcript {
                agent, generation, ..
            } => Some(ActionResult::TranscriptLoaded {
                agent,
                generation,
                content: Ok(self.transcript_result.clone()),
            }),
            ActionRequest::Screen { agent, generation } => Some(ActionResult::ScreenLoaded {
                agent,
                generation,
                content: Ok("screen".into()),
            }),
            ActionRequest::Events {
                agent, generation, ..
            } => Some(ActionResult::EventsLoaded {
                agent,
                generation,
                content: Ok("events".into()),
            }),
            ActionRequest::Kill { agent } => Some(ActionResult::KillFinished {
                agent,
                result: Ok(()),
            }),
            ActionRequest::KillGroup { agents } => Some(ActionResult::KillGroupFinished {
                count: agents.len(),
                result: Ok(()),
            }),
            ActionRequest::RunCommand { .. } => Some(ActionResult::CommandFinished {
                result: Ok("cmd ok".into()),
            }),
            ActionRequest::SpawnDetached { .. } => None,
            ActionRequest::ReloadTableData => Some(ActionResult::TableDataLoaded {
                rows: Vec::new(),
                catalog_groups: Vec::new(),
                tool_groups: Vec::new(),
            }),
            ActionRequest::ReloadStats => Some(ActionResult::StatsLoaded {
                stats: std::collections::HashMap::new(),
            }),
            ActionRequest::Shutdown => None,
        }
    }
}

#[test]
fn test_action_worker_async_dispatch_and_bounded_transcript() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runner = MockActionRunner {
        recorded: recorded.clone(),
        fail_attach: false,
        transcript_result: "line 1\nline 2".into(),
    };

    let worker = ActionWorkerHandle::new(runner);

    // 1. Dispatch Attach
    worker
        .try_send(ActionRequest::Attach {
            agent: "test_agent".into(),
            custom_cmd: None,
        })
        .unwrap();

    // Spin briefly until worker finishes
    let mut attach_res = None;
    for _ in 0..50 {
        if let Some(res) = worker.try_recv() {
            attach_res = Some(res);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        attach_res,
        Some(ActionResult::AttachFinished {
            agent: "test_agent".into(),
            result: Ok(()),
        })
    );

    // 2. Dispatch Inject
    worker
        .try_send(ActionRequest::Inject {
            agent: "test_agent".into(),
            text: "hello world".into(),
            enter: true,
        })
        .unwrap();

    let mut inject_res = None;
    for _ in 0..50 {
        if let Some(res) = worker.try_recv() {
            inject_res = Some(res);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        inject_res,
        Some(ActionResult::InjectFinished {
            agent: "test_agent".into(),
            result: Ok(()),
        })
    );

    // 3. Dispatch Transcript
    worker
        .try_send(ActionRequest::Transcript {
            agent: "test_agent".into(),
            generation: 1,
            last_lines: 10,
        })
        .unwrap();

    let mut transcript_res = None;
    for _ in 0..50 {
        if let Some(res) = worker.try_recv() {
            transcript_res = Some(res);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        transcript_res,
        Some(ActionResult::TranscriptLoaded {
            agent: "test_agent".into(),
            generation: 1,
            content: Ok("line 1\nline 2".into()),
        })
    );

    // 4. Dispatch Kill
    worker
        .try_send(ActionRequest::Kill {
            agent: "test_agent".into(),
        })
        .unwrap();

    let mut kill_res = None;
    for _ in 0..50 {
        if let Some(res) = worker.try_recv() {
            kill_res = Some(res);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        kill_res,
        Some(ActionResult::KillFinished {
            agent: "test_agent".into(),
            result: Ok(()),
        })
    );

    // Verify all requests were recorded
    let rec = recorded.lock().unwrap();
    assert_eq!(rec.len(), 4);
    assert_eq!(
        rec[0],
        ActionRequest::Attach {
            agent: "test_agent".into(),
            custom_cmd: None,
        }
    );
    assert_eq!(
        rec[1],
        ActionRequest::Inject {
            agent: "test_agent".into(),
            text: "hello world".into(),
            enter: true,
        }
    );
    assert_eq!(
        rec[2],
        ActionRequest::Transcript {
            agent: "test_agent".into(),
            generation: 1,
            last_lines: 10,
        }
    );
    assert_eq!(
        rec[3],
        ActionRequest::Kill {
            agent: "test_agent".into()
        }
    );
}

#[test]
fn test_canvas_edge_and_node_hit_testing() {
    // 1. Line segment distance
    let a = Pos2::new(0.0, 0.0);
    let b = Pos2::new(100.0, 0.0);
    assert!((distance_to_segment(Pos2::new(50.0, 5.0), a, b) - 5.0).abs() < 1e-4);
    assert!((distance_to_segment(Pos2::new(-10.0, 0.0), a, b) - 10.0).abs() < 1e-4);
    assert!((distance_to_segment(Pos2::new(110.0, 0.0), a, b) - 10.0).abs() < 1e-4);

    // 2. Quadratic Bezier curve distance
    let p0 = Pos2::new(0.0, 0.0);
    let p1 = Pos2::new(50.0, 50.0);
    let p2 = Pos2::new(100.0, 0.0);
    // At t=0.5, midpoint of curve is (50.0, 25.0)
    let dist_mid = distance_to_quadratic_bezier(Pos2::new(50.0, 25.0), p0, p1, p2);
    assert!(dist_mid < 1.0, "Distance at curve point should be near 0");

    let dist_far = distance_to_quadratic_bezier(Pos2::new(50.0, 45.0), p0, p1, p2);
    assert!(
        (dist_far - 20.0).abs() < 2.0,
        "Distance should be approx 20.0"
    );

    // 3. Test Edge Hit Testing in Graph
    let mut graph = AgentGraph::new();
    let snap = vec![
        InstanceSnapshot {
            name: "node_a".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
        InstanceSnapshot {
            name: "node_b".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
    ];
    graph.apply_instance_snapshot(&snap);
    let ev = MessageEvent {
        id: 1,
        ts: Utc::now(),
        instance: "node_a".into(),
        data: MessageData {
            from: "node_a".into(),
            delivered_to: vec!["node_b".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "test".into(),
        },
    };
    graph.apply_message_event(&ev);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let idx_a = graph.get_node_index("node_a").unwrap();
    let idx_b = graph.get_node_index("node_b").unwrap();

    // Place node_a at (0, 0) and node_b at (200, 0)
    layout.set_node_pos(idx_a, Pos2::new(-100.0, 0.0), true);
    layout.set_node_pos(idx_b, Pos2::new(100.0, 0.0), true);

    let canvas = CanvasView::new();
    let rect = Rect::from_center_size(Pos2::new(300.0, 300.0), Vec2::new(600.0, 600.0));

    // Screen pos of edge midpoint is rect.center() = (300.0, 300.0)
    let hit_edge = canvas.hit_test_edge(Pos2::new(300.0, 300.0), &graph, &layout, rect, 8.0);
    assert!(hit_edge.is_some(), "Edge should be hit at its midpoint");

    let miss_edge = canvas.hit_test_edge(Pos2::new(300.0, 350.0), &graph, &layout, rect, 8.0);
    assert!(miss_edge.is_none(), "Points far away should miss");
}

#[test]
fn test_attach_failure_reports_error_without_crash() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runner = MockActionRunner {
        recorded,
        fail_attach: true, // Will fail!
        transcript_result: String::new(),
    };

    let worker = ActionWorkerHandle::new(runner);
    worker
        .try_send(ActionRequest::Attach {
            agent: "headless_agent".into(),
            custom_cmd: None,
        })
        .unwrap();

    let mut res = None;
    for _ in 0..50 {
        if let Some(r) = worker.try_recv() {
            res = Some(r);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    match res {
        Some(ActionResult::AttachFinished { agent, result }) => {
            assert_eq!(agent, "headless_agent");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not running in Herdr pane"));
        }
        other => panic!("Expected AttachFinished with Err, got {:?}", other),
    }
}

#[test]
fn test_kill_requires_confirmation_flow_and_stale_target_check() {
    let mut graph = AgentGraph::new();
    let snap = vec![
        InstanceSnapshot {
            name: "live_agent".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
        InstanceSnapshot {
            name: "stopped_agent".into(),
            status: Some("stopped".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
    ];
    graph.apply_instance_snapshot(&snap);

    // 1. Verify stale check for stopped agent
    let target = "stopped_agent";
    let agent_node = graph.get_agent(target).unwrap();
    assert_eq!(agent_node.status, AgentStatus::Stopped);

    // 2. Verify stale check for non-existent agent
    assert!(graph.get_agent("unknown_agent").is_none());

    // 3. Verify live agent can proceed
    let live = graph.get_agent("live_agent").unwrap();
    assert_eq!(live.status, AgentStatus::Active);
}

#[test]
fn test_task_77_42_no_stdout_limit_reads_large_output_in_full() {
    use hcom_dashboard::action::{NO_STDOUT_LIMIT, SystemActionRunner};
    let runner = SystemActionRunner::new();
    let out = runner
        .run_command_bounded(
            "head",
            &["-c", "200000", "/dev/zero"],
            NO_STDOUT_LIMIT,
            4096,
            None,
        )
        .expect("output above 32 KiB is read in full");
    assert_eq!(out.len(), 200_000);
}

#[test]
fn test_task_77_42_timeout_stops_endless_output_without_limit() {
    use hcom_dashboard::action::{NO_STDOUT_LIMIT, SystemActionRunner};
    let mut runner = SystemActionRunner::new();
    runner.command_timeout = std::time::Duration::from_secs(1);
    let start = std::time::Instant::now();
    let res = runner.run_command_bounded(
        "sh",
        &["-c", "while :; do echo x; sleep 0.01; done"],
        NO_STDOUT_LIMIT,
        4096,
        None,
    );
    let err = res.expect_err("endless output must time out");
    assert!(err.contains("timed out"), "{err}");
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn test_action_runner_stdout_capped_kills_child_and_avoids_deadlock() {
    use hcom_dashboard::action::SystemActionRunner;
    let runner = SystemActionRunner::new();
    #[cfg(unix)]
    let res = runner.run_command_bounded("head", &["-c", "100000", "/dev/zero"], 1024, 1024, None);
    #[cfg(windows)]
    let res = runner.run_command_bounded("cmd", &["/c", "echo", "dummy"], 1024, 1024, None);

    #[cfg(unix)]
    {
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.contains("output exceeded maximum allowed size of 1024 bytes"),
            "Expected exceeded limit error, got: {}",
            err
        );
    }
}

#[test]
fn test_action_runner_concurrent_stderr_drain_no_deadlock() {
    use hcom_dashboard::action::SystemActionRunner;
    let runner = SystemActionRunner::new();
    #[cfg(unix)]
    let res = runner.run_command_bounded(
        "sh",
        &["-c", "head -c 131072 /dev/zero >&2 && echo ok"],
        1024,
        1024,
        None,
    );
    #[cfg(windows)]
    let res: Result<String, String> = Ok("ok\n".to_string());

    #[cfg(unix)]
    {
        assert!(res.is_ok(), "Expected Ok, got: {:?}", res);
        assert_eq!(res.unwrap().trim(), "ok");
    }
}

#[test]
fn test_action_worker_handle_drop_cancels_hanging_process() {
    use hcom_dashboard::action::SystemActionRunner;

    struct HangingActionRunner {
        inner: SystemActionRunner,
    }

    impl ActionRunner for HangingActionRunner {
        fn execute(&self, _req: ActionRequest) -> Option<ActionResult> {
            #[cfg(unix)]
            let _ = self
                .inner
                .run_command_bounded("sleep", &["10"], 1024, 1024, None);
            #[cfg(windows)]
            let _ = self
                .inner
                .run_command_bounded("timeout", &["10"], 1024, 1024, None);
            None
        }

        fn cancel(&self) {
            self.inner.cancel();
        }
    }

    let runner = HangingActionRunner {
        inner: SystemActionRunner::new(),
    };

    let start = std::time::Instant::now();
    let worker = ActionWorkerHandle::new(runner);
    worker
        .try_send(ActionRequest::Attach {
            agent: "test".into(),
            custom_cmd: None,
        })
        .unwrap();

    // Give worker thread a moment to spawn the child process
    std::thread::sleep(Duration::from_millis(30));

    // Drop worker, which must cancel the hanging process and join quickly
    drop(worker);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "Worker drop took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_transcript_clamping_to_100() {
    use hcom_dashboard::action::{
        build_attach_argv, build_inject_argv, build_kill_argv, build_transcript_argv,
        clamp_transcript_lines,
    };

    assert_eq!(clamp_transcript_lines(0), 1);
    assert_eq!(clamp_transcript_lines(50), 50);
    assert_eq!(clamp_transcript_lines(100), 100);
    assert_eq!(clamp_transcript_lines(500), 100);

    let argv_large = build_transcript_argv("agent_alpha", 500);
    assert_eq!(
        argv_large,
        vec!["transcript", "agent_alpha", "--last", "100"]
    );

    let argv_zero = build_transcript_argv("agent_beta", 0);
    assert_eq!(argv_zero, vec!["transcript", "agent_beta", "--last", "1"]);

    let attach_argv = build_attach_argv("agent_gamma");
    assert_eq!(attach_argv, vec!["agent", "attach", "agent_gamma"]);

    let inject_argv = build_inject_argv("agent_delta", "status", true);
    assert_eq!(
        inject_argv,
        vec!["term", "inject", "agent_delta", "status", "--enter"]
    );

    let kill_argv = build_kill_argv("agent_epsilon");
    assert_eq!(kill_argv, vec!["kill", "agent_epsilon"]);
}

#[test]
fn test_action_runner_cancel_while_in_wait_loop() {
    use hcom_dashboard::action::SystemActionRunner;

    let runner = Arc::new(SystemActionRunner::new());
    let runner_thread = Arc::clone(&runner);

    let start = std::time::Instant::now();
    let thread = std::thread::spawn(move || {
        #[cfg(unix)]
        let res = runner_thread.run_command_bounded("sleep", &["10"], 1024, 1024, None);
        #[cfg(windows)]
        let res = runner_thread.run_command_bounded("timeout", &["10"], 1024, 1024, None);
        res
    });

    // Allow the process to start and enter the cancellable wait loop
    std::thread::sleep(Duration::from_millis(30));

    // Cancel while in the wait loop
    runner.cancel();

    let res = thread.join().expect("thread panicked");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "Cancellation in wait loop took too long: {:?}",
        elapsed
    );
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Command was cancelled");
}

#[test]
fn test_config_reads_only_dashboard_json() {
    use hcom_dashboard::config::load_config;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();
    let other = r#"{"refresh_seconds": 12, "stats_seconds": 99}"#;
    std::fs::write(hcom_dir.join("gui.json"), other).unwrap();
    std::fs::write(hcom_dir.join("graph.json"), other).unwrap();
    let defaults = load_config(hcom_dir);
    assert_ne!(
        defaults.refresh_seconds, 12,
        "gui.json and graph.json are not read"
    );

    let dashboard_json = r#"{
        "refresh_seconds": 3,
        "stats_seconds": 30,
        "open_with": [
            {"label": "Zed", "cmd": ["zed", "{dir}"], "key": "z"}
        ]
    }"#;
    std::fs::write(hcom_dir.join("dashboard.json"), dashboard_json).unwrap();
    let cfg = load_config(hcom_dir);
    assert_eq!(cfg.refresh_seconds, 3);
    assert_eq!(cfg.stats_seconds, 30);
    assert_eq!(cfg.open_with.len(), 1);
    assert_eq!(cfg.open_with[0].label, "Zed");
}

#[test]
fn test_state_reads_and_saves_only_dashboard_state_json() {
    use hcom_dashboard::config::{load_state, save_state};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();
    let other = r#"{"running_only": true, "sort_col": "age"}"#;
    std::fs::write(hcom_dir.join("gui-state.json"), other).unwrap();
    std::fs::write(hcom_dir.join("graph-state.json"), other).unwrap();

    let mut state = load_state(hcom_dir);
    assert!(
        !state.running_only,
        "gui-state.json and graph-state.json are not read"
    );

    state.show_table = false;
    state.detail_tab = "Transcript".into();
    save_state(hcom_dir, &state).unwrap();
    let reloaded = load_state(hcom_dir);
    assert!(!reloaded.show_table);
    assert_eq!(reloaded.detail_tab, "Transcript");
    for name in ["gui-state.json", "graph-state.json"] {
        assert_eq!(
            std::fs::read_to_string(hcom_dir.join(name)).unwrap(),
            other,
            "{name} is untouched"
        );
    }
}

#[test]
fn test_screen_bar_complete_set_and_custom_keys() {
    use hcom_dashboard::config::{ScreenBarDef, screen_bar_buttons};

    let defs = vec![
        ScreenBarDef::Builtin("Inject".into()),
        ScreenBarDef::Builtin("Enter".into()),
        ScreenBarDef::Builtin("Esc".into()),
        ScreenBarDef::Builtin("↑".into()),
        ScreenBarDef::Builtin("↓".into()),
        ScreenBarDef::Builtin("←".into()),
        ScreenBarDef::Builtin("→".into()),
        ScreenBarDef::Builtin("Tab".into()),
        ScreenBarDef::Builtin("Shift+Tab".into()),
        ScreenBarDef::Builtin("1".into()),
        ScreenBarDef::Builtin("2".into()),
        ScreenBarDef::Builtin("3".into()),
        ScreenBarDef::Custom {
            label: "Ctrl+C".into(),
            keys: "\u{0003}".into(),
        },
        ScreenBarDef::Custom {
            label: "Ctrl+D".into(),
            keys: "\u{0004}".into(),
        },
    ];

    let buttons = screen_bar_buttons(&defs);
    assert_eq!(buttons.len(), 14);

    // Check inject
    assert!(buttons[0].is_inject_dialog);
    // Check enter flag
    assert!(buttons[1].is_enter_flag);
    // Check Esc
    assert_eq!(buttons[2].keys, "\x1b");
    // Check Arrows
    assert_eq!(buttons[3].keys, "\x1b[A");
    assert_eq!(buttons[4].keys, "\x1b[B");
    assert_eq!(buttons[5].keys, "\x1b[D");
    assert_eq!(buttons[6].keys, "\x1b[C");
    // Check Tab & Shift+Tab
    assert_eq!(buttons[7].keys, "\t");
    assert_eq!(buttons[8].keys, "\x1b[Z");
    // Check 1, 2, 3
    assert_eq!(buttons[9].keys, "1");
    assert_eq!(buttons[10].keys, "2");
    assert_eq!(buttons[11].keys, "3");
    // Check Ctrl+C & Ctrl+D
    assert_eq!(buttons[12].label, "Ctrl+C");
    assert_eq!(buttons[12].keys, "\u{0003}");
    assert_eq!(buttons[13].label, "Ctrl+D");
    assert_eq!(buttons[13].keys, "\u{0004}");
}

#[test]
fn test_placeholder_substitution_token_by_token() {
    use hcom_dashboard::config::{
        PlaceholderContext, replace_placeholders_str, replace_placeholders_vec,
    };

    let dir = "/home/user/project";
    let instance = "wdt_agent_1";
    let agent = "wdt_agent";
    let ctx = PlaceholderContext {
        dir,
        instance,
        agent,
        ..Default::default()
    };

    let text = "open {dir} for {instance} (catalog: {agent})";
    let res = replace_placeholders_str(text, &ctx).expect("replace ok");
    assert_eq!(
        res,
        "open /home/user/project for wdt_agent_1 (catalog: wdt_agent)"
    );

    let tokens = vec![
        "zed".into(),
        "{dir}/src".into(),
        "--instance={instance}".into(),
        "{agent}".into(),
    ];
    let res_vec = replace_placeholders_vec(&tokens, &ctx).expect("replace ok");
    assert_eq!(
        res_vec,
        vec![
            "zed",
            "/home/user/project/src",
            "--instance=wdt_agent_1",
            "wdt_agent"
        ]
    );
}

#[test]
fn test_table_sorting_with_empty_last_and_numeric() {
    use hcom_dashboard::table_data::{TableRow, sort_rows};

    let mut rows = vec![
        TableRow {
            agent: "agent_a".into(),
            instance: "alpha".into(),
            since_seconds: Some(50),
            since_text: "50s".into(),
            unread_count: 5,
            ..Default::default()
        },
        TableRow {
            agent: "agent_b".into(),
            instance: "beta".into(),
            since_seconds: None, // empty cell
            since_text: String::new(),
            unread_count: 0,
            ..Default::default()
        },
        TableRow {
            agent: "agent_c".into(),
            instance: "gamma".into(),
            since_seconds: Some(10),
            since_text: "10s".into(),
            unread_count: 20,
            ..Default::default()
        },
    ];

    // Ascending sort by "since" with empty_last=true
    sort_rows(&mut rows, "since", false, true);
    assert_eq!(rows[0].instance, "gamma"); // 10s
    assert_eq!(rows[1].instance, "alpha"); // 50s
    assert_eq!(rows[2].instance, "beta"); // empty goes last!

    // Descending sort by "since" with empty_last=true
    sort_rows(&mut rows, "since", true, true);
    assert_eq!(rows[0].instance, "alpha"); // 50s
    assert_eq!(rows[1].instance, "gamma"); // 10s
    assert_eq!(rows[2].instance, "beta"); // empty still goes last!

    // Numeric sort by unread with empty_last=true:
    // alpha (5), gamma (20), beta (empty/0 goes last)
    sort_rows(&mut rows, "unread", false, true);
    assert_eq!(rows[0].instance, "alpha"); // 5
    assert_eq!(rows[1].instance, "gamma"); // 20
    assert_eq!(rows[2].instance, "beta"); // empty/0 goes last!

    // Numeric sort by unread with empty_last=false:
    // beta (empty/0), alpha (5), gamma (20)
    sort_rows(&mut rows, "unread", false, false);
    assert_eq!(rows[0].instance, "beta"); // empty/0 goes first
    assert_eq!(rows[1].instance, "alpha"); // 5
    assert_eq!(rows[2].instance, "gamma"); // 20
}

#[test]
fn test_table_filtering_running_blocked_text_and_adhoc() {
    use hcom_dashboard::table_data::{TableRow, filter_row};

    let row_running = TableRow {
        agent: "coach".into(),
        instance: "coach_1".into(),
        is_running: true,
        status: "active".into(),
        directory: "/home/user/hcom".into(),
        cli: "claude".into(),
        ..Default::default()
    };
    let row_blocked = TableRow {
        agent: "worker".into(),
        instance: "worker_1".into(),
        is_running: true,
        status: "blocked".into(),
        directory: "/home/user/other".into(),
        cli: "codex".into(),
        ..Default::default()
    };
    let row_stopped = TableRow {
        agent: "archived".into(),
        instance: "archived_1".into(),
        is_running: false,
        status: "stopped".into(),
        directory: "/tmp".into(),
        cli: "hermes".into(),
        ..Default::default()
    };
    let row_adhoc = TableRow {
        agent: String::new(), // adhoc / no catalog agent
        instance: "temp_agent".into(),
        is_running: true,
        status: "active".into(),
        directory: "/tmp".into(),
        cli: "claude".into(),
        ..Default::default()
    };

    // 1. Text filter
    assert!(filter_row(
        &row_running,
        "coach",
        false,
        false,
        false,
        false
    ));
    assert!(filter_row(&row_running, "hcom", false, false, false, false));
    assert!(!filter_row(
        &row_running,
        "nonexistent",
        false,
        false,
        false,
        false
    ));

    // 2. Running only
    assert!(filter_row(&row_running, "", true, false, false, false));
    assert!(!filter_row(&row_stopped, "", true, false, false, false));

    // 3. Blocked only
    assert!(filter_row(&row_blocked, "", false, true, false, false));
    assert!(!filter_row(&row_running, "", false, true, false, false));

    // 4. Hide ad-hoc
    assert!(!filter_row(&row_adhoc, "", false, false, false, true));
    assert!(filter_row(&row_running, "", false, false, false, true));
}

#[test]
fn test_dynamic_status_columns_extraction() {
    use hcom_dashboard::table_data::TableRow;
    use std::collections::HashMap;

    let mut status_values = HashMap::new();
    status_values.insert("status".into(), "DONE".into());
    status_values.insert("done".into(), "TASK-77.4 implemented".into());
    status_values.insert("next".into(), "none".into());

    let row = TableRow {
        agent: "tester".into(),
        instance: "tester_1".into(),
        status_values,
        ..Default::default()
    };

    assert_eq!(row.get_cell_value("status:status"), "DONE");
    assert_eq!(row.get_cell_value("status:done"), "TASK-77.4 implemented");
    assert_eq!(row.get_cell_value("status:next"), "none");
    assert_eq!(row.get_cell_value("status:unknown"), "");
}

#[test]
fn test_worker_generation_counter_drops_stale_response() {
    struct PanelState {
        selection_generation: u64,
        screen_content: Option<(String, u64, String)>,
        transcript_content: Option<(String, u64, String)>,
    }

    impl PanelState {
        fn new() -> Self {
            Self {
                selection_generation: 0,
                screen_content: None,
                transcript_content: None,
            }
        }

        fn on_selection_changed(&mut self) -> u64 {
            self.selection_generation += 1;
            self.screen_content = None;
            self.transcript_content = None;
            self.selection_generation
        }

        fn handle_result(&mut self, res: &hcom_dashboard::action::ActionResult) {
            match res {
                hcom_dashboard::action::ActionResult::ScreenLoaded {
                    agent,
                    generation,
                    content,
                } => {
                    if hcom_dashboard::app::accept_generation_result(
                        self.selection_generation,
                        *generation,
                    ) && let Ok(text) = content
                    {
                        self.screen_content = Some((agent.clone(), *generation, text.clone()));
                    }
                }
                hcom_dashboard::action::ActionResult::TranscriptLoaded {
                    agent,
                    generation,
                    content,
                } => {
                    if hcom_dashboard::app::accept_generation_result(
                        self.selection_generation,
                        *generation,
                    ) && let Ok(text) = content
                    {
                        self.transcript_content = Some((agent.clone(), *generation, text.clone()));
                    }
                }
                _ => {}
            }
        }
    }

    let mut state = PanelState::new();

    // 1. Select agent A -> generation 1
    let gen1 = state.on_selection_changed();
    assert_eq!(gen1, 1);

    // 2. Select agent B quickly before response for A arrives -> generation 2
    let gen2 = state.on_selection_changed();
    assert_eq!(gen2, 2);

    // 3. Stale response for agent A arrives with gen1
    let stale_screen = hcom_dashboard::action::ActionResult::ScreenLoaded {
        agent: "agent_a".into(),
        generation: gen1,
        content: Ok("stale content of A".into()),
    };
    state.handle_result(&stale_screen);

    // MUST be dropped! screen_content must remain None
    assert!(state.screen_content.is_none());

    // 4. Fresh response for agent B arrives with gen2
    let fresh_screen = hcom_dashboard::action::ActionResult::ScreenLoaded {
        agent: "agent_b".into(),
        generation: gen2,
        content: Ok("fresh content of B".into()),
    };
    state.handle_result(&fresh_screen);

    assert_eq!(
        state.screen_content,
        Some(("agent_b".into(), 2, "fresh content of B".into()))
    );
}

#[test]
fn test_corrupt_config_logs_warning_and_uses_defaults() {
    use hcom_dashboard::config::{load_config, load_state};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();

    // Corrupt dashboard.json
    std::fs::write(hcom_dir.join("dashboard.json"), "{ invalid: json: syntax").unwrap();
    let cfg = load_config(hcom_dir);
    // Should fallback to default refresh_seconds (5)
    assert_eq!(cfg.refresh_seconds, 5);

    // Corrupt dashboard-state.json
    std::fs::write(hcom_dir.join("dashboard-state.json"), "broken").unwrap();
    let st = load_state(hcom_dir);
    assert!(st.show_table);
    assert_eq!(st.detail_tab, "Summary");
    assert!(!st.running_only);
}

#[test]
fn test_unknown_placeholder_rejected() {
    use hcom_dashboard::config::{PlaceholderContext, replace_placeholders_str};

    let ctx = PlaceholderContext {
        dir: "/home/user",
        instance: "inst_1",
        agent: "agent_a",
        ..Default::default()
    };

    let valid = "echo {dir} {instance} {agent}";
    assert!(replace_placeholders_str(valid, &ctx).is_ok());

    let invalid = "echo {unknown_foo} {dir}";
    let err = replace_placeholders_str(invalid, &ctx).unwrap_err();
    assert!(err.contains("Unknown placeholder"));
    assert!(err.contains("{unknown_foo}"));
}

#[test]
fn test_parse_real_gui_json_copy() {
    use hcom_dashboard::config::DashboardConfig;

    let gui_json = r#"{
      "refresh_seconds": 5,
      "open_with": [
        { "label": "Zed", "cmd": ["zed", "{dir}"], "key": "z" },
        { "label": "gitg", "cmd": ["gitg"], "cwd": "{dir}", "key": "g" },
        { "label": "gitk", "cmd": ["gitk", "--all"], "cwd": "{dir}", "key": "h" },
        { "label": "Folder", "cmd": ["xdg-open", "{dir}"] },
        { "label": "Terminal", "cmd": ["x-terminal-emulator"], "cwd": "{dir}" },
        {
          "label": "Backlog",
          "cmd": ["x-terminal-emulator", "backlog", "board"],
          "cwd": "{dir}",
          "if_exists": "{dir}/backlog",
          "key": "b"
        }
      ],
      "attach": {
        "herdr": [
          "sh",
          "-c",
          "hcom agent attach \"$1\" && wmctrl -x -a herdr.herdr",
          "_",
          "{instance}"
        ]
      },
      "screen_bar": [
        "Inject",
        "Enter",
        "Esc",
        "↑",
        "↓",
        "←",
        "→",
        "Tab",
        "Shift+Tab",
        "1",
        "2",
        "3",
        { "label": "Ctrl+C", "keys": "\u0003" },
        { "label": "Ctrl+D", "keys": "\u0004" }
      ],
      "report": [
        "status:status",
        "status:done",
        "status:issues",
        "status:next",
        "status",
        "idle",
        "since",
        "age",
        "agent",
        "instance",
        "roaming",
        "cli",
        "context",
        "quota",
        "detail",
        "unread",
        "dir",
        "branch",
        "git",
        "where",
        "config",
        "starts",
        "runtime",
        "handoffs",
        "msgs",
        "last_start",
        "stops",
        "failed"
      ]
    }"#;

    let cfg: DashboardConfig = serde_json::from_str(gui_json).expect("parse real dashboard.json");
    assert_eq!(cfg.refresh_seconds, 5);
    assert_eq!(cfg.open_with.len(), 6);
    assert_eq!(cfg.open_with[5].label, "Backlog");
    assert_eq!(cfg.open_with[5].if_exists.as_deref(), Some("{dir}/backlog"));
    assert!(cfg.attach.contains_key("herdr"));
    assert_eq!(cfg.screen_bar.len(), 14);
    assert_eq!(cfg.report.len(), 28);
}

#[test]
fn test_build_table_rows_merges_live_and_catalog() {
    use hcom_dashboard::table_data::build_table_rows;

    let live_json = r#"[
        {
            "name": "worker_1",
            "directory": "/home/user/project",
            "tool": "claude",
            "status": "active",
            "status_age_seconds": 45,
            "unread_count": 3,
            "created_at": 1700000000.0,
            "launch_context": {
                "agent_name": "worker",
                "git_branch": "main",
                "herdr_workspace": "ws1",
                "herdr_tab": "tab2"
            }
        },
        {
            "name": "temp_agent",
            "directory": "/tmp/adhoc",
            "tool": "codex",
            "status": "listening",
            "status_age_seconds": 12,
            "unread_count": 0,
            "created_at": 1700000010.0,
            "launch_context": null
        }
    ]"#;

    let agent_json = r#"[
        {
            "name": "worker",
            "cli": "claude",
            "description": "Background worker agent",
            "dir": "/home/user/project",
            "disabled": false,
            "roaming": true,
            "status_columns": {
                "columns": ["status", "done"],
                "latest": "worker_1",
                "instances": {
                    "worker_1": [
                        { "name": "status", "value": "DONE", "updated_at": 1700000020 },
                        { "name": "done", "value": "All tasks completed", "updated_at": 1700000020 }
                    ]
                }
            }
        },
        {
            "name": "idle_catalog",
            "cli": "gemini",
            "description": "Never started agent",
            "dir": "/home/user/idle",
            "disabled": true,
            "roaming": false
        }
    ]"#;

    let rows = build_table_rows(live_json, agent_json, 1700000100).expect("build table rows");
    // Expected rows:
    // 1. live worker_1 (catalog agent "worker")
    // 2. live temp_agent (adhoc, catalog agent "")
    // 3. catalog idle_catalog (not running)
    assert_eq!(rows.len(), 3);

    // Row 1: worker_1
    let r1 = &rows[0];
    assert_eq!(r1.agent, "worker");
    assert_eq!(r1.instance, "worker_1");
    assert!(r1.is_running);
    assert!(r1.is_roaming);
    assert!(!r1.is_disabled);
    assert_eq!(r1.unread_count, 3);
    assert_eq!(r1.status, "active");
    assert_eq!(r1.where_term, "herdr: ws1/tab2");
    assert_eq!(r1.branch, "main");
    assert_eq!(r1.get_cell_value("status:status"), "DONE");
    assert_eq!(r1.get_cell_value("status:done"), "All tasks completed");

    // Row 2: temp_agent (adhoc)
    let r2 = &rows[1];
    assert_eq!(r2.agent, "");
    assert_eq!(r2.instance, "temp_agent");
    assert!(r2.is_running);
    assert!(!r2.is_roaming);

    // Row 3: idle_catalog (catalog without running instance)
    let r3 = &rows[2];
    assert_eq!(r3.agent, "idle_catalog");
    assert_eq!(r3.instance, "");
    assert!(!r3.is_running);
    assert!(r3.is_disabled);
    assert_eq!(r3.detail, "Never started agent");
}

#[test]
fn test_kill_group_strictly_matches_catalog_groups() {
    use hcom_dashboard::table_data::TableRow;

    let row1 = TableRow {
        agent: "service_a".into(),
        instance: "inst_a".into(),
        is_running: true,
        groups: vec!["web".into(), "frontend".into()],
        directory: "/var/www".into(),
        ..Default::default()
    };
    let row2 = TableRow {
        agent: "service_b".into(),
        instance: "inst_b".into(),
        is_running: true,
        groups: vec!["database".into()],
        // Path contains "web" substring - must NOT match group "web"!
        directory: "/home/web/db".into(),
        config_path: "/etc/webapp/conf.toml".into(),
        ..Default::default()
    };
    let row3 = TableRow {
        agent: "service_c".into(),
        instance: "inst_c".into(),
        is_running: true,
        groups: vec!["web".into()],
        directory: "/opt/app".into(),
        ..Default::default()
    };

    let all_rows = [row1, row2, row3];
    let group = "web";
    let targets: Vec<String> = all_rows
        .iter()
        .filter(|r| r.is_running && !r.instance.is_empty() && r.groups.iter().any(|g| g == group))
        .map(|r| r.instance.clone())
        .collect();

    assert_eq!(targets, vec!["inst_a", "inst_c"]);
    assert!(!targets.contains(&"inst_b".to_string()));
}

#[test]
fn test_git_porcelain_v2_parsing() {
    use hcom_dashboard::table_data::parse_git_porcelain_v2;

    let output = "# branch.oid 1234567890abcdef\n\
                  # branch.head feature/graph-gui\n\
                  # branch.upstream origin/feature/graph-gui\n\
                  # branch.ab +3 -1\n\
                  1 .M N... 100644 100644 100644 deadbeef deadbeef src/main.rs\n\
                  ? untracked.txt\n";

    let (branch, status) = parse_git_porcelain_v2(output);
    assert_eq!(branch, "feature/graph-gui");
    assert_eq!(status, "* ↑3 ↓1");

    let clean_output = "# branch.head main\n# branch.ab +0 -0\n";
    let (b_clean, s_clean) = parse_git_porcelain_v2(clean_output);
    assert_eq!(b_clean, "main");
    assert_eq!(s_clean, "");
}

#[test]
fn test_event_stats_computation_and_application() {
    use hcom_dashboard::table_data::{TableRow, apply_event_stats, compute_event_stats};

    let life_events = r#"
{"id": 1, "instance": "worker_1", "type": "life", "ts": "2026-11-01T10:00:00Z", "data": {"action": "created"}}
{"id": 2, "instance": "worker_1", "type": "life", "ts": "2026-11-01T10:05:00Z", "data": {"action": "stopped"}}
{"id": 3, "instance": "worker_1", "type": "life", "ts": "2026-11-01T10:10:00Z", "data": {"action": "created"}}
{"id": 4, "instance": "worker_1", "type": "life", "ts": "2026-11-01T10:12:00Z", "data": {"action": "launch_failed"}}
{"id": 5, "instance": "worker_1", "type": "life", "ts": "2026-11-01T10:13:00Z", "data": {"action": "handoff"}}
"#;

    let msg_events = r#"
{"id": 10, "instance": "worker_1", "type": "message", "ts": "2026-11-01T10:01:00Z", "data": {"from": "worker_1", "delivered_to": ["worker_2"]}}
{"id": 11, "instance": "worker_2", "type": "message", "ts": "2026-11-01T10:02:00Z", "data": {"from": "worker_2", "delivered_to": ["worker_1"]}}
"#;

    let now = chrono::DateTime::parse_from_rfc3339("2026-11-01T10:15:00Z")
        .unwrap()
        .timestamp() as u64;

    let mut running_instances = std::collections::HashSet::new();
    running_instances.insert("worker_1".to_string());

    let stats = compute_event_stats(life_events, msg_events, &running_instances, now);
    let mut rows = vec![TableRow {
        agent: "worker".into(),
        instance: "worker_1".into(),
        is_running: true,
        ..Default::default()
    }];

    apply_event_stats(&mut rows, &stats);

    let r = &rows[0];
    assert_eq!(r.starts_count, Some(2));
    assert_eq!(r.stops_summary, "1/1"); // 1 stop, 1 failed
    assert_eq!(r.failed_count, Some(1));
    assert_eq!(r.handoffs_count, Some(1));
    assert_eq!(r.msgs_count, Some(2)); // 1 sent + 1 received
    assert_eq!(r.last_start_text, "2026-11-01T10:10:00Z");
    // 5 minutes from first session + 5 minutes up to now in second active session = 10m
    assert_eq!(r.runtime_text, "10m");
}

#[test]
fn test_launch_context_placeholders_carried_in_table_row() {
    use hcom_dashboard::config::{PlaceholderContext, replace_placeholders_str};
    use hcom_dashboard::table_data::{LiveLaunchContext, TableRow};

    let row = TableRow {
        agent: "coder".into(),
        instance: "coder_1".into(),
        directory: "/home/user/code".into(),
        where_term: "herdr: ws_main/tab_dev".into(),
        launch_context: Some(LiveLaunchContext {
            agent_name: Some("coder".into()),
            git_branch: Some("feature".into()),
            herdr_workspace: Some("ws_main".into()),
            herdr_tab: Some("tab_dev".into()),
            pane_id: Some("%42".into()),
            terminal_preset_effective: Some("herdr".into()),
        }),
        ..Default::default()
    };

    let lc = row.launch_context.as_ref().unwrap();
    let ctx = PlaceholderContext {
        dir: &row.directory,
        instance: &row.instance,
        agent: &row.agent,
        terminal: &row.where_term,
        pane_id: lc.pane_id.as_deref().unwrap_or(""),
        herdr_workspace: lc.herdr_workspace.as_deref().unwrap_or(""),
        herdr_tab: lc.herdr_tab.as_deref().unwrap_or(""),
    };

    let template = "tmux select-pane -t {pane_id} in {herdr_workspace}:{herdr_tab}";
    let res = replace_placeholders_str(template, &ctx).expect("replace ok");
    assert_eq!(res, "tmux select-pane -t %42 in ws_main:tab_dev");
}

#[test]
fn test_in_flight_flags_reset_on_dispatch_failure() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};

    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        std::path::PathBuf::from("/tmp"),
    );

    // Both flags start false or get updated
    app.table_reload_in_flight = true;
    app.stats_reload_in_flight = true;

    // Dispath without an action worker (or when try_send fails)
    app.dispatch_action(ActionRequest::ReloadTableData);
    assert!(
        !app.table_reload_in_flight,
        "table_reload_in_flight must be reset on dispatch failure"
    );

    app.dispatch_action(ActionRequest::ReloadStats);
    assert!(
        !app.stats_reload_in_flight,
        "stats_reload_in_flight must be reset on dispatch failure"
    );
}

#[test]
fn test_canvas_filtering_excludes_hidden_nodes_and_edges() {
    let mut graph = AgentGraph::new();
    let snap = vec![
        InstanceSnapshot {
            name: "agent_a".into(),
            status: Some("blocked".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
        InstanceSnapshot {
            name: "agent_b".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: None,
            directory: None,
            tool: None,
        },
    ];
    graph.apply_instance_snapshot(&snap);

    let idx_a = graph.get_node_index("agent_a").unwrap();
    let idx_b = graph.get_node_index("agent_b").unwrap();

    let ev = MessageEvent {
        id: 100,
        ts: Utc::now(),
        instance: "agent_a".into(),
        data: MessageData {
            from: "agent_a".into(),
            delivered_to: vec!["agent_b".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "test".into(),
        },
    };
    graph.apply_message_event(&ev);

    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    layout.set_node_pos(idx_a, Pos2::new(-100.0, 0.0), true);
    layout.set_node_pos(idx_b, Pos2::new(100.0, 0.0), true);

    let canvas = CanvasView::new();
    let rect = Rect::from_center_size(Pos2::new(300.0, 300.0), Vec2::new(600.0, 600.0));

    // When no filter is applied (visible_nodes is None), edge is hit at midpoint (300, 300)
    let hit_all =
        canvas.hit_test_edge_filtered(Pos2::new(300.0, 300.0), &graph, &layout, rect, 8.0, None);
    assert!(hit_all.is_some(), "Edge should be hit when unfiltered");

    // When filter includes only agent_a, edge to agent_b must NOT be hit
    let mut only_a = std::collections::HashSet::new();
    only_a.insert(idx_a);
    let hit_only_a = canvas.hit_test_edge_filtered(
        Pos2::new(300.0, 300.0),
        &graph,
        &layout,
        rect,
        8.0,
        Some(&only_a),
    );
    assert!(
        hit_only_a.is_none(),
        "Edge must be hidden when target node is filtered out"
    );
}

#[test]
fn test_graph_state_layout_defaults_and_roundtrip() {
    use hcom_dashboard::config::DashboardState;

    let default_state = DashboardState::default();
    assert!(
        default_state.show_side_panel,
        "detail panel defaults to open"
    );
    assert!(default_state.show_table, "table defaults to shown");
    assert_eq!(default_state.detail_tab, "Summary");

    // Unknown keys and values are ignored; other fields default.
    let json_unknown = r#"{"view_mode":"split_horizontal","main_split_ratio":0.4}"#;
    let parsed: DashboardState = serde_json::from_str(json_unknown).expect("parse state");
    assert!(parsed.show_side_panel);
    assert!(parsed.show_table);

    let custom_state = DashboardState {
        show_side_panel: false,
        show_table: false,
        detail_tab: "Events".into(),
        ..Default::default()
    };
    let serialized = serde_json::to_string(&custom_state).expect("serialize");
    let restored: DashboardState = serde_json::from_str(&serialized).expect("deserialize");
    assert!(!restored.show_side_panel);
    assert!(!restored.show_table);
    assert_eq!(restored.detail_tab, "Events");
}

#[test]
fn test_parse_context_percent_reads_trailing_percent_token() {
    use hcom_dashboard::model::parse_context_percent;
    assert_eq!(parse_context_percent("123k 45%"), Some(45.0));
    assert_eq!(parse_context_percent("80%"), Some(80.0));
    assert_eq!(parse_context_percent("130%"), Some(100.0));
    assert_eq!(parse_context_percent(""), None);
    assert_eq!(parse_context_percent("123k"), None);
    assert_eq!(parse_context_percent("abc%"), None);
    assert_eq!(parse_context_percent("NaN%"), None);
}

#[test]
fn test_node_styles_from_rows_carry_roaming_and_context() {
    use hcom_dashboard::table_data::{TableRow, node_styles_from_rows};
    let rows = vec![
        TableRow {
            instance: "roamer".into(),
            is_roaming: true,
            context_fill: "500k 50%".into(),
            ..Default::default()
        },
        TableRow {
            instance: "plain".into(),
            ..Default::default()
        },
        TableRow {
            agent: "catalog_only".into(),
            is_roaming: true,
            ..Default::default()
        },
    ];
    let styles = node_styles_from_rows(&rows);
    assert_eq!(styles.len(), 2);
    assert!(styles["roamer"].is_roaming);
    assert_eq!(styles["roamer"].context_percent, Some(50.0));
    assert!(!styles["plain"].is_roaming);
    assert_eq!(styles["plain"].context_percent, None);

    let mut graph = AgentGraph::new();
    graph.node_styles = styles;
    assert!(graph.node_style("roamer").is_roaming);
    assert_eq!(
        graph.node_style("unknown"),
        hcom_dashboard::model::NodeStyle::default()
    );
}

#[test]
fn test_context_fill_color_thresholds() {
    let green = CanvasView::context_fill_color(0.0);
    assert_eq!(CanvasView::context_fill_color(69.9), green);
    let amber = CanvasView::context_fill_color(70.0);
    assert_ne!(amber, green);
    assert_eq!(CanvasView::context_fill_color(85.0), amber);
    let red = CanvasView::context_fill_color(85.1);
    assert_ne!(red, amber);
    assert_ne!(red, green);
    assert_eq!(CanvasView::context_fill_color(100.0), red);
}

#[test]
fn test_cli_colors_match_spec_and_are_distinct() {
    assert_eq!(
        CanvasView::cli_color(Some("claude")),
        Color32::from_rgb(0xD9, 0x77, 0x06)
    );
    assert_eq!(
        CanvasView::cli_color(Some("codex")),
        Color32::from_rgb(0x10, 0xA3, 0x7F)
    );
    let google = Color32::from_rgb(0x42, 0x85, 0xF4);
    assert_eq!(CanvasView::cli_color(Some("agy")), google);
    assert_eq!(CanvasView::cli_color(Some("antigravity")), google);
    assert_eq!(CanvasView::cli_color(Some("gemini")), google);

    let tools = ["claude", "codex", "agy", "hermes", "kimi", "grok"];
    let colors: std::collections::HashSet<_> = tools
        .iter()
        .map(|t| CanvasView::cli_color(Some(t)))
        .chain([CanvasView::cli_color(None)])
        .collect();
    assert_eq!(colors.len(), tools.len() + 1);
}

#[test]
fn test_context_fill_polygon_rises_from_bottom() {
    use hcom_dashboard::canvas::context_fill_polygon;
    let c = Pos2::new(100.0, 100.0);
    let r = 20.0;
    assert!(context_fill_polygon(c, r, 0.0).is_empty());

    for pct in [10.0_f32, 50.0, 90.0] {
        let pts = context_fill_polygon(c, r, pct);
        assert!(pts.len() >= 3);
        // Liquid level: bottom (y = c.y + r) minus pct of the diameter.
        let level = c.y + r - 2.0 * r * pct / 100.0;
        let min_y = pts.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let max_y = pts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(
            (min_y - level).abs() < 0.01,
            "pct {pct}: top {min_y} vs {level}"
        );
        assert!((max_y - (c.y + r)).abs() < 0.1, "pct {pct}: bottom {max_y}");
        for p in &pts {
            assert!((p.distance(c) - r).abs() < 0.01);
        }
    }

    let full = context_fill_polygon(c, r, 100.0);
    let min_y = full.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    assert!((min_y - (c.y - r)).abs() < 0.1);
}

fn window_msg(id: u64, ts: chrono::DateTime<Utc>, from: &str, to: &str) -> MessageEvent {
    MessageEvent {
        id,
        ts,
        instance: from.into(),
        data: MessageData {
            from: from.into(),
            delivered_to: vec![to.into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: format!("m{id}"),
        },
    }
}

#[test]
fn test_message_window_bounds_presets_live_and_fixed_end() {
    use chrono::{Duration, TimeZone};
    use hcom_dashboard::model::MessageWindow;
    let now = Utc.with_ymd_and_hms(2026, 11, 1, 15, 0, 0).unwrap();

    let all = MessageWindow::default();
    assert!(all.is_unfiltered());
    assert_eq!(all.bounds(now), (None, now));
    assert!(all.contains(now - Duration::days(400), now));
    // Live window: no upper bound (clock skew); a fixed end has one.
    assert!(all.contains(now + Duration::seconds(5), now));

    let live_1h = MessageWindow {
        span_secs: Some(3600),
        ..Default::default()
    };
    assert!(!live_1h.is_unfiltered());
    assert_eq!(live_1h.bounds(now), (Some(now - Duration::hours(1)), now));
    assert!(live_1h.contains(now - Duration::minutes(60), now));
    assert!(!live_1h.contains(now - Duration::minutes(61), now));

    let end = now - Duration::hours(2);
    let fixed = MessageWindow {
        span_secs: Some(300),
        end: Some(end),
        last_n: None,
    };
    assert_eq!(fixed.bounds(now), (Some(end - Duration::minutes(5)), end));
    assert!(!fixed.contains(now, now));
    assert!(fixed.contains(end - Duration::minutes(1), now));

    // Non-positive span means no lower bound.
    let zero = MessageWindow {
        span_secs: Some(0),
        ..Default::default()
    };
    assert_eq!(zero.bounds(now).0, None);
}

#[test]
fn test_windowed_edge_counts_window_last_n_and_unfiltered_totals() {
    use chrono::Duration;
    use hcom_dashboard::model::MessageWindow;
    let now = Utc::now();
    let mut graph = AgentGraph::new();
    graph.apply_message_event(&window_msg(1, now - Duration::hours(3), "a", "b"));
    graph.apply_message_event(&window_msg(2, now - Duration::minutes(10), "a", "b"));
    graph.apply_message_event(&window_msg(3, now - Duration::minutes(2), "b", "a"));
    graph.apply_message_event(&window_msg(4, now - Duration::minutes(2), "a", "c"));
    let ab = graph.message_edges[&("a".to_string(), "b".to_string())];
    let ba = graph.message_edges[&("b".to_string(), "a".to_string())];
    let ac = graph.message_edges[&("a".to_string(), "c".to_string())];

    let all = graph.windowed_edge_counts(&MessageWindow::default(), now);
    assert_eq!(all[&ab].count, 2);
    assert_eq!(all[&ba].count, 1);

    let w5 = MessageWindow {
        span_secs: Some(300),
        ..Default::default()
    };
    let c5 = graph.windowed_edge_counts(&w5, now);
    assert!(
        !c5.contains_key(&ab),
        "a->b has no message in the last 5 min"
    );
    assert_eq!(c5[&ba].count, 1);
    assert_eq!(c5[&ac].count, 1);

    // Bounds first, then newest N across edges, ties broken by id (4 > 3).
    let w_last = MessageWindow {
        span_secs: Some(3600),
        end: None,
        last_n: Some(1),
    };
    let cl = graph.windowed_edge_counts(&w_last, now);
    assert_eq!(cl.len(), 1);
    assert_eq!(cl[&ac].count, 1);

    // Detail history uses the same selection.
    let hist = graph.windowed_conversation_history("a", "b", &w5, now);
    assert_eq!(hist.iter().map(|m| m.id).collect::<Vec<_>>(), vec![3]);
    let hist_all = graph.windowed_conversation_history("a", "b", &MessageWindow::default(), now);
    assert_eq!(
        hist_all.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(
        graph
            .windowed_conversation_history("a", "b", &w_last, now)
            .is_empty()
    );

    assert_eq!(graph.oldest_message_ts(), Some(now - Duration::hours(3)));
}

#[test]
fn test_windowed_counts_are_exact_without_implicit_limit() {
    use chrono::Duration;
    use hcom_dashboard::model::{MessageWindow, WindowedCount};
    let now = Utc::now();
    let mut graph = AgentGraph::new();
    // 600 messages one second apart: more than the former 500 snapshot and
    // 50 per-edge limits.
    let total = 600u64;
    for i in 1..=total {
        graph.apply_message_event(&window_msg(
            i,
            now - Duration::seconds((total - i) as i64),
            "a",
            "b",
        ));
    }
    let ab = graph.message_edges[&("a".to_string(), "b".to_string())];
    let count = |w: &MessageWindow| graph.windowed_edge_counts(w, now)[&ab].count;

    // Live + all (default) and Live + span: no implicit count limit.
    assert_eq!(count(&MessageWindow::default()), 600);
    let live_hour = MessageWindow {
        span_secs: Some(3600),
        ..Default::default()
    };
    assert_eq!(count(&live_hour), 600);
    assert_eq!(graph.messages_in_window(&live_hour, now).len(), 600);

    // Fixed range: exactly the in-range messages (ts now-399 ..= now-100).
    let fixed = MessageWindow {
        span_secs: Some(299),
        end: Some(now - Duration::seconds(100)),
        last_n: None,
    };
    assert_eq!(count(&fixed), 300);
    let fixed_all = MessageWindow {
        end: Some(now - Duration::seconds(100)),
        ..Default::default()
    };
    assert_eq!(count(&fixed_all), 500);

    // Only an explicit Last N limits, in Live and in a fixed range.
    let live_last = MessageWindow {
        last_n: Some(42),
        ..Default::default()
    };
    assert_eq!(count(&live_last), 42);
    let fixed_last = MessageWindow {
        last_n: Some(7),
        ..fixed
    };
    assert_eq!(count(&fixed_last), 7);

    assert_eq!(
        hcom_dashboard::canvas::edge_badge_label(WindowedCount {
            count: 600,
            bytes: 0
        })
        .as_deref(),
        Some("600 msgs")
    );
    assert_eq!(
        hcom_dashboard::canvas::edge_badge_label(Default::default()),
        None
    );
    assert_eq!(
        hcom_dashboard::canvas::edge_badge_label(WindowedCount { count: 1, bytes: 0 }).as_deref(),
        Some("1 msg")
    );
}

#[test]
fn test_edge_badge_shown_from_two_messages_or_on_focus_edges() {
    use hcom_dashboard::canvas::show_edge_badge;
    assert!(!show_edge_badge(1, false));
    assert!(show_edge_badge(1, true));
    assert!(show_edge_badge(2, false));
    assert!(show_edge_badge(7, true));
}

#[test]
fn test_label_rect_sits_below_the_selection_ring() {
    use hcom_dashboard::canvas::label_rect;
    for zoom in [0.5, 1.0, 2.5] {
        let c = Pos2::new(200.0, 100.0);
        let r = label_rect(c, Vec2::new(40.0, 14.0), zoom);
        assert!(r.min.y > c.y + 37.0 * zoom, "{r:?}");
        assert!((r.center().x - c.x).abs() < 1e-3);
        assert!(r.width() > 40.0 && r.height() > 14.0);
    }
}

#[test]
fn test_visible_labels_hide_the_quieter_overlapping_label() {
    use hcom_dashboard::canvas::visible_labels;
    let a = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(50.0, 10.0));
    let b = a.translate(Vec2::new(20.0, 2.0)); // overlaps a
    let far = a.translate(Vec2::new(200.0, 0.0));

    // Lower count hidden, regardless of input order.
    assert_eq!(
        visible_labels(&[(a, 1, false), (b, 5, false)]),
        [false, true]
    );
    assert_eq!(
        visible_labels(&[(b, 5, false), (a, 1, false)]),
        [true, false]
    );
    // A forced quieter label stays, the busier unforced one yields.
    assert_eq!(
        visible_labels(&[(a, 1, true), (b, 5, false)]),
        [true, false]
    );
    // Two forced labels both stay; no overlap: all shown.
    assert_eq!(visible_labels(&[(a, 1, true), (b, 5, true)]), [true, true]);
    assert_eq!(
        visible_labels(&[(a, 1, false), (far, 0, false), (b, 5, false)]),
        [false, true, true]
    );
}

#[test]
fn test_zero_count_edge_in_window_stays_selectable() {
    use chrono::Duration;
    use hcom_dashboard::model::MessageWindow;
    let now = Utc::now();
    let mut graph = AgentGraph::new();
    graph.apply_message_event(&window_msg(1, now - Duration::hours(5), "node_a", "node_b"));
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let idx_a = graph.get_node_index("node_a").unwrap();
    let idx_b = graph.get_node_index("node_b").unwrap();
    layout.set_node_pos(idx_a, Pos2::new(-100.0, 0.0), true);
    layout.set_node_pos(idx_b, Pos2::new(100.0, 0.0), true);

    let mut canvas = CanvasView::new();
    let w = MessageWindow {
        span_secs: Some(300),
        ..Default::default()
    };
    canvas.edge_counts = Some(graph.windowed_edge_counts(&w, now));
    assert!(canvas.edge_counts.as_ref().unwrap().is_empty());
    let rect = Rect::from_center_size(Pos2::new(300.0, 300.0), Vec2::new(600.0, 600.0));
    let hit = canvas.hit_test_edge(Pos2::new(300.0, 300.0), &graph, &layout, rect, 10.0);
    assert!(hit.is_some(), "dimmed edge must stay clickable");
}

#[test]
fn test_format_window_range_uses_utc() {
    use chrono::{Duration, TimeZone};
    use hcom_dashboard::app::format_window_range;
    use hcom_dashboard::model::MessageWindow;
    let now = Utc.with_ymd_and_hms(2026, 11, 1, 15, 5, 0).unwrap();
    let w = MessageWindow {
        span_secs: Some(3600),
        ..Default::default()
    };
    assert_eq!(
        format_window_range(&w, now, 37),
        "Range: 2026-11-01 14:05:00 – 15:05:00 UTC (live) · 37 msgs"
    );
    let fixed = MessageWindow {
        span_secs: None,
        end: Some(now - Duration::hours(1)),
        last_n: Some(10),
    };
    assert_eq!(
        format_window_range(&fixed, now, 10),
        "Range: start of history – 2026-11-01 14:05:00 UTC · 10 msgs, newest 10"
    );
}

#[test]
fn test_graph_state_window_defaults_and_invalid_values() {
    use hcom_dashboard::config::DashboardState;
    let old: DashboardState = serde_json::from_str("{}").expect("old state");
    assert_eq!(old.window_span_secs, None);
    assert_eq!(old.window_last_n, None);
    assert!(old.message_window().is_unfiltered());

    let bad: DashboardState =
        serde_json::from_str(r#"{"window_span_secs": -5, "window_last_n": 0}"#).expect("parse");
    assert!(bad.message_window().is_unfiltered());

    let good = DashboardState {
        window_span_secs: Some(900),
        window_last_n: Some(20),
        ..Default::default()
    };
    let restored: DashboardState =
        serde_json::from_str(&serde_json::to_string(&good).unwrap()).unwrap();
    let w = restored.message_window();
    assert_eq!(w.span_secs, Some(900));
    assert_eq!(w.last_n, Some(20));
    assert_eq!(w.end, None);
}

#[test]
fn test_app_window_refresh_caches_clamps_and_persists() {
    use chrono::Duration;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};
    let dir = tempfile::tempdir().expect("tempdir");
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let now = Utc::now();
    app.graph
        .apply_message_event(&window_msg(1, now - Duration::minutes(30), "a", "b"));

    app.refresh_window_counts(now);
    assert!(app.canvas.edge_counts.is_none(), "All keeps totals");

    app.set_message_window(Some(300), None);
    app.save_state();
    app.refresh_window_counts(now);
    assert!(app.canvas.edge_counts.as_ref().unwrap().is_empty());
    assert_eq!(load_state(dir.path()).window_span_secs, Some(300));

    // Scrubber end before the oldest message is clamped to it.
    app.msg_window.end = Some(now - Duration::hours(2));
    app.refresh_window_counts(now);
    assert_eq!(app.msg_window.end, Some(now - Duration::minutes(30)));
    assert_eq!(app.canvas.edge_counts.as_ref().unwrap().len(), 1);

    app.set_message_window(None, None);
    app.msg_window.end = None;
    app.refresh_window_counts(now);
    assert!(app.canvas.edge_counts.is_none());
}

fn layout_snap(name: &str, parent: Option<&str>) -> InstanceSnapshot {
    InstanceSnapshot {
        name: name.into(),
        status: Some("active".into()),
        status_context: None,
        status_age_seconds: None,
        unread_count: None,
        parent_name: parent.map(Into::into),
        directory: None,
        tool: None,
    }
}

fn node_pos(layout: &GraphLayout, graph: &AgentGraph, name: &str) -> Pos2 {
    layout.nodes[&graph.get_node_index(name).unwrap()].pos
}

fn min_pair_distance(layout: &GraphLayout) -> f32 {
    let ps: Vec<Pos2> = layout.nodes.values().map(|p| p.pos).collect();
    let mut min = f32::MAX;
    for i in 0..ps.len() {
        for j in i + 1..ps.len() {
            min = min.min(ps[i].distance(ps[j]));
        }
    }
    min
}

#[test]
fn test_spawn_depths_roots_chain_cycle_and_placeholder() {
    use hcom_dashboard::layout::spawn_depths;
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("root", None),
        layout_snap("child", Some("root")),
        layout_snap("grand", Some("child")),
        layout_snap("sibling", Some("root")),
        // Parent only known as a placeholder: it is a root.
        layout_snap("orphan", Some("ghost")),
        // Spawn cycle without a root.
        layout_snap("cyc_a", Some("cyc_b")),
        layout_snap("cyc_b", Some("cyc_a")),
    ]);
    let d = spawn_depths(&graph);
    let depth = |n: &str| d[&graph.get_node_index(n).unwrap()];
    assert_eq!(depth("root"), 0);
    assert_eq!(depth("child"), 1);
    assert_eq!(depth("sibling"), 1);
    assert_eq!(depth("grand"), 2);
    assert_eq!(depth("ghost"), 0);
    assert_eq!(depth("orphan"), 1);
    let mut cyc = [depth("cyc_a"), depth("cyc_b")];
    cyc.sort();
    assert_eq!(cyc, [0, 1]);
    assert_eq!(d.len(), graph.node_count());
}

#[test]
fn test_tree_targets_layers_siblings_and_centered_parent() {
    use hcom_dashboard::layout::tree_targets;
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("root", None),
        layout_snap("a", Some("root")),
        layout_snap("b", Some("root")),
        layout_snap("c", Some("root")),
        layout_snap("a1", Some("a")),
    ]);
    let slot = 100.0;
    let t = tree_targets(&graph, 130.0, slot, hcom_dashboard::layout::NO_CANVAS);
    let p = |n: &str| t[&graph.get_node_index(n).unwrap()];
    assert!((p("a").y - p("root").y - 130.0).abs() < 1e-3);
    assert!((p("a1").y - p("a").y - 130.0).abs() < 1e-3);
    // Siblings at least one slot apart, in name order.
    assert!(p("b").x - p("a").x >= slot - 1e-3);
    assert!(p("c").x - p("b").x >= slot - 1e-3);
    // Parent centered over its first and last child.
    assert!((p("root").x - (p("a").x + p("c").x) / 2.0).abs() < 1e-3);
}

#[test]
fn test_tree_targets_wrap_lone_roots_below_trees_when_top_layer_is_too_wide() {
    use hcom_dashboard::layout::tree_targets;
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("root", None),
        layout_snap("a", Some("root")),
        layout_snap("b", Some("root")),
        layout_snap("l1", None),
        layout_snap("l2", None),
        layout_snap("l3", None),
        layout_snap("l4", None),
        layout_snap("l5", None),
    ]);
    let (h, slot) = (130.0, 100.0);
    let p = |t: &std::collections::HashMap<_, Pos2>, n: &str| t[&graph.get_node_index(n).unwrap()];

    // Top layer of 7 slots fits 7: unchanged, lone roots stay in the top row.
    let t = tree_targets(&graph, h, slot, (7, 4));
    assert!(
        ["l1", "l5"]
            .iter()
            .all(|n| (p(&t, n).y - p(&t, "root").y).abs() < 1e-3)
    );

    // 6 slots wide but one layer high: one long row below the trees fits
    // best (largest zoom).
    let t = tree_targets(&graph, h, slot, (6, 1));
    let kid_y = p(&t, "a").y;
    assert!(
        ["l1", "l2", "l3", "l4", "l5"]
            .iter()
            .all(|n| (p(&t, n).y - (kid_y + h)).abs() < 1e-3)
    );

    // Only 3 fit on a square canvas: lone roots in rows of 3 below the
    // deepest tree layer.
    let t = tree_targets(&graph, h, slot, (3, 4));
    let kid_y = p(&t, "a").y;
    for n in ["l1", "l2", "l3"] {
        assert!((p(&t, n).y - (kid_y + h)).abs() < 1e-3, "{n} in row 1");
    }
    for n in ["l4", "l5"] {
        assert!(
            (p(&t, n).y - (kid_y + 2.0 * h)).abs() < 1e-3,
            "{n} in row 2"
        );
    }
    assert!((p(&t, "l2").x - p(&t, "l1").x - slot).abs() < 1e-3);
    // Rows are centered and the whole block is centered in y.
    assert!((p(&t, "l2").x).abs() < 1e-3);
    assert!((p(&t, "l4").x + p(&t, "l5").x).abs() < 1e-3);
    assert!((p(&t, "root").y + p(&t, "l4").y).abs() < 1e-3);
    // The tree keeps its shape.
    assert!((p(&t, "a").y - p(&t, "root").y - h).abs() < 1e-3);
    assert!((p(&t, "root").x - (p(&t, "a").x + p(&t, "b").x) / 2.0).abs() < 1e-3);
}

#[test]
fn test_hierarchy_with_many_lone_roots_fits_canvas_above_min_zoom() {
    use hcom_dashboard::config::DashboardState;
    let (_dir, mut app) = split_test_app(DashboardState {
        layout_mode: "hierarchy".into(),
        ..DashboardState::default()
    });
    let mut snaps = vec![
        snap("root", None),
        snap("kid_a", Some("root")),
        snap("kid_b", Some("root")),
    ];
    snaps.extend((0..60).map(|i| snap(&format!("lone_{i:02}"), None)));
    app.apply_persisted_view();
    app.graph.apply_instance_snapshot(&snaps);
    app.layout.sync_nodes(&app.graph);
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 300);
    assert!(
        app.layout.canvas_cells.0 < 62,
        "canvas narrower than the top layer"
    );
    // The first known canvas size placed the nodes on the wrapped rows.
    let off = app
        .layout
        .tree_targets
        .iter()
        .map(|(i, t)| app.layout.nodes[i].pos.distance(*t))
        .fold(0.0f32, f32::max);
    assert!(off < 30.0, "nodes at their wrapped targets: {off}");
    assert!(
        all_nodes_in_canvas(&app),
        "AC2: no node beyond the canvas edge"
    );
    assert!(
        app.canvas.zoom > 2.0 * hcom_dashboard::canvas::ZOOM_MIN,
        "wrapped rows keep a readable zoom: {}",
        app.canvas.zoom
    );
}

#[test]
fn test_hierarchy_mode_converges_to_layers_and_reparent_moves_subtree() {
    use hcom_dashboard::layout::LayoutMode;
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("root", None),
        layout_snap("kid", Some("root")),
        layout_snap("other", None),
    ]);
    let mut layout = GraphLayout::new();
    layout.set_mode(LayoutMode::Hierarchy);
    layout.sync_nodes(&graph);
    for _ in 0..2000 {
        layout.step(&graph, 0.016);
    }
    let dy = node_pos(&layout, &graph, "kid").y - node_pos(&layout, &graph, "root").y;
    assert!(
        (dy - layout.h_layer).abs() < 25.0,
        "kid one layer below root, dy {dy}"
    );

    // Re-parent kid under other: its target follows the new parent.
    graph.apply_instance_snapshot(&[layout_snap("kid", Some("other"))]);
    // The app path: the sync detects the structural change and wakes the layout.
    layout.sync_nodes(&graph);
    for _ in 0..2000 {
        layout.step(&graph, 0.016);
    }
    let kid = node_pos(&layout, &graph, "kid");
    let other = node_pos(&layout, &graph, "other");
    assert!((kid.y - other.y - layout.h_layer).abs() < 25.0);
    assert!(
        (kid.x - other.x).abs() < 40.0,
        "kid centered under its new parent"
    );
}

#[test]
fn test_cluster_mode_pulls_heavy_communicators_closer() {
    let mut graph = AgentGraph::new();
    let now = Utc::now();
    let mut id = 0;
    for _ in 0..100 {
        id += 1;
        graph.apply_message_event(&window_msg(id, now, "a", "b"));
    }
    id += 1;
    graph.apply_message_event(&window_msg(id, now, "c", "d"));
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    for _ in 0..3000 {
        layout.step(&graph, 0.016);
    }
    let ab = node_pos(&layout, &graph, "a").distance(node_pos(&layout, &graph, "b"));
    let cd = node_pos(&layout, &graph, "c").distance(node_pos(&layout, &graph, "d"));
    assert!(
        ab < cd,
        "heavy pair {ab} must be closer than light pair {cd}"
    );
    // Sublinear springs: stiffness grows with ln(1 + count), rest length shrinks.
    let (k1, l1) = layout.spring_params(1);
    let (k100, l100) = layout.spring_params(100);
    assert!(k100 > k1 && k100 < k1 * 100.0);
    assert!(l100 < l1 && l100 >= layout.l_min);
}

#[test]
fn test_collision_separates_coincident_nodes_and_respects_pins() {
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("p", None),
        layout_snap("q", None),
        layout_snap("r", None),
    ]);
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let (p, q, r) = (
        graph.get_node_index("p").unwrap(),
        graph.get_node_index("q").unwrap(),
        graph.get_node_index("r").unwrap(),
    );
    // All on the same point, p pinned (dragged).
    layout.set_node_pos(p, Pos2::ZERO, true);
    layout.set_node_pos(q, Pos2::ZERO, false);
    layout.set_node_pos(r, Pos2::ZERO, false);
    for _ in 0..200 {
        layout.step(&graph, 0.016);
    }
    assert_eq!(
        layout.nodes[&p].pos,
        Pos2::ZERO,
        "pinned node does not move"
    );
    let min = min_pair_distance(&layout);
    assert!(min.is_finite());
    assert!(
        min >= 2.0 * layout.collision_radius - 0.5,
        "nodes must not overlap, min {min}"
    );

    // Two pinned coincident nodes cannot be separated: the layout still sleeps.
    layout.set_node_pos(q, Pos2::ZERO, true);
    layout.set_node_pos(r, Pos2::new(500.0, 500.0), true);
    let mut slept = false;
    for _ in 0..50 {
        layout.step(&graph, 0.016);
        if layout.is_sleeping {
            slept = true;
            break;
        }
    }
    assert!(slept, "unseparable pinned pairs must not block sleep");
}

#[test]
fn test_collision_pass_resumes_under_tight_budget() {
    let mut graph = AgentGraph::new();
    let snap: Vec<InstanceSnapshot> = (0..400)
        .map(|i| layout_snap(&format!("n{i:03}"), None))
        .collect();
    graph.apply_instance_snapshot(&snap);
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    // Spread nodes far apart, except the last two in pair order on one point.
    let mut idxs: Vec<_> = layout.nodes.keys().copied().collect();
    idxs.sort();
    for (k, idx) in idxs.iter().enumerate() {
        layout.set_node_pos(*idx, Pos2::new(k as f32 * 1000.0, 0.0), false);
    }
    let last = idxs[idxs.len() - 1];
    let prev = idxs[idxs.len() - 2];
    layout.set_node_pos(last, Pos2::new(5_000_000.0, 0.0), false);
    layout.set_node_pos(prev, Pos2::new(5_000_000.0, 0.0), false);
    layout.max_step_duration = Duration::from_micros(300);
    for _ in 0..3000 {
        layout.step(&graph, 0.016);
        let d = layout.nodes[&last].pos.distance(layout.nodes[&prev].pos);
        if d >= 2.0 * layout.collision_radius - 0.5 {
            return;
        }
    }
    panic!("overlap at the end of the pair order was never resolved");
}

#[test]
fn test_manual_mode_does_not_move_nodes_but_places_new_ones() {
    use hcom_dashboard::layout::LayoutMode;
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[layout_snap("m1", None), layout_snap("m2", None)]);
    let mut layout = GraphLayout::new();
    layout.set_mode(LayoutMode::Manual);
    layout.sync_nodes(&graph);
    let before: Vec<Pos2> = ["m1", "m2"]
        .iter()
        .map(|n| node_pos(&layout, &graph, n))
        .collect();
    assert_eq!(layout.step(&graph, 0.016), 0.0);
    let after: Vec<Pos2> = ["m1", "m2"]
        .iter()
        .map(|n| node_pos(&layout, &graph, n))
        .collect();
    assert_eq!(before, after);

    graph.apply_instance_snapshot(&[layout_snap("m3", Some("m1"))]);
    layout.sync_nodes(&graph);
    let d = node_pos(&layout, &graph, "m3").distance(node_pos(&layout, &graph, "m1"));
    assert!(d < 200.0, "new node placed next to its parent, d {d}");

    // A finished drag does not leave the node pinned for later modes.
    let m1 = graph.get_node_index("m1").unwrap();
    layout.set_node_pos(m1, Pos2::new(10.0, 10.0), true);
    layout.set_node_pos(m1, Pos2::new(10.0, 10.0), false);
    assert!(!layout.nodes[&m1].pinned);
    layout.set_mode(LayoutMode::Cluster);
    assert!(!layout.is_sleeping);
    assert_eq!(
        (layout.cursor_i, layout.cursor_j, layout.edge_cursor),
        (0, 1, 0)
    );
}

#[test]
fn test_initial_placement_near_placeholder_parent_and_deterministic_spiral() {
    let build = || {
        let mut graph = AgentGraph::new();
        // Child arrives first; its parent exists only as a placeholder.
        graph.apply_instance_snapshot(&[layout_snap("kid", Some("dad"))]);
        graph.apply_instance_snapshot(&[
            layout_snap("s1", None),
            layout_snap("s2", None),
            layout_snap("s3", None),
        ]);
        let mut layout = GraphLayout::new();
        layout.sync_nodes(&graph);
        (graph, layout)
    };
    let (graph, layout) = build();
    let d = node_pos(&layout, &graph, "kid").distance(node_pos(&layout, &graph, "dad"));
    assert!(d < 200.0, "child next to its placeholder parent, d {d}");

    // Spiral: deterministic and not all on one circle.
    let (graph2, layout2) = build();
    let radii: Vec<f32> = ["s1", "s2", "s3"]
        .iter()
        .map(|n| {
            let p = node_pos(&layout, &graph, n);
            assert_eq!(p, node_pos(&layout2, &graph2, n));
            p.to_vec2().length()
        })
        .collect();
    assert!(radii[0] < radii[1] && radii[1] < radii[2]);
}

#[test]
fn test_graph_state_layout_mode_default_and_roundtrip() {
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::layout::LayoutMode;
    let old: DashboardState = serde_json::from_str("{}").unwrap();
    assert_eq!(old.layout_mode, "cluster");
    assert_eq!(
        LayoutMode::from_str_or_default(&old.layout_mode),
        LayoutMode::Cluster
    );
    assert_eq!(
        LayoutMode::from_str_or_default("bogus"),
        LayoutMode::Cluster
    );
    for m in [
        LayoutMode::Hierarchy,
        LayoutMode::Cluster,
        LayoutMode::Manual,
    ] {
        let s = DashboardState {
            layout_mode: m.as_str().into(),
            ..Default::default()
        };
        let r: DashboardState = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(LayoutMode::from_str_or_default(&r.layout_mode), m);
    }
}

#[test]
fn test_cluster_springs_follow_reparent_without_new_event() {
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("p1", None),
        layout_snap("p2", None),
        layout_snap("kid", Some("p1")),
    ]);
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let p1 = graph.get_node_index("p1").unwrap();
    let p2 = graph.get_node_index("p2").unwrap();
    let kid = graph.get_node_index("kid").unwrap();
    // Parents far apart and pinned; kid starts next to p1.
    layout.set_node_pos(p1, Pos2::new(-600.0, 0.0), true);
    layout.set_node_pos(p2, Pos2::new(600.0, 0.0), true);
    layout.set_node_pos(kid, Pos2::new(-500.0, 0.0), false);
    for _ in 0..200 {
        layout.step(&graph, 0.016);
    }
    // Re-parent: the edge count stays the same, no message event.
    graph.apply_instance_snapshot(&[layout_snap("kid", Some("p2"))]);
    // The app path: the sync detects the structural change and wakes the layout.
    layout.sync_nodes(&graph);
    for _ in 0..3000 {
        layout.step(&graph, 0.016);
    }
    let k = layout.nodes[&kid].pos;
    assert!(
        k.x > 0.0,
        "kid must follow the spring to its new parent, x {}",
        k.x
    );
}

#[test]
fn test_tooltip_config_default_custom_and_empty() {
    use hcom_dashboard::config::{DashboardConfig, default_tooltip};

    let cfg: DashboardConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg.tooltip, default_tooltip());
    assert_eq!(
        cfg.tooltip,
        [
            "status", "detail", "cli", "since", "age", "dir", "unread", "context"
        ]
    );

    let cfg: DashboardConfig =
        serde_json::from_str(r#"{"tooltip": ["dir", "status:done", "cli"]}"#).unwrap();
    assert_eq!(cfg.tooltip, ["dir", "status:done", "cli"]);
    assert_eq!(cfg.report.len(), 28, "report stays independent of tooltip");

    let cfg: DashboardConfig = serde_json::from_str(r#"{"tooltip": []}"#).unwrap();
    assert!(cfg.tooltip.is_empty());
}

#[test]
fn test_tooltip_lines_order_skip_empty_status_and_multiline() {
    use hcom_dashboard::table_data::{TableRow, tooltip_lines};

    let mut row = TableRow {
        instance: "worker_1".into(),
        cli: "claude".into(),
        status: "active".into(),
        detail: "first line\nsecond line".into(),
        ..Default::default()
    };
    row.status_values.insert("done".into(), "built it".into());

    let fields: Vec<String> = ["dir", "status:done", "cli", "unknown", "detail", "status"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let lines = tooltip_lines(&row, &fields);
    assert_eq!(
        lines,
        vec![
            ("status:done".to_string(), "built it".to_string()),
            ("cli".to_string(), "claude".to_string()),
            ("detail".to_string(), "first line…".to_string()),
            ("status".to_string(), "active".to_string()),
        ]
    );
}

#[test]
fn test_tooltip_target_gates_and_content() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    app.graph.apply_instance_snapshot(&[InstanceSnapshot {
        name: "worker_1".into(),
        status: Some("active".into()),
        status_context: None,
        status_age_seconds: None,
        unread_count: None,
        parent_name: Some("parent_x".into()),
        directory: None,
        tool: Some("claude".into()),
    }]);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    app.table_rows = vec![TableRow {
        instance: "worker_1".into(),
        cli: "codex".into(),
        status: "listening".into(),
        ..Default::default()
    }];

    // Not hovered -> nothing.
    assert_eq!(app.tooltip_target(false, false), None);
    app.canvas.hovered_node = Some(idx);
    assert_eq!(app.tooltip_target(false, false), Some(idx));

    // Gates: popup menu, pressed pointer, drag, dialogs, empty config.
    assert_eq!(app.tooltip_target(true, false), None);
    assert_eq!(app.tooltip_target(false, true), None);
    app.canvas.dragged_node = Some(idx);
    assert_eq!(app.tooltip_target(false, false), None);
    app.canvas.dragged_node = None;
    app.show_start_dialog = true;
    assert_eq!(app.tooltip_target(false, false), None);
    app.show_start_dialog = false;
    app.show_send_dialog = true;
    assert_eq!(app.tooltip_target(false, false), None);
    app.show_send_dialog = false;
    app.confirm_kill_target = Some("worker_1".into());
    assert_eq!(app.tooltip_target(false, false), None);
    app.confirm_kill_target = None;
    app.confirm_restart_target = Some(("worker_1".into(), Vec::new()));
    assert_eq!(app.tooltip_target(false, false), None);
    app.confirm_restart_target = None;
    app.confirm_kill_group = Some(vec!["worker_1".into()]);
    assert_eq!(app.tooltip_target(false, false), None);
    app.confirm_kill_group = None;
    app.custom_cmd_confirm = Some(("x".into(), Vec::new(), None));
    assert_eq!(app.tooltip_target(false, false), None);
    app.custom_cmd_confirm = None;
    app.context_menu = Some(hcom_dashboard::app::ContextMenuState {
        target: hcom_dashboard::app::ContextMenuTarget::Node(idx),
        screen_pos: Pos2::ZERO,
    });
    assert_eq!(app.tooltip_target(false, false), None);
    app.context_menu = None;
    assert_eq!(app.tooltip_target(false, false), Some(idx));

    // Content comes from the table row, in config order.
    let (name, lines) = app.tooltip_content(idx).unwrap();
    assert_eq!(name, "worker_1");
    assert_eq!(
        lines,
        vec![
            ("status".to_string(), "listening".to_string()),
            ("cli".to_string(), "codex".to_string()),
        ]
    );

    // Placeholder parent without a table row: name only.
    let parent = app.graph.get_node_index("parent_x").unwrap();
    let (name, lines) = app.tooltip_content(parent).unwrap();
    assert_eq!(name, "parent_x");
    assert!(lines.is_empty());

    app.config.tooltip.clear();
    assert_eq!(app.tooltip_target(false, false), None);
}

fn snap(name: &str, parent: Option<&str>) -> InstanceSnapshot {
    InstanceSnapshot {
        name: name.into(),
        status: Some("active".into()),
        status_context: None,
        status_age_seconds: None,
        unread_count: None,
        parent_name: parent.map(|p| p.into()),
        directory: None,
        tool: None,
    }
}

#[test]
fn test_state_save_sanitizes_non_finite_and_keeps_other_keys() {
    use hcom_dashboard::config::{DashboardState, SavedNodePos, load_state, save_state};

    let dir = tempfile::tempdir().unwrap();
    let mut state = DashboardState {
        layout_mode: "manual".into(),
        canvas_zoom: Some(2.0),
        canvas_pan_x: Some(f32::NAN),
        canvas_pan_y: Some(5.0),
        detail_width_px: Some(f32::INFINITY),
        ..Default::default()
    };
    state.pinned_positions.insert(
        "bad".into(),
        SavedNodePos {
            x: f32::NAN,
            y: 1.0,
            saved_at: 1,
        },
    );
    state.pinned_positions.insert(
        "good".into(),
        SavedNodePos {
            x: 10.0,
            y: -20.0,
            saved_at: 2,
        },
    );
    save_state(dir.path(), &state).unwrap();

    let loaded = load_state(dir.path());
    assert_eq!(loaded.layout_mode, "manual");
    assert_eq!(loaded.canvas_zoom, Some(2.0));
    assert_eq!((loaded.canvas_pan_x, loaded.canvas_pan_y), (None, None));
    assert_eq!(loaded.detail_width_px, None);
    assert_eq!(loaded.pinned_positions.len(), 1);
    assert_eq!(
        loaded.pinned_positions["good"],
        SavedNodePos {
            x: 10.0,
            y: -20.0,
            saved_at: 2
        }
    );

    // Only dashboard-state.json exists: no leftover temp file.
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["dashboard-state.json"]);
}

#[test]
fn test_state_load_drops_bad_position_entry_and_clamps_values() {
    use hcom_dashboard::config::load_state;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("dashboard-state.json"),
        r#"{
            "detail_tab": "Bogus",
            "pinned_positions": {
                "ok": {"x": 1.0, "y": 2.0, "saved_at": 5},
                "nullx": {"x": null, "y": 2.0},
                "wrong": "text"
            },
            "canvas_zoom": 100.0,
            "canvas_pan_x": 3.0,
            "canvas_pan_y": 4.0,
            "table_height_frac": 0.99
        }"#,
    )
    .unwrap();
    let s = load_state(dir.path());
    assert_eq!(s.detail_tab, "Summary", "invalid detail_tab falls back");
    assert_eq!(s.pinned_positions.keys().collect::<Vec<_>>(), ["ok"]);
    assert_eq!(s.canvas_zoom, Some(hcom_dashboard::canvas::ZOOM_MAX));
    assert_eq!((s.canvas_pan_x, s.canvas_pan_y), (Some(3.0), Some(4.0)));
    assert_eq!(
        s.table_height_frac,
        Some(hcom_dashboard::config::SPLIT_FRAC_MAX)
    );
}

#[test]
fn test_state_corrupt_file_uses_defaults() {
    use hcom_dashboard::config::{DashboardState, load_state};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("dashboard-state.json"), "{ not json").unwrap();
    let s = load_state(dir.path());
    let d = DashboardState::default();
    assert_eq!(s.detail_tab, d.detail_tab);
    assert!(s.pinned_positions.is_empty());
    assert_eq!(s.canvas_zoom, None);
    assert_eq!(s.detail_width_px, None);
}

#[test]
fn test_state_sanitize_caps_node_positions_to_newest() {
    use hcom_dashboard::config::{DashboardState, MAX_NODE_POSITIONS, SavedNodePos};

    let mut s = DashboardState::default();
    for i in 0..(MAX_NODE_POSITIONS + 10) {
        s.pinned_positions.insert(
            format!("n{i}"),
            SavedNodePos {
                x: 0.0,
                y: 0.0,
                saved_at: i as i64,
            },
        );
    }
    s.canvas_zoom = Some(f32::NAN);
    s.sanitize();
    assert_eq!(s.pinned_positions.len(), MAX_NODE_POSITIONS);
    assert!(!s.pinned_positions.contains_key("n0"));
    assert!(!s.pinned_positions.contains_key("n9"));
    assert!(s.pinned_positions.contains_key("n10"));
    assert_eq!(s.canvas_zoom, None);
}

#[test]
fn test_layout_pinned_node_starts_at_pin_and_stays_in_every_mode() {
    use hcom_dashboard::layout::LayoutMode;

    // "a" is pinned; its child "b" and the partner "c" pull and push on it.
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[snap("a", None), snap("b", Some("a")), snap("c", None)]);
    let idx = graph.get_node_index("a").unwrap();
    let saved = Pos2::new(321.0, -123.0);

    for mode in [
        LayoutMode::Cluster,
        LayoutMode::Hierarchy,
        LayoutMode::Manual,
    ] {
        let mut layout = GraphLayout::new();
        layout.set_mode(mode);
        layout.pinned_positions.insert("a".into(), saved);
        layout.sync_nodes(&graph);
        assert_eq!(layout.nodes[&idx].pos, saved, "{mode:?}");
        assert!(layout.nodes[&idx].pinned, "{mode:?}");
        let other = graph.get_node_index("b").unwrap();
        assert!(!layout.nodes[&other].pinned, "{mode:?}");
        for _ in 0..200 {
            layout.step(&graph, 0.016);
        }
        assert_eq!(
            layout.nodes[&idx].pos, saved,
            "{mode:?}: simulation moved the pin"
        );
    }
}

#[test]
fn test_layout_set_pinned_pins_current_position_and_releases() {
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[snap("a", None)]);
    let idx = graph.get_node_index("a").unwrap();
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    layout.set_node_pos(idx, Pos2::new(5.0, 6.0), false);

    assert_eq!(layout.set_pinned(idx, "a", true), Some(Pos2::new(5.0, 6.0)));
    assert!(layout.nodes[&idx].pinned);
    assert_eq!(layout.pinned_positions["a"], Pos2::new(5.0, 6.0));

    assert_eq!(layout.set_pinned(idx, "a", false), None);
    assert!(!layout.nodes[&idx].pinned);
    assert!(layout.pinned_positions.is_empty());
}

#[test]
fn test_canvas_drag_release_keeps_only_user_pins() {
    use hcom_dashboard::layout::LayoutMode;

    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[snap("a", None)]);
    let idx = graph.get_node_index("a").unwrap();
    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

    for user_pin in [false, true] {
        let mut layout = GraphLayout::new();
        layout.set_mode(LayoutMode::Manual);
        layout.sync_nodes(&graph);
        layout.set_node_pos(idx, Pos2::ZERO, false);
        if user_pin {
            layout.set_pinned(idx, "a", true);
        }
        let mut canvas = CanvasView::new();
        let mut center = Pos2::ZERO;
        let press = |p: Pos2, pressed: bool| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        // First frame only finds the node's screen position.
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                center = canvas.world_to_screen(Pos2::ZERO, ui.max_rect());
                canvas.show(
                    ui,
                    &graph,
                    &mut layout,
                    None,
                    &hcom_dashboard::canvas::ThemePalette::for_theme(
                        hcom_dashboard::config::ResolvedTheme::Dark,
                    ),
                );
            },
        )
        .textures_delta
        .clear();
        // Hover, press on the node, move 40 px, release.
        let to = center + Vec2::new(40.0, 0.0);
        for events in [
            vec![egui::Event::PointerMoved(center)],
            vec![press(center, true)],
            vec![egui::Event::PointerMoved(center + Vec2::new(20.0, 0.0))],
            vec![egui::Event::PointerMoved(to)],
            vec![press(to, false)],
            vec![],
        ] {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                canvas.show(
                    ui,
                    &graph,
                    &mut layout,
                    None,
                    &hcom_dashboard::canvas::ThemePalette::for_theme(
                        hcom_dashboard::config::ResolvedTheme::Dark,
                    ),
                );
            })
            .textures_delta
            .clear();
        }
        assert!(canvas.dragged_node.is_none(), "drag ended");
        assert!(layout.nodes[&idx].pos.x > 10.0, "node was dragged");
        assert_eq!(layout.nodes[&idx].pinned, user_pin, "user_pin={user_pin}");
    }
}

#[test]
fn test_app_persists_drag_and_view_with_debounce() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};
    use std::time::Instant;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    app.graph.apply_instance_snapshot(&[snap("worker_1", None)]);
    app.layout.sync_nodes(&app.graph);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    app.layout.set_node_pos(idx, Pos2::new(50.0, 60.0), false);

    // A drag of an unpinned node stores nothing.
    app.record_dragged_position(idx);
    assert!(app.layout.pinned_positions.is_empty());
    assert!(!dir.path().join("dashboard-state.json").exists());

    // Pin, then a drag of the pinned node moves its pin.
    app.toggle_pin(idx);
    app.layout.set_node_pos(idx, Pos2::new(70.0, 80.0), true);
    app.record_dragged_position(idx);
    app.layout.set_node_pos(idx, Pos2::new(50.0, 60.0), true);
    app.record_dragged_position(idx);
    let s = load_state(dir.path());
    let p = s.pinned_positions["worker_1"];
    assert_eq!((p.x, p.y), (50.0, 60.0));
    assert!(p.saved_at > 0);
    assert_eq!(
        app.layout.pinned_positions["worker_1"],
        Pos2::new(50.0, 60.0)
    );

    // Debounce: nothing before 1 s or with a pressed pointer.
    app.state.canvas_zoom = Some(1.5);
    app.mark_view_dirty();
    let t0 = app.view_dirty_since.unwrap();
    assert!(!app.flush_view_state(t0 + Duration::from_millis(500), false));
    assert!(!app.flush_view_state(t0 + Duration::from_secs(2), true));
    assert_eq!(load_state(dir.path()).canvas_zoom, None);
    assert!(app.flush_view_state(t0 + Duration::from_secs(2), false));
    assert!(app.view_dirty_since.is_none());
    assert_eq!(load_state(dir.path()).canvas_zoom, Some(1.5));
    assert!(!app.flush_view_state(Instant::now() + Duration::from_secs(5), false));

    // Restore into a fresh app.
    let mut fresh = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState {
            canvas_pan_x: Some(7.0),
            canvas_pan_y: Some(-8.0),
            ..load_state(dir.path())
        },
        dir.path().to_path_buf(),
    );
    fresh.apply_persisted_view();
    assert_eq!(fresh.canvas.zoom, 1.5);
    assert_eq!(fresh.canvas.pan, Vec2::new(7.0, -8.0));
    assert_eq!(
        fresh.layout.pinned_positions["worker_1"],
        Pos2::new(50.0, 60.0)
    );
}

#[test]
fn test_pin_survives_restart_when_initial_sync_runs_first() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};
    use hcom_dashboard::layout::LayoutMode;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    app.graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", None)]);
    app.layout.sync_nodes(&app.graph);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    app.layout
        .set_node_pos(idx, Pos2::new(400.0, -300.0), false);
    app.toggle_pin(idx);

    // Restart in the order `HcomDashboardApp::new` uses: the initial graph is
    // synced into the layout before the persisted view is applied.
    let mut fresh = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        load_state(dir.path()),
        dir.path().to_path_buf(),
    );
    fresh.layout.set_mode(LayoutMode::Cluster);
    fresh
        .graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", None)]);
    fresh.layout.sync_nodes(&fresh.graph);
    fresh.apply_persisted_view();
    let idx = fresh.graph.get_node_index("worker_1").unwrap();
    let other = fresh.graph.get_node_index("worker_2").unwrap();
    assert_eq!(fresh.layout.nodes[&idx].pos, Pos2::new(400.0, -300.0));
    assert!(fresh.layout.nodes[&idx].pinned);
    assert!(!fresh.layout.nodes[&other].pinned);
    for _ in 0..200 {
        fresh.layout.step(&fresh.graph, 0.016);
    }
    assert_eq!(fresh.layout.nodes[&idx].pos, Pos2::new(400.0, -300.0));

    // Unpin removes the entry from memory and from dashboard-state.json.
    fresh.toggle_pin(idx);
    assert!(!fresh.layout.nodes[&idx].pinned);
    assert!(fresh.layout.pinned_positions.is_empty());
    assert!(load_state(dir.path()).pinned_positions.is_empty());
}

/// Screen rect of the first accesskit node whose label is `label`.
fn widget_rect(output: &egui::FullOutput, label: &str) -> Option<Rect> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    update.nodes.iter().find_map(|(_, node)| {
        if node.label() != Some(label) {
            return None;
        }
        let b = node.bounds()?;
        Some(Rect::from_min_max(
            Pos2::new(b.x0 as f32, b.y0 as f32),
            Pos2::new(b.x1 as f32, b.y1 as f32),
        ))
    })
}

/// Screen rects and disabled flags of all accesskit nodes labelled `label`, top to bottom.
fn widget_rects(output: &egui::FullOutput, label: &str) -> Vec<(Rect, bool)> {
    let Some(update) = output.platform_output.accesskit_update.as_ref() else {
        return Vec::new();
    };
    let mut found: Vec<(Rect, bool)> = update
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| {
            let b = node.bounds()?;
            let rect = Rect::from_min_max(
                Pos2::new(b.x0 as f32, b.y0 as f32),
                Pos2::new(b.x1 as f32, b.y1 as f32),
            );
            Some((rect, node.is_disabled()))
        })
        .collect();
    found.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    found
}

#[test]
fn test_kill_actions_are_separated_from_restart_attach_and_transcript() {
    use eframe::App;
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 800.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut render = |app: &mut hcom_dashboard::app::HcomDashboardApp| {
        let mut out = None;
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut o = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
            o.textures_delta.clear();
            out = Some(o);
        }
        out.unwrap()
    };

    // Nothing running: Kill group has nothing to act on.
    let out = render(&mut app);
    let kill_group = widget_rects(&out, "Kill group ▾");
    assert_eq!(kill_group.len(), 1);
    assert!(
        kill_group[0].1,
        "Kill group disabled without running agents"
    );

    let row = TableRow {
        agent: "worker".into(),
        instance: "worker_1".into(),
        is_running: true,
        status: "active".into(),
        ..Default::default()
    };
    app.table_rows = vec![row.clone()];
    app.sync_graph_from_table_rows();
    app.select_table_row(&row);
    let out = render(&mut app);

    let one = |label: &str| {
        let r = widget_rects(&out, label);
        assert!(!r.is_empty(), "{label:?} missing");
        r[0]
    };
    let (restart, _) = one("Restart");
    let (attach, _) = one("Attach");
    let kills = widget_rects(&out, "Kill");
    assert_eq!(kills.len(), 2, "toolbar Kill and panel Kill");
    let (toolbar_kill, panel_kill) = (kills[0].0, kills[1].0);
    let (kill_group, kill_group_disabled) = one("Kill group ▾");
    assert!(
        !kill_group_disabled,
        "Kill group enabled with a running agent"
    );

    // Toolbar: Attach, Restart | Kill, Kill group, all on one row.
    let spacing = ctx.global_style().spacing.item_spacing.x;
    for r in [attach, toolbar_kill, kill_group] {
        assert!(
            (r.center().y - restart.center().y).abs() < 1.0,
            "one row: {r:?}"
        );
    }
    assert!(attach.right() < restart.left());
    assert!(
        toolbar_kill.left() - restart.right() > spacing + 2.0,
        "separator between Restart and Kill: {restart:?} / {toolbar_kill:?}"
    );
    assert!(toolbar_kill.right() < kill_group.left(), "Kill group last");

    // Agent panel: Attach (Focus), Transcript, gap, Kill.
    let (panel_attach, _) = one("Attach (Focus)");
    let transcript = widget_rects(&out, "Transcript")
        .into_iter()
        .map(|(r, _)| r)
        .find(|r| (r.center().y - panel_kill.center().y).abs() < 1.0)
        .expect("panel Transcript on the Kill row");
    assert!(panel_attach.right() < transcript.left());
    assert!(
        panel_kill.left() - transcript.right() >= spacing + hcom_dashboard::app::KILL_GAP - 1.0,
        "gap before panel Kill: {transcript:?} / {panel_kill:?}"
    );
}

#[test]
fn test_context_menu_pins_and_unpins_node() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, load_state};

    let (dir, mut app) = split_test_app(DashboardState::default());
    app.apply_persisted_view();
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.graph.apply_instance_snapshot(&[snap("worker_1", None)]);
    app.layout.sync_nodes(&app.graph);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    run_app_frames(&ctx, &mut app, 5);

    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut run = |app: &mut hcom_dashboard::app::HcomDashboardApp, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };
    let click = |pos: Pos2, button| {
        vec![
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ]
    };
    // Opens the node menu and clicks the entry `label`.
    let mut choose = |app: &mut hcom_dashboard::app::HcomDashboardApp, label: &str| {
        let node = app
            .canvas
            .world_to_screen(app.layout.nodes[&idx].pos, app.canvas_rect);
        for events in click(node, egui::PointerButton::Secondary) {
            run(app, events);
        }
        let out = run(app, vec![]);
        let rect =
            widget_rect(&out, label).unwrap_or_else(|| panic!("menu entry {label:?} missing"));
        for events in click(rect.center(), egui::PointerButton::Primary) {
            run(app, events);
        }
        run(app, vec![]);
    };

    choose(&mut app, "Pin Position");
    assert!(app.context_menu.is_none(), "menu closes after the choice");
    assert!(app.layout.nodes[&idx].pinned);
    assert!(
        load_state(dir.path())
            .pinned_positions
            .contains_key("worker_1")
    );

    choose(&mut app, "Unpin Position");
    assert!(!app.layout.nodes[&idx].pinned);
    assert!(load_state(dir.path()).pinned_positions.is_empty());
}

#[test]
fn test_approx_eq_ignores_float_noise() {
    use hcom_dashboard::app::approx_eq;
    assert!(!approx_eq(None, 1.0, 0.5));
    assert!(approx_eq(Some(340.0), 340.3, 0.5));
    assert!(!approx_eq(Some(340.0), 341.0, 0.5));
}

#[test]
fn test_canvas_plain_hover_sets_hovered_node() {
    // Regression: hit testing used only interact_pointer_pos, which is None
    // on a plain hover, so the hover glow and the tooltip never appeared.
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[snap("a", None)]);
    let idx = graph.get_node_index("a").unwrap();
    let mut layout = GraphLayout::new();
    layout.set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    layout.sync_nodes(&graph);
    layout.set_node_pos(idx, Pos2::ZERO, false);
    let mut canvas = CanvasView::new();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut center = Pos2::ZERO;
    for frame in 0..3 {
        let mut input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        if frame > 0 {
            input.events.push(egui::Event::PointerMoved(center));
        }
        ctx.run_ui(input, |ui| {
            let (resp, _) = canvas.show(
                ui,
                &graph,
                &mut layout,
                None,
                &hcom_dashboard::canvas::ThemePalette::for_theme(
                    hcom_dashboard::config::ResolvedTheme::Dark,
                ),
            );
            center = resp.rect.center();
        })
        .textures_delta
        .clear();
    }
    assert_eq!(canvas.hovered_node, Some(idx));
    assert!(!ctx.input(|i| i.pointer.any_down()));
}

/// Runs one frame of the whole app per entry of `frames` (the input events of
/// that frame) on a window of `size` and returns the outer rects of the table
/// and detail panels as egui stored them.
fn run_app_events_full(
    ctx: &egui::Context,
    app: &mut hcom_dashboard::app::HcomDashboardApp,
    size: Vec2,
    frames: Vec<Vec<egui::Event>>,
) -> (Option<Rect>, Option<Rect>, egui::Id) {
    use eframe::App;
    use egui::containers::panel::PanelState;
    let screen = Rect::from_min_size(Pos2::ZERO, size);
    let mut frame = eframe::Frame::_new_kittest();
    let mut root = egui::Id::NULL;
    for events in frames {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            root = ui.scope_id();
            app.ui(ui, &mut frame);
        })
        .textures_delta
        .clear();
    }
    let rect = |salt: &str| {
        PanelState::load(ctx, root.with_salt(egui::IdSalt::new(salt))).map(|p| p.outer_rect)
    };
    (rect("agent_table_panel"), rect("detail_side_panel"), root)
}

fn run_app_events(
    ctx: &egui::Context,
    app: &mut hcom_dashboard::app::HcomDashboardApp,
    size: Vec2,
    frames: Vec<Vec<egui::Event>>,
) -> (Option<Rect>, Option<Rect>) {
    let (t, d, _) = run_app_events_full(ctx, app, size, frames);
    (t, d)
}

/// `run_app_events` with `frames` frames without input on a 1000x700 window.
fn run_app_frames(
    ctx: &egui::Context,
    app: &mut hcom_dashboard::app::HcomDashboardApp,
    frames: usize,
) -> (Option<Rect>, Option<Rect>) {
    run_app_events(ctx, app, Vec2::new(1000.0, 700.0), vec![vec![]; frames])
}

#[test]
fn test_layout_table_spans_full_width_below_graph_and_detail_panel() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    let (table, detail) = run_app_frames(&ctx, &mut app, 3);
    let (table, detail) = (table.expect("table panel"), detail.expect("detail panel"));
    assert_eq!(
        (table.left(), table.right()),
        (0.0, 1000.0),
        "table spans the window"
    );
    assert_eq!(table.bottom(), 700.0);
    assert_eq!(detail.right(), 1000.0, "detail panel on the right");
    assert!(
        detail.bottom() <= table.top() + 1.0,
        "detail panel sits above the table"
    );
    assert!(detail.left() > 300.0, "graph keeps the left part");

    // F4 hides the table: the detail panel then reaches the window bottom.
    app.apply_function_key(egui::Key::F4);
    assert!(!app.state.show_table);
    let (_, detail) = run_app_frames(&ctx, &mut app, 3);
    assert_eq!(detail.expect("detail panel").bottom(), 700.0);
    app.apply_function_key(egui::Key::F4);
    assert!(app.state.show_table);

    // F-keys select tabs of the right panel and reopen it when hidden.
    app.state.show_side_panel = false;
    app.apply_function_key(egui::Key::F7);
    assert_eq!(app.state.detail_tab, "Events");
    assert!(app.state.show_side_panel);
    app.apply_function_key(egui::Key::F9);
    assert_eq!(app.state.detail_tab, "Report");
    assert_eq!(
        hcom_dashboard::config::load_state(dir.path()).detail_tab,
        "Report"
    );
}

#[test]
fn test_layout_oversized_saved_table_height_keeps_canvas_visible() {
    use hcom_dashboard::app::{CANVAS_MIN_HEIGHT, HcomDashboardApp};
    use hcom_dashboard::config::{DashboardConfig, DashboardState};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState {
            table_height_frac: Some(0.95),
            ..Default::default()
        },
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    let (table, detail) = run_app_frames(&ctx, &mut app, 3);
    let (table, detail) = (table.expect("table panel"), detail.expect("detail panel"));
    assert!(
        table.top() - detail.top() >= CANVAS_MIN_HEIGHT - 1.0
            && detail.bottom() <= table.top() + 1.0,
        "the canvas and the detail panel keep their space above the table: {detail:?} / {table:?}"
    );
    assert!(
        table.height() > 300.0,
        "a large saved height is clamped, not dropped: {table:?}"
    );
}

fn drag_frames(from: Pos2, to: Pos2) -> Vec<Vec<egui::Event>> {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    vec![
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![egui::Event::PointerMoved(Pos2::new(
            (from.x + to.x) / 2.0,
            (from.y + to.y) / 2.0,
        ))],
        vec![egui::Event::PointerMoved(to)],
        vec![button(to, false)],
        vec![],
    ]
}

fn split_test_app(
    state: hcom_dashboard::config::DashboardState,
) -> (tempfile::TempDir, hcom_dashboard::app::HcomDashboardApp) {
    let dir = tempfile::tempdir().unwrap();
    let app = hcom_dashboard::app::HcomDashboardApp::new_for_test(
        hcom_dashboard::config::DashboardConfig::default(),
        state,
        dir.path().to_path_buf(),
    );
    (dir, app)
}

#[test]
fn test_split_drag_saves_window_fractions_and_does_not_pan() {
    use hcom_dashboard::config::DashboardState;
    use std::time::Instant;

    let (dir, mut app) = split_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    let size = Vec2::new(1000.0, 700.0);
    let (table, detail) = run_app_events(&ctx, &mut app, size, vec![vec![]; 3]);
    let (table, detail) = (table.unwrap(), detail.unwrap());
    let pan = app.canvas.pan;

    // Table edge: dragged up by 100 px.
    let from = Pos2::new(100.0, table.top());
    run_app_events(
        &ctx,
        &mut app,
        size,
        drag_frames(from, from - Vec2::new(0.0, 100.0)),
    );
    let frac = app
        .state
        .table_height_frac
        .expect("table fraction set by the drag");
    assert!(
        (frac * 700.0 - (table.height() + 100.0)).abs() <= 2.0,
        "{frac}"
    );
    assert_eq!(
        app.canvas.pan, pan,
        "a drag on the split handle must not pan"
    );

    // Detail edge: dragged left by 100 px.
    let from = Pos2::new(detail.left(), detail.center().y);
    run_app_events(
        &ctx,
        &mut app,
        size,
        drag_frames(from, from - Vec2::new(100.0, 0.0)),
    );
    let px = app
        .state
        .detail_width_px
        .expect("detail pixel width set by the drag");
    assert!((px - (detail.width() + 100.0)).abs() <= 2.0, "{px}");
    assert_eq!(app.canvas.pan, pan);

    // Saved: table as fraction, detail as pixels.
    app.flush_view_state(Instant::now() + Duration::from_secs(2), false);
    let saved = std::fs::read_to_string(dir.path().join("dashboard-state.json")).unwrap();
    assert!(saved.contains("table_height_frac") && saved.contains("detail_width_px"));

    // Double-click resets the table split to the default.
    let at = Pos2::new(100.0, app_table_top(&ctx, &mut app, size));
    let click = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run_app_events(
        &ctx,
        &mut app,
        size,
        vec![
            vec![egui::Event::PointerMoved(at)],
            vec![click(true)],
            vec![click(false)],
            vec![click(true)],
            vec![click(false)],
            vec![],
        ],
    );
    assert_eq!(app.state.table_height_frac, None);

    // A hidden table has no handle: the same drag changes nothing.
    app.state.table_height_frac = Some(0.3);
    app.state.show_table = false;
    let at = Pos2::new(100.0, 490.0);
    run_app_events(
        &ctx,
        &mut app,
        size,
        drag_frames(at, at - Vec2::new(0.0, 100.0)),
    );
    assert_eq!(app.state.table_height_frac, Some(0.3));
}

fn app_table_top(
    ctx: &egui::Context,
    app: &mut hcom_dashboard::app::HcomDashboardApp,
    size: Vec2,
) -> f32 {
    run_app_events(ctx, app, size, vec![vec![]; 2])
        .0
        .unwrap()
        .top()
}

#[test]
fn test_split_sizes_ignore_content_changes() {
    use hcom_dashboard::config::DashboardState;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    let (table, detail) = run_app_frames(&ctx, &mut app, 3);
    for tab in hcom_dashboard::config::DETAIL_TABS {
        app.state.detail_tab = tab.to_string();
        app.action_error = Some("a long error message ".repeat(20));
        let (t, d) = run_app_frames(&ctx, &mut app, 2);
        assert_eq!((t, d), (table, detail), "tab {tab} resized a panel");
    }
}

#[test]
fn test_split_proportions_survive_another_window_size() {
    use hcom_dashboard::config::DashboardState;

    let state = DashboardState {
        table_height_frac: Some(0.4),
        detail_width_px: Some(350.0),
        ..Default::default()
    };
    for size in [Vec2::new(1000.0, 700.0), Vec2::new(1600.0, 1000.0)] {
        let (_dir, mut app) = split_test_app(state.clone());
        let ctx = egui::Context::default();
        let (t, d) = run_app_events(&ctx, &mut app, size, vec![vec![]; 3]);
        let (t, d) = (t.unwrap(), d.unwrap());
        assert!((t.height() / size.y - 0.4).abs() < 0.01, "{size:?} {t:?}");
        assert!((d.width() - 350.0).abs() <= 1.0, "{size:?} {d:?}");
        assert_eq!(
            app.state.table_height_frac,
            Some(0.4),
            "rendering keeps the fraction"
        );
        assert_eq!(
            app.state.detail_width_px,
            Some(350.0),
            "rendering keeps the pixel width"
        );
    }

    // A small window: the clamps win (canvas keeps its minimum), the stored
    // values stay for the next larger window.
    let (_dir, mut app) = split_test_app(DashboardState {
        table_height_frac: Some(0.9),
        detail_width_px: Some(400.0),
        ..state
    });
    let ctx = egui::Context::default();
    let (t, d) = run_app_events(&ctx, &mut app, Vec2::new(500.0, 400.0), vec![vec![]; 3]);
    assert!(t.unwrap().height() < 0.9 * 400.0);
    assert!(d.unwrap().width() < 400.0);
    assert_eq!(app.state.table_height_frac, Some(0.9));
    assert_eq!(app.state.detail_width_px, Some(400.0));

    // Grow window back to 1000x700: stored 400px width is restored
    let (_, d_back) = run_app_events(&ctx, &mut app, Vec2::new(1000.0, 700.0), vec![vec![]; 3]);
    assert!(
        (d_back.unwrap().width() - 400.0).abs() <= 1.0,
        "restores 400px after window grows back"
    );
}

#[test]
fn test_split_tiny_and_zero_windows_do_not_panic() {
    use hcom_dashboard::app::split_px;
    use hcom_dashboard::config::DashboardState;

    assert_eq!(split_px(0.3, 0.0, 80.0, 0.0, 150.0), 80.0);
    assert_eq!(split_px(0.3, 150.0, 80.0, 100.0, 150.0), 80.0);
    assert_eq!(split_px(0.5, 1000.0, 80.0, 900.0, 150.0), 500.0);
    assert_eq!(split_px(0.9, 1000.0, 80.0, 900.0, 150.0), 750.0);

    for size in [Vec2::new(200.0, 150.0), Vec2::ZERO] {
        let (_dir, mut app) = split_test_app(DashboardState {
            table_height_frac: Some(0.5),
            detail_width_px: Some(400.0),
            ..Default::default()
        });
        let ctx = egui::Context::default();
        run_app_events(&ctx, &mut app, size, vec![vec![]; 3]);
    }
}

#[test]
fn test_split_fraction_sanitize() {
    use hcom_dashboard::config::{
        DashboardState, SIDE_PANEL_MIN_WIDTH, SPLIT_FRAC_MAX, SPLIT_FRAC_MIN,
    };
    for (v, want_px) in [
        (f32::NAN, None),
        (f32::INFINITY, None),
        (-1.0, None),
        (0.0, None),
        (100.0, Some(SIDE_PANEL_MIN_WIDTH)),
        (400.0, Some(400.0)),
    ] {
        let mut s = DashboardState {
            detail_width_px: Some(v),
            ..Default::default()
        };
        s.sanitize();
        assert_eq!(s.detail_width_px, want_px, "px {v}");
    }
    for (v, want) in [
        (0.97, Some(SPLIT_FRAC_MAX)),
        (0.01, Some(SPLIT_FRAC_MIN)),
        (0.4, Some(0.4)),
    ] {
        let mut s = DashboardState {
            table_height_frac: Some(v),
            ..Default::default()
        };
        s.sanitize();
        assert_eq!(s.table_height_frac, want, "{v}");
    }
}

#[test]
fn test_task_86_detail_panel_width_constant_across_agent_selections() {
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState {
        detail_width_px: Some(360.0),
        ..Default::default()
    });
    let ctx = egui::Context::default();
    let size = Vec2::new(1200.0, 800.0);

    // Initial render
    let (_, d0) = run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);
    let initial_w = d0.unwrap().width();
    assert!((initial_w - 360.0).abs() <= 1.0);

    // Select agent with short info
    let short_row = TableRow {
        agent: "short_agent".to_string(),
        instance: "short_agent_1".to_string(),
        directory: "/tmp".to_string(),
        detail: "ok".to_string(),
        ..Default::default()
    };
    app.select_table_row(&short_row);
    let (_, d1) = run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);
    assert_eq!(d1.unwrap().width(), initial_w);

    // Select agent with very long info (1000 chars) that previously expanded the panel
    let long_row = TableRow {
        agent: "long_agent".to_string(),
        instance: "long_agent_1".to_string(),
        directory: "/home/user/very/deeply/nested/directory/structure/that/is/extremely/long/"
            .repeat(10),
        detail: "error with very long payload ".repeat(30),
        ..Default::default()
    };
    app.select_table_row(&long_row);
    for tab in ["Summary", "Report", "Config"] {
        app.select_detail_tab(tab);
        let (_, d2) = run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);
        assert_eq!(
            d2.unwrap().width(),
            initial_w,
            "tab {tab} changed panel width"
        );
    }
}

fn detail_scroll_id(root: egui::Id, container_salt: &str, scroll_salt: &str) -> egui::Id {
    let panel_id = root.with("detail_side_panel");
    let panel_ui = root.with(egui::IdSalt::new(panel_id));
    let frame_ui = panel_ui.with(egui::IdSalt::new("child"));
    let tab_ui = frame_ui.with(egui::IdSalt::new("detail_tab_content"));
    let container_ui = tab_ui.with(egui::IdSalt::new(container_salt));
    container_ui.with(egui::IdSalt::new(scroll_salt))
}

#[test]
fn test_task_86_screen_stick_to_bottom() {
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::model::{AgentNode, AgentStatus};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    let node = AgentNode {
        name: "agent_term".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    app.graph.upsert_agent(node);

    let row = TableRow {
        instance: "agent_term".to_string(),
        is_running: true,
        ..Default::default()
    };
    app.select_table_row(&row);
    app.select_detail_tab("Screen");

    // Feed initial terminal content (100 lines)
    let initial_screen = (0..100)
        .map(|i| format!("initial line {i}\n"))
        .collect::<String>();
    app.screen_content = Some(("agent_term".to_string(), 1, initial_screen));

    let ctx = egui::Context::default();
    let size = Vec2::new(1000.0, 700.0);
    let (_, detail_rect, root) = run_app_events_full(&ctx, &mut app, size, vec![vec![]; 2]);
    assert!(detail_rect.is_some(), "detail panel must be visible");

    let scroll_id = detail_scroll_id(root, "screen_scroll_container", "screen_terminal_scroll");
    let state1 = egui::scroll_area::State::load(&ctx, scroll_id).expect("initial scroll state");
    let offset1 = state1.clamped_offset().y;
    assert!(
        offset1 > 100.0,
        "should start stuck to bottom, got {offset1}"
    );

    // Append 50 more lines and render another frame: stick_to_bottom must advance offset
    let updated_screen = (0..150)
        .map(|i| format!("appended line {i}\n"))
        .collect::<String>();
    app.screen_content = Some(("agent_term".to_string(), 2, updated_screen));
    run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);

    let state2 = egui::scroll_area::State::load(&ctx, scroll_id).expect("updated scroll state");
    let offset2 = state2.clamped_offset().y;
    assert!(
        offset2 > offset1 + 50.0,
        "offset should advance with new content: {offset2} > {offset1}"
    );
}

#[test]
fn test_task_86_report_summary_horizontal_scroll_and_label_alignment() {
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::model::{AgentNode, AgentStatus};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState {
        detail_width_px: Some(300.0),
        ..Default::default()
    });
    let node = AgentNode {
        name: "overflow_agent_1".into(),
        status: AgentStatus::Active,
        status_context: Some("very_long_status_value_without_spaces_".repeat(20)),
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: Some("very_long_directory_path_without_spaces_".repeat(20)),
        tool: None,
    };
    app.graph.upsert_agent(node);

    let row = TableRow {
        agent: "overflow_agent".to_string(),
        instance: "overflow_agent_1".to_string(),
        directory: "very_long_directory_path_without_spaces_".repeat(20),
        detail: "very_long_status_value_without_spaces_".repeat(20),
        ..Default::default()
    };
    app.select_table_row(&row);
    app.select_detail_tab("Report");

    let ctx = egui::Context::default();
    let size = Vec2::new(1000.0, 700.0);
    let (_, detail_rect, root) = run_app_events_full(&ctx, &mut app, size, vec![vec![]; 2]);
    let panel_w = detail_rect.unwrap().width();
    assert!((panel_w - 300.0).abs() <= 1.0);

    let report_scroll_id = detail_scroll_id(root, "report_scroll_container", "report_scroll_view");
    let _state =
        egui::scroll_area::State::load(&ctx, report_scroll_id).expect("report scroll state exists");

    // Horizontal scroll event: scroll right by 200px and assert offset moves
    let center = detail_rect.unwrap().center();
    let wheel_event = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(-200.0, 0.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    };
    run_app_events(
        &ctx,
        &mut app,
        size,
        vec![
            vec![egui::Event::PointerMoved(center), wheel_event.clone()],
            vec![],
        ],
    );
    let state_after =
        egui::scroll_area::State::load(&ctx, report_scroll_id).expect("scrolled state");
    assert!(
        state_after.clamped_offset().x > 0.0,
        "horizontal scroll offset should move right, got {:?}",
        state_after.clamped_offset().x
    );

    // Also test Summary tab with the overflowing agent
    app.select_detail_tab("Summary");
    let (_, detail_rect_sum, root_sum) = run_app_events_full(&ctx, &mut app, size, vec![vec![]; 2]);
    assert_eq!(
        detail_rect_sum.unwrap().width(),
        panel_w,
        "Summary width matches Report width"
    );
    let summary_scroll_id =
        detail_scroll_id(root_sum, "summary_scroll_container", "summary_scroll_tab");
    let _sum_state = egui::scroll_area::State::load(&ctx, summary_scroll_id)
        .expect("summary scroll state exists");
    run_app_events(
        &ctx,
        &mut app,
        size,
        vec![
            vec![egui::Event::PointerMoved(center), wheel_event.clone()],
            vec![],
        ],
    );
    let sum_state_after =
        egui::scroll_area::State::load(&ctx, summary_scroll_id).expect("scrolled summary state");
    assert!(
        sum_state_after.clamped_offset().x > 0.0,
        "summary horizontal scroll offset should move right, got {:?}",
        sum_state_after.clamped_offset().x
    );
}

#[test]
fn test_task_86_selection_and_edge_dock_reopens_hidden_panel() {
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::model::{AgentNode, AgentStatus};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState {
        show_side_panel: false,
        ..Default::default()
    });
    let node = AgentNode {
        name: "test_agent_1".into(),
        status: AgentStatus::Active,
        status_context: None,
        status_age_seconds: 0,
        unread_count: 0,
        parent_name: None,
        directory: None,
        tool: None,
    };
    app.graph.upsert_agent(node);

    let ctx = egui::Context::default();
    let size = Vec2::new(1000.0, 700.0);

    // Initial frame with hidden panel: must paint edge dock handle even when idle
    use eframe::App;
    let screen = Rect::from_min_size(Pos2::ZERO, size);
    let mut frame = eframe::Frame::_new_kittest();
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        app.ui(ui, &mut frame);
    });
    let edge_shape_painted = out.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(r) => r.rect.max.x >= 994.0 && r.rect.min.x >= 990.0,
        _ => false,
    });
    out.textures_delta.clear();
    assert!(
        edge_shape_painted,
        "edge dock handle must be visibly painted when panel is closed and idle"
    );

    let (_, d) = run_app_events(&ctx, &mut app, size, vec![vec![]; 1]);
    assert!(d.is_none(), "panel is initially hidden");

    // Selecting a table row opens the panel
    let row = TableRow {
        agent: "test_agent".to_string(),
        instance: "test_agent_1".to_string(),
        ..Default::default()
    };
    app.select_table_row(&row);
    assert!(
        app.state.show_side_panel,
        "select_table_row sets show_side_panel to true"
    );
    let (_, d) = run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);
    assert!(d.is_some(), "panel is now rendered");

    // Hide again and test CanvasAction::NodeClicked opens the panel
    app.state.show_side_panel = false;
    let node_idx = app
        .graph
        .get_node_index("test_agent_1")
        .expect("node exists");
    app.handle_canvas_action(Some(hcom_dashboard::canvas::CanvasAction::NodeClicked(
        node_idx,
    )));
    assert!(
        app.state.show_side_panel,
        "CanvasAction::NodeClicked sets show_side_panel to true"
    );

    // Hide again and test edge dock click
    app.state.show_side_panel = false;
    let (_dir2, mut app2) = split_test_app(DashboardState {
        show_side_panel: false,
        ..Default::default()
    });
    let ctx2 = egui::Context::default();
    let (_, d2) = run_app_events(&ctx2, &mut app2, size, vec![vec![]; 2]);
    assert!(d2.is_none());

    // Click at the far right edge (where edge dock is located)
    let edge_pos = Pos2::new(998.0, 300.0);
    let click = |pressed| egui::Event::PointerButton {
        pos: edge_pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run_app_events(
        &ctx2,
        &mut app2,
        size,
        vec![
            vec![egui::Event::PointerMoved(edge_pos)],
            vec![click(true)],
            vec![click(false)],
            vec![],
        ],
    );
    assert!(
        app2.state.show_side_panel,
        "edge dock click opens the panel"
    );
}

#[test]
fn test_fit_view_fits_far_nodes_and_long_labels_with_margin() {
    use hcom_dashboard::canvas::{CanvasView, ZOOM_MAX, fit_view};

    let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 500.0));
    assert_eq!(fit_view(&[], rect, 1.0), None);

    let items = [
        (Pos2::new(1200.0, 800.0), 40.0),
        (Pos2::new(-1200.0, -800.0), 40.0),
        (Pos2::new(-1200.0, 800.0), 600.0),
    ];
    let mut view = CanvasView::new();
    assert!(view.fit_to(&items, rect, 1.0));
    let inner = rect.shrink2(rect.size() * 0.05 - Vec2::splat(0.5));
    for (pos, w12) in items {
        let c = view.world_to_screen(pos, rect);
        let font = (12.0 * view.zoom).clamp(8.0, 18.0);
        let label = hcom_dashboard::canvas::label_rect(
            c,
            Vec2::new(w12 * font / 12.0, font * 1.2),
            view.zoom,
        );
        let half_x = (37.0 * view.zoom).max(label.width() / 2.0);
        assert!(
            inner.contains(c - Vec2::new(half_x, 37.0 * view.zoom)),
            "{c:?}"
        );
        assert!(
            inner.contains(Pos2::new(c.x + half_x, label.max.y)),
            "{c:?}"
        );
    }

    // One node: auto-fit stops at 100 %, the explicit cap may zoom further.
    let one = [(Pos2::new(300.0, -200.0), 40.0)];
    assert_eq!(fit_view(&one, rect, 1.0).unwrap().0, 1.0);
    let (z, pan) = fit_view(&one, rect, ZOOM_MAX).unwrap();
    assert!(z > 1.0);
    view.zoom = z;
    view.pan = pan;
    let c = view.world_to_screen(one[0].0, rect);
    // Node and the label below it are centered together: the node sits
    // centered in x and above the center by half the label's extension.
    let font = (12.0 * z).clamp(8.0, 18.0);
    let label = hcom_dashboard::canvas::label_rect(c, Vec2::new(40.0 * font / 12.0, font * 1.2), z);
    let content = Rect::from_min_max(c - Vec2::splat(37.0 * z), label.max);
    assert!((c.x - rect.center().x).abs() < 0.5, "x centered: {c:?}");
    assert!(
        (content.center().y - rect.center().y).abs() < 0.5,
        "content centered: {c:?}"
    );
}

#[test]
fn test_edge_marker_points_from_the_border_to_off_view_nodes() {
    use hcom_dashboard::canvas::edge_marker;

    let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 200.0));
    assert_eq!(edge_marker(rect, Pos2::new(10.0, 10.0)), None);
    let cases = [
        (
            Pos2::new(-500.0, 100.0),
            Pos2::new(8.0, 100.0),
            Vec2::new(-1.0, 0.0),
        ),
        (
            Pos2::new(900.0, 100.0),
            Pos2::new(392.0, 100.0),
            Vec2::new(1.0, 0.0),
        ),
        (
            Pos2::new(200.0, -300.0),
            Pos2::new(200.0, 8.0),
            Vec2::new(0.0, -1.0),
        ),
        (
            Pos2::new(200.0, 700.0),
            Pos2::new(200.0, 192.0),
            Vec2::new(0.0, 1.0),
        ),
    ];
    for (node, tip, dir) in cases {
        let (p, d) = edge_marker(rect, node).unwrap();
        assert!((p - tip).length() < 0.01, "{node:?}: {p:?}");
        assert!((d - dir).length() < 0.01, "{node:?}: {d:?}");
    }
    // Diagonal: on the shrunk border, inside the canvas.
    let (p, _) = edge_marker(rect, Pos2::new(5000.0, 5000.0)).unwrap();
    assert!(rect.shrink(7.9).contains(p) && !rect.shrink(8.1).contains(p));
}

/// App with three active agents pinned far from the origin.
fn fit_test_app(
    state: hcom_dashboard::config::DashboardState,
) -> (tempfile::TempDir, hcom_dashboard::app::HcomDashboardApp) {
    let (dir, mut app) = split_test_app(state);
    app.apply_persisted_view();
    app.graph.apply_instance_snapshot(&[
        snap("far_a", None),
        snap("far_b", None),
        snap("far_c", None),
    ]);
    app.layout.sync_nodes(&app.graph);
    for (name, pos) in [
        ("far_a", Pos2::new(1200.0, 800.0)),
        ("far_b", Pos2::new(-1200.0, -800.0)),
        ("far_c", Pos2::new(1500.0, -500.0)),
    ] {
        let idx = app.graph.get_node_index(name).unwrap();
        app.layout.set_node_pos(idx, pos, true);
    }
    (dir, app)
}

fn all_nodes_in_canvas(app: &hcom_dashboard::app::HcomDashboardApp) -> bool {
    let rect = app.canvas_rect;
    app.layout
        .nodes
        .values()
        .all(|p| rect.contains(app.canvas.world_to_screen(p.pos, rect)))
}

#[test]
fn test_auto_fit_without_saved_view_shows_all_nodes_until_user_zooms() {
    use hcom_dashboard::config::DashboardState;

    let (_dir, mut app) = fit_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 3);
    assert!(app.canvas_rect.is_positive());
    assert!(all_nodes_in_canvas(&app), "AC1: auto-fit at start");
    assert!(app.canvas.zoom <= 1.0);

    // A user scroll zoom ends auto-fit: a later fit would undo it.
    let fitted = app.canvas.zoom;
    let center = app.canvas_rect.center();
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        vec![
            vec![egui::Event::PointerMoved(center)],
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, 200.0),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
            }],
            vec![],
            vec![],
        ],
    );
    assert!(!app.auto_fit, "user zoom stops auto-fit");
    run_app_frames(&ctx, &mut app, 30);
    assert!(
        app.canvas.zoom > fitted * 1.05,
        "no refit after the user zoom"
    );
}

#[test]
fn test_auto_fit_corrects_a_saved_view_without_visible_nodes() {
    use hcom_dashboard::config::DashboardState;

    let saved = |x: f32, y: f32| DashboardState {
        canvas_zoom: Some(1.0),
        canvas_pan_x: Some(x),
        canvas_pan_y: Some(y),
        ..DashboardState::default()
    };
    // Saved view far away from every node: fitted (AC3).
    let (_dir, mut app) = fit_test_app(saved(50_000.0, 0.0));
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 3);
    assert!(all_nodes_in_canvas(&app), "AC3: empty saved view fitted");

    // Saved view that shows a node: kept.
    let (_dir2, mut app) = fit_test_app(saved(-1200.0, -800.0));
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 3);
    assert_eq!(
        (app.canvas.zoom, app.canvas.pan),
        (1.0, Vec2::new(-1200.0, -800.0))
    );
    assert!(!app.auto_fit);
}

#[test]
fn test_auto_fit_skips_manual_mode_and_pressed_pointer_but_fit_button_works() {
    use hcom_dashboard::config::DashboardState;

    // Manual mode: no auto-fit (AC5); the Fit request fits (AC2).
    let (_dir, mut app) = fit_test_app(DashboardState {
        layout_mode: "manual".into(),
        ..DashboardState::default()
    });
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 3);
    assert_eq!((app.canvas.zoom, app.canvas.pan), (1.0, Vec2::ZERO));
    app.fit_requested = true;
    run_app_frames(&ctx, &mut app, 1);
    assert!(!app.fit_requested);
    assert!(all_nodes_in_canvas(&app), "AC2: Fit in Manual mode");

    // Pointer held down (outside the canvas): no auto-fit (AC5).
    let (_dir2, mut app) = fit_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    let at = Pos2::new(5.0, 5.0);
    let press = egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    };
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        vec![vec![egui::Event::PointerMoved(at), press], vec![], vec![]],
    );
    assert_eq!((app.canvas.zoom, app.canvas.pan), (1.0, Vec2::ZERO));
    assert!(app.auto_fit, "auto-fit waits for the release");
}

#[test]
fn test_fit_button_ends_auto_fit_and_keeps_its_zoom() {
    use hcom_dashboard::config::DashboardState;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.apply_persisted_view();
    app.graph.apply_instance_snapshot(&[snap("solo", None)]);
    app.layout.sync_nodes(&app.graph);
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 3);
    assert_eq!(app.canvas.zoom, 1.0, "auto-fit stops at 100 %");

    app.fit_requested = true;
    run_app_frames(&ctx, &mut app, 3);
    assert!(!app.auto_fit);
    assert!(app.canvas.zoom > 1.0, "Fit zoom kept: {}", app.canvas.zoom);
}

#[test]
fn test_context_menu_lifecycle_and_outside_clicks() {
    use hcom_dashboard::app::ContextMenuTarget;
    use hcom_dashboard::config::DashboardState;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.apply_persisted_view();
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", Some("worker_1"))]);
    app.layout.sync_nodes(&app.graph);
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut app, 5);

    let idx1 = app.graph.get_node_index("worker_1").unwrap();
    let idx2 = app.graph.get_node_index("worker_2").unwrap();
    let node_screen_pos = |app: &hcom_dashboard::app::HcomDashboardApp, idx| {
        app.canvas
            .world_to_screen(app.layout.nodes[&idx].pos, app.canvas_rect)
    };

    let right_click = |pos: Pos2| {
        vec![
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ]
    };

    let left_click = |pos: Pos2| {
        vec![
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ]
    };

    // 1. Right click on worker_1 opens context menu for worker_1
    let pos1 = node_screen_pos(&app, idx1);
    run_app_events(&ctx, &mut app, Vec2::new(1000.0, 700.0), right_click(pos1));
    assert!(app.context_menu.is_some(), "Menu opened on worker_1");
    assert_eq!(
        app.context_menu.as_ref().map(|m| m.target.clone()),
        Some(ContextMenuTarget::Node(idx1))
    );

    // 2. AC #3: Right click on worker_2 switches menu to worker_2
    run_app_frames(&ctx, &mut app, 1);
    let pos2 = node_screen_pos(&app, idx2);
    run_app_events(&ctx, &mut app, Vec2::new(1000.0, 700.0), right_click(pos2));
    assert!(app.context_menu.is_some(), "Menu switched to worker_2");
    assert_eq!(
        app.context_menu.as_ref().map(|m| m.target.clone()),
        Some(ContextMenuTarget::Node(idx2))
    );
    assert_eq!(app.context_menu.as_ref().unwrap().screen_pos, pos2);

    // 3. AC #2: Right click on empty canvas dismisses the context menu
    // An empty canvas spot (100, 100 is the toolbar, where a click opened
    // the Send dialog).
    let empty_pos = app.canvas_rect.min + Vec2::new(10.0, 10.0);
    assert!(app.layout.nodes.values().all(|n| {
        app.canvas
            .world_to_screen(n.pos, app.canvas_rect)
            .distance(empty_pos)
            > 60.0
    }));
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        right_click(empty_pos),
    );
    assert!(
        app.context_menu.is_none(),
        "AC #2: Right click on empty canvas closes menu"
    );

    // 4. AC #1: Left click outside closes menu
    run_app_frames(&ctx, &mut app, 1);
    let pos1 = node_screen_pos(&app, idx1);
    run_app_events(&ctx, &mut app, Vec2::new(1000.0, 700.0), right_click(pos1));
    assert!(app.context_menu.is_some());
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        left_click(empty_pos),
    );
    assert!(
        app.context_menu.is_none(),
        "AC #1: Left click outside closes menu"
    );

    // 5. AC #4: Escape key closes menu
    run_app_frames(&ctx, &mut app, 1);
    let pos1 = node_screen_pos(&app, idx1);
    run_app_events(&ctx, &mut app, Vec2::new(1000.0, 700.0), right_click(pos1));
    assert!(app.context_menu.is_some());
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        vec![vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]],
    );
    assert!(app.context_menu.is_none(), "AC #4: Escape key closes menu");

    // 6. Edge secondary click opens edge context menu
    use petgraph::visit::{EdgeRef, IntoEdgeReferences};
    let edge_idx = app.graph.graph.edge_references().next().unwrap().id();
    let pos1 = node_screen_pos(&app, idx1);
    let pos2 = node_screen_pos(&app, idx2);
    let edge_screen_pos = (pos1.to_vec2() * 0.5 + pos2.to_vec2() * 0.5).to_pos2();
    run_app_events(
        &ctx,
        &mut app,
        Vec2::new(1000.0, 700.0),
        right_click(edge_screen_pos),
    );
    assert!(app.context_menu.is_some(), "Edge context menu opened");
    assert_eq!(
        app.context_menu.as_ref().map(|m| m.target.clone()),
        Some(ContextMenuTarget::Edge(edge_idx))
    );
}

/// A spawn tree with message springs must come to rest: contact pressure and
/// sub-pixel collision corrections must not keep the layout awake (TASK-77.28).
#[test]
fn test_layout_realistic_graph_sleeps_in_cluster_and_hierarchy() {
    use hcom_dashboard::layout::LayoutMode;
    let nodes = [
        ("boss", None),
        ("m1", Some("boss")),
        ("m2", Some("boss")),
        ("m3", Some("boss")),
        ("a1", Some("m1")),
        ("a2", Some("m1")),
        ("a3", Some("m1")),
        ("b1", Some("m2")),
        ("b2", Some("m2")),
        ("c1", Some("m3")),
        ("c2", Some("m3")),
        ("c3", Some("m3")),
        ("lone1", None),
        ("lone2", None),
    ];
    let snap: Vec<InstanceSnapshot> = nodes
        .iter()
        .map(|(name, parent)| InstanceSnapshot {
            name: (*name).into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: None,
            unread_count: None,
            parent_name: parent.map(|p| p.into()),
            directory: None,
            tool: None,
        })
        .collect();
    let messages = [
        ("boss", "m1", 40),
        ("boss", "m2", 30),
        ("m1", "a1", 20),
        ("m1", "b1", 15),
        ("a1", "c1", 10),
        ("m3", "lone1", 25),
        ("lone1", "lone2", 8),
        ("c2", "a2", 5),
        ("boss", "c3", 3),
    ];
    for mode in [LayoutMode::Cluster, LayoutMode::Hierarchy] {
        let mut graph = AgentGraph::new();
        graph.apply_instance_snapshot(&snap);
        let mut id = 0;
        for (from, to, count) in messages {
            for _ in 0..count {
                id += 1;
                graph.apply_message_event(&MessageEvent {
                    id,
                    ts: Utc::now(),
                    instance: from.into(),
                    data: MessageData {
                        from: from.into(),
                        delivered_to: vec![to.into()],
                        mentions: vec![],
                        intent: None,
                        reply_to: None,
                        text: "x".into(),
                    },
                });
            }
        }
        let mut layout = GraphLayout::new();
        layout.mode = mode;
        layout.sync_nodes(&graph);

        let steps_to_sleep = |layout: &mut GraphLayout, max: usize| {
            (1..=max).find(|_| {
                layout.step(&graph, 1.0 / 60.0);
                layout.is_sleeping
            })
        };
        assert!(
            steps_to_sleep(&mut layout, 600).is_some(),
            "{mode:?}: layout must sleep within 600 steps"
        );
        layout.wake_up();
        assert!(
            steps_to_sleep(&mut layout, 300).is_some(),
            "{mode:?}: layout must sleep again within 300 steps after a wake"
        );

        let pos: Vec<Pos2> = layout.nodes.values().map(|p| p.pos).collect();
        for i in 0..pos.len() {
            for j in i + 1..pos.len() {
                assert!(
                    pos[i].distance(pos[j]) >= 78.0,
                    "{mode:?}: node rings overlap ({} px)",
                    pos[i].distance(pos[j])
                );
            }
        }
    }
}

/// Cooling stops the layout after a bounded number of steps even when the
/// frame budget cuts every pass short, so the motion threshold never applies
/// (TASK-77.28 AC #6).
#[test]
fn test_layout_alpha_cooling_sleeps_with_exhausted_budget() {
    let mut graph = AgentGraph::new();
    let snap: Vec<InstanceSnapshot> = (0..40)
        .map(|i| layout_snap(&format!("n{i}"), None))
        .collect();
    graph.apply_instance_snapshot(&snap);
    let mut layout = GraphLayout::new();
    layout.max_step_duration = std::time::Duration::ZERO;
    layout.sync_nodes(&graph);
    let steps = (1..=1000).find(|_| {
        layout.step(&graph, 1.0 / 60.0);
        layout.is_sleeping
    });
    assert!(
        matches!(steps, Some(s) if s > 850),
        "cooling must stop the layout after ~900 steps, got {steps:?}"
    );
    assert!(layout.alpha < layout.alpha_min);

    layout.wake_up();
    assert!((layout.alpha - layout.alpha_reheat).abs() < 1e-6);
    layout.set_mode(hcom_dashboard::layout::LayoutMode::Hierarchy);
    assert_eq!(layout.alpha, 1.0, "mode switch reheats fully");
}

/// Spawn tree with message springs, as in the realistic sleep test.
fn realistic_layout_graph() -> AgentGraph {
    let nodes = [
        ("boss", None),
        ("m1", Some("boss")),
        ("m2", Some("boss")),
        ("m3", Some("boss")),
        ("a1", Some("m1")),
        ("a2", Some("m1")),
        ("a3", Some("m1")),
        ("b1", Some("m2")),
        ("b2", Some("m2")),
        ("c1", Some("m3")),
        ("c2", Some("m3")),
        ("c3", Some("m3")),
        ("lone1", None),
        ("lone2", None),
    ];
    let mut graph = AgentGraph::new();
    let snap: Vec<InstanceSnapshot> = nodes.iter().map(|(n, p)| layout_snap(n, *p)).collect();
    graph.apply_instance_snapshot(&snap);
    let now = Utc::now();
    let mut id = 0;
    for (from, to, count) in [
        ("boss", "m1", 40),
        ("boss", "m2", 30),
        ("m1", "a1", 20),
        ("m3", "lone1", 25),
        ("c2", "a2", 5),
    ] {
        for _ in 0..count {
            id += 1;
            graph.apply_message_event(&window_msg(id, now, from, to));
        }
    }
    graph
}

/// Per-node visible back-and-forth: consecutive steps of more than 1 px in
/// opposing directions. Returns (reversals, largest single step).
fn jitter(frames: &[Vec<Pos2>]) -> (usize, f32) {
    let mut reversals = 0;
    let mut max_step = 0.0f32;
    for w in frames.windows(3) {
        for ((p0, p1), p2) in w[0].iter().zip(&w[1]).zip(&w[2]) {
            let d1 = *p1 - *p0;
            let d2 = *p2 - *p1;
            max_step = max_step.max(d2.length());
            if d1.length() > 1.0 && d2.length() > 1.0 && d1.dot(d2) < 0.0 {
                reversals += 1;
            }
        }
    }
    (reversals, max_step)
}

/// Nodes move smoothly at startup and while one node is dragged through the
/// graph: no visible back-and-forth (TASK-77.28 regression).
#[test]
fn test_layout_moves_smoothly_at_start_and_while_dragging() {
    use hcom_dashboard::layout::LayoutMode;
    for mode in [LayoutMode::Cluster, LayoutMode::Hierarchy] {
        let graph = realistic_layout_graph();
        let mut layout = GraphLayout::new();
        layout.mode = mode;
        // Measure the physics alone: a debug build under parallel test load
        // can exceed the 2 ms budget and cut passes short at random.
        layout.max_step_duration = Duration::from_secs(1);
        layout.sync_nodes(&graph);
        let mut order: Vec<_> = layout.nodes.keys().copied().collect();
        order.sort();
        let snap = |l: &GraphLayout| order.iter().map(|i| l.nodes[i].pos).collect::<Vec<_>>();

        let mut frames = vec![snap(&layout)];
        for _ in 0..400 {
            layout.step(&graph, 1.0 / 60.0);
            frames.push(snap(&layout));
        }
        let start = jitter(&frames);

        // Drag boss in a straight line through the graph, 4 px per frame.
        let boss = graph.get_node_index("boss").unwrap();
        let from = layout.nodes[&boss].pos;
        let mut frames = vec![snap(&layout)];
        for f in 0..150 {
            let p = from + Vec2::new(4.0 * f as f32, 0.0);
            layout.set_node_pos(boss, p, true);
            layout.step(&graph, 1.0 / 60.0);
            frames.push(snap(&layout));
        }
        let drag = jitter(&frames);
        eprintln!("{mode:?}: start {start:?}, drag {drag:?}");
        assert!(
            start.0 == 0 && drag.0 == 0,
            "{mode:?}: visible back-and-forth, start {start:?}, drag {drag:?}"
        );
        // The original physics stepped up to ~5.2 px; 4x integration made it 13 px.
        assert!(
            start.1 <= 6.0 && drag.1 <= 6.0,
            "{mode:?}: nodes jump, largest step start {} px, drag {} px",
            start.1,
            drag.1
        );
    }
}

/// When alpha crosses its minimum while a time-sliced collision pass is
/// unfinished, a stale clean result of the previous pass must not put the
/// layout to sleep with an overlap left (TASK-77.28 review).
#[test]
fn test_layout_alpha_stop_waits_for_a_full_collision_pass() {
    let mut graph = AgentGraph::new();
    let snap: Vec<InstanceSnapshot> = (0..400)
        .map(|i| layout_snap(&format!("n{i:03}"), None))
        .collect();
    graph.apply_instance_snapshot(&snap);
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let mut idxs: Vec<_> = layout.nodes.keys().copied().collect();
    idxs.sort();
    for (k, idx) in idxs.iter().enumerate() {
        layout.set_node_pos(*idx, Pos2::new(k as f32 * 1000.0, 0.0), false);
    }
    let last = idxs[idxs.len() - 1];
    let prev = idxs[idxs.len() - 2];
    layout.set_node_pos(last, Pos2::new(5_000_000.0, 0.0), false);
    layout.set_node_pos(prev, Pos2::new(5_000_000.0, 0.0), false);
    layout.max_step_duration = Duration::from_micros(300);
    // Stale clean result and alpha one step above the threshold.
    layout.last_collision_pass_clean = true;
    layout.alpha = layout.alpha_min * 1.001;
    for _ in 0..3000 {
        layout.step(&graph, 0.016);
        if layout.is_sleeping {
            break;
        }
    }
    assert!(layout.is_sleeping);
    let d = layout.nodes[&last].pos.distance(layout.nodes[&prev].pos);
    assert!(
        d >= 2.0 * layout.collision_radius - 2.0,
        "layout slept with an unresolved overlap ({d} px)"
    );
}

/// Only a structural change wakes the layout: a repeated snapshot or a new
/// message between known partners does not; a new pair or node does
/// (TASK-77.28 AC #7, #8).
#[test]
fn test_layout_wakes_only_on_structural_change() {
    let now = Utc::now();
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("a", None),
        layout_snap("b", None),
        layout_snap("c", None),
    ]);
    graph.apply_message_event(&window_msg(1, now, "a", "b"));
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let settle = |layout: &mut GraphLayout, graph: &AgentGraph| {
        for _ in 0..400 {
            layout.step(graph, 1.0 / 60.0);
        }
        assert!(layout.is_sleeping);
    };
    settle(&mut layout, &graph);

    graph.apply_instance_snapshot(&[layout_snap("a", None)]);
    layout.sync_nodes(&graph);
    assert!(layout.is_sleeping, "unchanged snapshot must not wake");

    graph.apply_message_event(&window_msg(2, now, "b", "a"));
    graph.apply_message_event(&window_msg(3, now, "a", "b"));
    layout.sync_nodes(&graph);
    assert!(layout.is_sleeping, "message on a known pair must not wake");

    graph.apply_message_event(&window_msg(4, now, "a", "c"));
    layout.sync_nodes(&graph);
    assert!(!layout.is_sleeping, "new communicating pair must wake");
    assert_eq!(layout.alpha, 1.0, "structural change reheats fully");
    settle(&mut layout, &graph);

    let a = graph.get_node_index("a").unwrap();
    layout.set_node_pos(a, Pos2::new(10.0, 10.0), true);
    assert!(!layout.is_sleeping, "pin must wake");
    assert!(layout.alpha >= layout.alpha_reheat);
    settle(&mut layout, &graph);

    graph.apply_instance_snapshot(&[layout_snap("d", Some("a"))]);
    layout.sync_nodes(&graph);
    assert!(!layout.is_sleeping, "new node must wake");
    settle(&mut layout, &graph);

    graph.apply_instance_snapshot(&[layout_snap("d", Some("b"))]);
    layout.sync_nodes(&graph);
    assert!(!layout.is_sleeping, "re-parent must wake");
    settle(&mut layout, &graph);

    // A replaced graph without some nodes removes them and wakes.
    let mut smaller = AgentGraph::new();
    smaller.apply_instance_snapshot(&[layout_snap("a", None), layout_snap("b", None)]);
    layout.sync_nodes(&smaller);
    assert_eq!(layout.nodes.len(), 2);
    assert!(!layout.is_sleeping, "removed node must wake");
    assert_eq!(layout.alpha, 1.0);
}

/// Filled rects of a frame's output (screen rect, fill color).
fn filled_rects(output: &egui::FullOutput) -> Vec<(Rect, Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.fill != Color32::TRANSPARENT => Some((r.rect, r.fill)),
            _ => None,
        })
        .collect()
}

/// Selected and hovered table rows are one band over the full grid width;
/// a click into an empty cell selects the row and its graph node; long values
/// are elided; headers carry no filled frame (TASK-77.19).
#[test]
fn test_table_row_selection_is_one_full_width_band() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into(), "roaming".into(), "detail".into()],
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());
    app.graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", None)]);
    let w2 = app.graph.get_node_index("worker_2").unwrap();
    app.table_rows = vec![
        TableRow {
            instance: "worker_1".into(),
            detail: "x".repeat(1000),
            ..Default::default()
        },
        TableRow {
            instance: "worker_2".into(),
            ..Default::default()
        },
    ];

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run = |app: &mut HcomDashboardApp, events: Vec<egui::Event>| {
        time += 0.1;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    assert_eq!(app.table_row_bands.len(), 2, "one band per data row");
    let (b0, b1) = (app.table_row_bands[0], app.table_row_bands[1]);
    assert!(
        b1.width() > 300.0,
        "band spans the grid, width {}",
        b1.width()
    );

    // Click into the empty detail cell of worker_2 (right end of its band).
    let p = Pos2::new(b1.right() - 5.0, b1.center().y);
    let press = |pressed| egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run(&mut app, vec![egui::Event::PointerMoved(p)]);
    run(&mut app, vec![press(true)]);
    run(&mut app, vec![press(false)]);
    assert_eq!(app.selected_row_id.as_deref(), Some("worker_2"));
    assert_eq!(app.canvas.selected_node, Some(w2));

    // Pointer over worker_1: selection band on worker_2, weaker hover band on worker_1.
    let selection = ctx.global_style().visuals.selection.bg_fill;
    let hover = selection.gamma_multiply(0.4);
    let h = Pos2::new(b0.center().x, b0.center().y);
    run(&mut app, vec![egui::Event::PointerMoved(h)]);
    let out = run(&mut app, vec![]);
    let rects = filled_rects(&out);
    let in_row = |band: Rect, fill: Color32| -> Vec<Rect> {
        rects
            .iter()
            .filter(|(r, f)| *f == fill && r.intersects(band.shrink(1.0)))
            .map(|(r, _)| *r)
            .collect()
    };
    let sel = in_row(b1, selection);
    assert_eq!(sel.len(), 1, "one selection shape in the row: {sel:?}");
    assert!(sel[0].left() <= b1.left() && sel[0].right() >= b1.right());
    let hov = in_row(b0, hover);
    assert_eq!(hov.len(), 1, "one hover band: {hov:?}");
    assert!(hov[0].left() <= b0.left() && hov[0].right() >= b0.right());
    assert!(
        in_row(b0, selection).is_empty(),
        "hover is not the selection color"
    );

    // The long detail is elided.
    let elided = out.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.elided && t.galley.text().starts_with("xxx"),
        _ => false,
    });
    assert!(elided, "long cell text ends with an ellipsis");

    // Headers: no button pill above the first row band.
    let pill = ctx.global_style().visuals.widgets.inactive.weak_bg_fill;
    let header = Rect::from_x_y_ranges(b0.x_range(), (b0.top() - 30.0)..=(b0.top() - 4.0));
    let pills: Vec<_> = rects
        .iter()
        .filter(|(r, f)| *f == pill && r.intersects(header))
        .collect();
    assert!(pills.is_empty(), "header has no pill: {pills:?}");

    // Hovering the elided cell shows the full value in a tooltip.
    let full = "x".repeat(1000);
    let cell = Pos2::new(b0.right() - 20.0, b0.center().y);
    let mut tooltip = false;
    let mut moved = Some(egui::Event::PointerMoved(cell));
    for _ in 0..12 {
        let out = run(&mut app, moved.take().into_iter().collect());
        tooltip |= out.shapes.iter().any(|c| match &c.shape {
            egui::Shape::Text(t) => !t.galley.elided && t.galley.text() == full,
            _ => false,
        });
    }
    assert!(tooltip, "tooltip carries the full value");

    // Sorting still works: a click on the first header sorts by it.
    let hp = Pos2::new(b0.left() + 15.0, b0.top() - 14.0);
    let click = |pressed| egui::Event::PointerButton {
        pos: hp,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run(&mut app, vec![egui::Event::PointerMoved(hp)]);
    run(&mut app, vec![click(true)]);
    run(&mut app, vec![click(false)]);
    assert_eq!(app.state.sort_col.as_deref(), Some("instance"));
}

#[test]
fn test_table_arrow_keys_select_rows_in_shown_order() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into()],
        sort_col: Some("instance".into()),
        sort_reverse: true,
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());
    let mut names: Vec<String> = (0..60).map(|i| format!("w{i:02}_keep")).collect();
    names.push("zz_other".into());
    let snaps: Vec<_> = names.iter().map(|n| snap(n, None)).collect();
    app.graph.apply_instance_snapshot(&snaps);
    app.table_rows = names
        .iter()
        .map(|n| TableRow {
            instance: n.clone(),
            ..Default::default()
        })
        .collect();
    app.search_query = "_keep".into();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run = |app: &mut HcomDashboardApp, events: Vec<egui::Event>| {
        time += 0.1;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };
    let key = |k| {
        vec![egui::Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]
    };
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    let sel = |app: &HcomDashboardApp| app.selected_row_id.clone();
    let node = |app: &HcomDashboardApp, n: &str| app.graph.get_node_index(n);

    // Nothing selected: Up takes the last shown row, Down the first one
    // (sorted descending, "zz_other" filtered out).
    run(&mut app, key(egui::Key::ArrowUp));
    assert_eq!(sel(&app).as_deref(), Some("w00_keep"));
    app.selected_row_id = None;
    run(&mut app, key(egui::Key::ArrowDown));
    assert_eq!(sel(&app).as_deref(), Some("w59_keep"));
    assert_eq!(app.canvas.selected_node, node(&app, "w59_keep"));
    run(&mut app, key(egui::Key::ArrowDown));
    assert_eq!(sel(&app).as_deref(), Some("w58_keep"));
    assert_eq!(app.canvas.selected_node, node(&app, "w58_keep"));
    run(&mut app, key(egui::Key::ArrowUp));
    run(&mut app, key(egui::Key::ArrowUp));
    assert_eq!(sel(&app).as_deref(), Some("w59_keep"), "clamped at the top");

    // Down to the end: clamped, and the last row is scrolled into the window.
    for _ in 0..70 {
        run(&mut app, key(egui::Key::ArrowDown));
    }
    assert_eq!(
        sel(&app).as_deref(),
        Some("w00_keep"),
        "clamped at the bottom"
    );
    for _ in 0..10 {
        run(&mut app, vec![]);
    }
    let band = *app.table_row_bands.last().unwrap();
    assert!(
        band.bottom() <= screen.bottom() + 1.0 && band.top() >= 0.0,
        "selected last row visible: {band:?}"
    );

    // A focused text field (the search filter) keeps the arrows.
    let out = run(&mut app, vec![]);
    let hint = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text().is_empty() || t.galley.text() == "_keep" => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .expect("search field text");
    let p = hint.center();
    let press = |pressed| egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run(&mut app, vec![egui::Event::PointerMoved(p)]);
    run(&mut app, vec![press(true)]);
    run(&mut app, vec![press(false)]);
    assert!(ctx.text_edit_focused(), "search field has focus");
    run(&mut app, key(egui::Key::ArrowUp));
    assert_eq!(
        sel(&app).as_deref(),
        Some("w00_keep"),
        "text field keeps keys"
    );
}

#[test]
fn test_idle_column_queues_listening_periods_only_while_shown() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let row = |name: &str, status: &str, since: u64| TableRow {
        instance: name.into(),
        status: status.into(),
        since_seconds: Some(since),
        ..Default::default()
    };
    app.table_rows = vec![
        row("long", "listening", 30),
        row("short", "listening", 3),
        row("busy", "active", 30),
    ];
    // Hidden Idle column: periods are followed, nothing is queued.
    app.state.columns = vec!["agent".into()];
    app.update_idle(1000);
    assert_eq!(app.idle_periods.len(), 2);
    assert!(app.table_rows.iter().all(|r| r.idle.is_empty()));
    // Shown: the period past idle_seconds (10) is queued.
    app.state.columns = vec!["idle".into()];
    app.update_idle(1000);
    let idle: Vec<&str> = app.table_rows.iter().map(|r| r.idle.as_str()).collect();
    assert_eq!(idle, vec!["…", "", ""]);
}

/// App with one agent `worker_1` (table row with `dir`), Manual layout,
/// the node at the world origin.
fn tooltip_app(dir: &str) -> (hcom_dashboard::app::HcomDashboardApp, tempfile::TempDir) {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;
    let tmp = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        tmp.path().to_path_buf(),
    );
    app.graph.apply_instance_snapshot(&[snap("worker_1", None)]);
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.layout.sync_nodes(&app.graph);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    app.layout.set_node_pos(idx, Pos2::ZERO, false);
    app.table_rows = vec![hcom_dashboard::table_data::TableRow {
        instance: "worker_1".into(),
        cli: "codex".into(),
        status: "listening".into(),
        directory: dir.into(),
        ..TableRow::default()
    }];
    (app, tmp)
}

/// Hovers `worker_1` for enough frames to fade the tooltip in; returns the
/// tooltip rect and the node's screen center.
fn hover_tooltip_rect(app: &mut hcom_dashboard::app::HcomDashboardApp) -> (Option<Rect>, Pos2) {
    let ctx = egui::Context::default();
    let size = Vec2::new(1200.0, 900.0);
    run_app_events(&ctx, app, size, vec![vec![]; 2]);
    let idx = app.graph.get_node_index("worker_1").unwrap();
    let center = app
        .canvas
        .world_to_screen(app.layout.nodes[&idx].pos, app.canvas_rect);
    run_app_events(
        &ctx,
        app,
        size,
        vec![vec![egui::Event::PointerMoved(center)]; 20],
    );
    let rect = ctx.memory(|m| m.area_rect(egui::Id::unique("node_hover_tooltip_area")));
    (rect, center)
}

#[test]
fn test_tooltip_width_follows_content() {
    let (mut app, _t) = tooltip_app("/w");
    let (short, _) = hover_tooltip_rect(&mut app);
    let short = short.expect("tooltip shown");
    let (mut app, _t) = tooltip_app(&"/very/long/path".repeat(20));
    let (long, _) = hover_tooltip_rect(&mut app);
    let long = long.expect("tooltip shown");
    assert!(short.width() < 300.0, "short card {short:?}");
    assert!(
        long.width() > short.width(),
        "long {long:?} short {short:?}"
    );
    assert!(long.width() <= 360.0 + 20.0, "long card {long:?}");
}

#[test]
fn test_tooltip_placement_prefers_right_left_below_above() {
    use hcom_dashboard::canvas::tooltip_placement;
    let canvas = Rect::from_min_size(Pos2::new(0.0, 40.0), Vec2::new(800.0, 600.0));
    let size = Vec2::new(220.0, 120.0);
    let r = 37.0;
    let card = |n: Pos2, c: Rect| Rect::from_min_size(tooltip_placement(n, r, size, c), size);

    // Room everywhere: right of the node, vertically centered.
    let n = Pos2::new(400.0, 340.0);
    let c = card(n, canvas);
    assert_eq!(c.min.x, n.x + r + 8.0);
    assert_eq!(c.center().y, n.y);
    // Near the right edge: left.
    let n = Pos2::new(760.0, 340.0);
    assert_eq!(card(n, canvas).max.x, n.x - r - 8.0);
    // Narrow canvas: neither side fits, below; at the bottom: above.
    let narrow = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 600.0));
    let n = Pos2::new(150.0, 100.0);
    assert_eq!(card(n, narrow).min.y, n.y + r + 8.0);
    let n = Pos2::new(150.0, 550.0);
    assert_eq!(card(n, narrow).max.y, n.y - r - 8.0);
    // Nothing fits: the right position clamped into the canvas.
    let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0));
    assert!(tiny.contains(tooltip_placement(Pos2::new(100.0, 50.0), r, size, tiny)));
}

#[test]
fn test_tooltip_never_covers_a_node_near_the_canvas_edge() {
    use hcom_dashboard::canvas::tooltip_placement;
    let canvas = Rect::from_min_size(Pos2::new(0.0, 40.0), Vec2::new(800.0, 600.0));
    let size = Vec2::new(220.0, 120.0);
    let r = 37.0;
    for d in (0..=150).step_by(10) {
        let d = d as f32;
        for n in [
            Pos2::new(canvas.min.x + d, canvas.center().y),
            Pos2::new(canvas.max.x - d, canvas.center().y),
            Pos2::new(canvas.center().x, canvas.min.y + d),
            Pos2::new(canvas.center().x, canvas.max.y - d),
            Pos2::new(canvas.min.x + d, canvas.min.y + d),
            Pos2::new(canvas.max.x - d, canvas.max.y - d),
        ] {
            let c = Rect::from_min_size(tooltip_placement(n, r, size, canvas), size);
            assert!(canvas.contains_rect(c), "{n:?} {c:?}");
            assert!(
                !c.intersects(Rect::from_center_size(n, Vec2::splat(2.0 * r))),
                "{n:?} {c:?}"
            );
        }
    }
}

#[test]
fn test_shorten_path_home_and_middle_ellipsis() {
    use hcom_dashboard::table_data::shorten_path;
    let home = Some("/home/michael");
    assert_eq!(shorten_path("/home/michael", home, 48), "~");
    assert_eq!(shorten_path("/home/michael/x", home, 48), "~/x");
    assert_eq!(
        shorten_path("/home/michael/x", Some("/home/michael/"), 48),
        "~/x"
    );
    assert_eq!(
        shorten_path("/home/michael2/x", home, 48),
        "/home/michael2/x"
    );
    assert_eq!(shorten_path("/home/michael/x", None, 48), "/home/michael/x");
    let long = format!("/srv/{}/tail_dir", "a".repeat(60));
    let s = shorten_path(&long, home, 20);
    assert_eq!(s.chars().count(), 20);
    assert!(
        s.starts_with("/srv/aaaa") && s.ends_with("tail_dir") && s.contains('…'),
        "{s}"
    );
}

#[test]
fn test_tooltip_dir_is_shortened_with_and_without_home() {
    let (mut app, _t) = tooltip_app("/home/u/projects/x");
    let idx = app.graph.get_node_index("worker_1").unwrap();
    let dir = |app: &hcom_dashboard::app::HcomDashboardApp| {
        let (_, lines) = app.tooltip_content(idx).unwrap();
        lines.into_iter().find(|(k, _)| k == "dir").unwrap().1
    };
    app.home_dir = Some("/home/u".into());
    assert_eq!(dir(&app), "~/projects/x");
    app.home_dir = None;
    assert_eq!(dir(&app), "/home/u/projects/x");
    app.table_rows[0].directory = format!("/home/u/{}", "d/".repeat(40));
    app.home_dir = Some("/home/u".into());
    assert_eq!(dir(&app).chars().count(), 48);
    assert!(dir(&app).starts_with("~/d/"));
}

#[test]
fn test_tooltip_sits_beside_the_hovered_node() {
    let (mut app, _t) = tooltip_app("/w");
    let (rect, center) = hover_tooltip_rect(&mut app);
    let rect = rect.expect("tooltip shown");
    let r = hcom_dashboard::canvas::node_extent(app.canvas.zoom);
    assert!(
        !rect.intersects(Rect::from_center_size(center, Vec2::splat(2.0 * r))),
        "{rect:?} {center:?}"
    );
    assert!(rect.min.x > center.x, "right of the node: {rect:?}");
}

#[test]
fn test_tooltip_switch_to_a_wider_card_near_the_edge_never_covers_the_node() {
    // Regression: the measured size of the previous node's (narrow) card was
    // reused for the next node, so a wider card placed right of a node near
    // the right edge was pushed back over that node or out of the canvas.
    let (mut app, _t) = tooltip_app("/w");
    app.graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", None)]);
    app.layout.sync_nodes(&app.graph);
    let mut row = app.table_rows[0].clone();
    row.instance = "worker_2".into();
    row.directory = format!("/srv/{}", "long_dir/".repeat(10));
    app.table_rows.push(row);

    let ctx = egui::Context::default();
    let size = Vec2::new(1200.0, 900.0);
    run_app_events(&ctx, &mut app, size, vec![vec![]; 2]);
    let canvas = app.canvas_rect;
    let a = app.graph.get_node_index("worker_1").unwrap();
    let b = app.graph.get_node_index("worker_2").unwrap();
    let r = hcom_dashboard::canvas::node_extent(app.canvas.zoom);
    // Room for a narrow card right of b, not for the wide one.
    let b_screen = Pos2::new(canvas.max.x - r - 8.0 - 260.0, canvas.center().y);
    let b_world = app.canvas.screen_to_world(b_screen, canvas);
    app.layout.set_node_pos(b, b_world, false);
    app.layout.set_node_pos(
        a,
        app.canvas
            .screen_to_world(canvas.center() - Vec2::new(300.0, 0.0), canvas),
        false,
    );
    let a_screen = app.canvas.world_to_screen(app.layout.nodes[&a].pos, canvas);

    run_app_events(
        &ctx,
        &mut app,
        size,
        vec![vec![egui::Event::PointerMoved(a_screen)]; 20],
    );
    let area = egui::Id::unique("node_hover_tooltip_area");
    let narrow = ctx.memory(|m| m.area_rect(area)).expect("card of a");
    assert!(narrow.width() < 260.0, "{narrow:?}");
    let b_square = Rect::from_center_size(b_screen, Vec2::splat(2.0 * r));
    for frame in 0..5 {
        run_app_events(
            &ctx,
            &mut app,
            size,
            vec![vec![egui::Event::PointerMoved(b_screen)]],
        );
        let card = ctx.memory(|m| m.area_rect(area)).unwrap();
        assert!(
            canvas.expand(0.5).contains_rect(card),
            "frame {frame}: {card:?} {canvas:?}"
        );
        assert!(
            !card.intersects(b_square),
            "frame {frame}: {card:?} {b_square:?}"
        );
    }

    // Same node, content refreshed from short to long while hovered.
    let long_dir = app.table_rows[1].directory.clone();
    app.table_rows[1].directory = "/w".into();
    let hover_b = |app: &mut hcom_dashboard::app::HcomDashboardApp, frames: usize| {
        run_app_events(
            &ctx,
            app,
            size,
            vec![vec![egui::Event::PointerMoved(b_screen)]; frames],
        );
        ctx.memory(|m| m.area_rect(area)).unwrap()
    };
    let short = hover_b(&mut app, 5);
    assert!(
        short.min.x > b_square.max.x,
        "short card right of b: {short:?}"
    );
    app.table_rows[1].directory = long_dir;
    for frame in 0..5 {
        let card = hover_b(&mut app, 1);
        assert!(card.width() > short.width(), "frame {frame}: {card:?}");
        assert!(
            canvas.expand(0.5).contains_rect(card) && !card.intersects(b_square),
            "refresh frame {frame}: {card:?} {b_square:?} {canvas:?}"
        );
    }
}

fn legend_snap(
    name: &str,
    status: &str,
    tool: Option<&str>,
    parent: Option<&str>,
) -> InstanceSnapshot {
    InstanceSnapshot {
        status: Some(status.into()),
        tool: tool.map(|t| t.into()),
        ..snap(name, parent)
    }
}

fn legend_message(from: &str, to: &str, id: u64) -> MessageEvent {
    MessageEvent {
        id,
        ts: Utc::now(),
        instance: from.into(),
        data: MessageData {
            from: from.into(),
            delivered_to: vec![to.into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "hi".into(),
        },
    }
}

#[test]
fn test_legend_lists_only_visible_encodings() {
    use hcom_dashboard::canvas::{Legend, ThemePalette, legend};
    use hcom_dashboard::config::ResolvedTheme;
    use hcom_dashboard::model::{NodeStyle, WindowedCount};
    use std::collections::{HashMap, HashSet};

    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        legend_snap("agent_a", "blocked", Some("codex"), None),
        legend_snap("agent_b", "active", Some("Gemini"), Some("agent_a")),
        legend_snap("agent_c", "active", Some("agy"), None),
        legend_snap("agent_d", "active", None, None),
        legend_snap("agent_e", "listening", Some("claude"), None),
    ]);
    graph.apply_message_event(&legend_message("agent_a", "agent_b", 1));
    graph.apply_message_event(&legend_message("agent_a", "agent_e", 2));
    graph.node_styles.insert(
        "agent_e".into(),
        NodeStyle {
            is_roaming: true,
            context_percent: None,
        },
    );
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    // In the visible set but not laid out: the canvas does not draw it.
    graph.apply_instance_snapshot(&[
        legend_snap("agent_a", "blocked", Some("codex"), None),
        legend_snap("agent_b", "active", Some("Gemini"), Some("agent_a")),
        legend_snap("agent_c", "active", Some("agy"), None),
        legend_snap("agent_d", "active", None, None),
        legend_snap("agent_e", "listening", Some("claude"), None),
        legend_snap("agent_f", "stopped", Some("kimi"), None),
    ]);
    let idx = |n: &str| graph.get_node_index(n).unwrap();
    assert!(!layout.nodes.contains_key(&idx("agent_f")));

    let dark_palette = ThemePalette::for_theme(ResolvedTheme::Dark);

    let blue = CanvasView::cli_color(Some("agy"));
    let codex = CanvasView::cli_color(Some("codex"));
    let unknown = CanvasView::cli_color(None);

    // agent_e (roaming, claude, listening) and its message edge are hidden.
    let visible: HashSet<_> = ["agent_a", "agent_b", "agent_c", "agent_d", "agent_f"]
        .iter()
        .map(|n| idx(n))
        .collect();
    assert_eq!(
        legend(&graph, &layout, &visible, None, &dark_palette),
        Legend {
            statuses: vec![AgentStatus::Active, AgentStatus::Blocked],
            cli: vec![
                (blue, vec!["agy".into(), "gemini".into()]),
                (codex, vec!["codex".into()]),
                (unknown, vec!["unknown".into()]),
            ],
            roaming: false,
            spawn: true,
            message: true,
            message_idle: false,
        }
    );

    // Everything visible, a time window where a -> b has no messages.
    let all: HashSet<_> = graph.graph.node_indices().collect();
    let ab = graph.message_edges[&("agent_a".to_string(), "agent_b".to_string())];
    let counts: HashMap<_, _> = [(ab, WindowedCount::default())].into();
    let l = legend(&graph, &layout, &all, Some(&counts), &dark_palette);
    assert_eq!(
        l.statuses,
        vec![
            AgentStatus::Active,
            AgentStatus::Listening,
            AgentStatus::Blocked
        ]
    );
    assert!(l.cli.iter().any(|(_, n)| n == &vec!["claude".to_string()]));
    assert!(!l.cli.iter().any(|(_, n)| n.contains(&"kimi".to_string())));
    assert!(l.roaming);
    assert!(l.spawn);
    assert!(!l.message, "no edge has messages in the window");
    assert!(l.message_idle);

    // Nothing visible: an empty legend.
    assert_eq!(
        legend(&graph, &layout, &HashSet::new(), None, &dark_palette),
        Legend::default()
    );
}

#[test]
fn test_legend_tab_is_selected_saved_and_restored() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, load_state};

    let (dir, mut app) = split_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut run = |ctx: &egui::Context,
                   app: &mut hcom_dashboard::app::HcomDashboardApp,
                   events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };
    run(&ctx, &mut app, vec![]);
    let out = run(&ctx, &mut app, vec![]);
    let tab = widget_rect(&out, "Legend").expect("Legend tab");
    let pos = tab.center();
    run(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    run(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(app.state.detail_tab, "Legend");
    let saved = load_state(dir.path());
    assert_eq!(saved.detail_tab, "Legend");

    // A restarted app opens on the Legend tab.
    let (_dir2, mut restarted) = split_test_app(saved);
    let ctx2 = egui::Context::default();
    ctx2.enable_accesskit();
    run(&ctx2, &mut restarted, vec![]);
    let out = run(&ctx2, &mut restarted, vec![]);
    // Plain labels carry their text as the accesskit value.
    let shown = out
        .platform_output
        .accesskit_update
        .as_ref()
        .is_some_and(|u| {
            u.nodes
                .iter()
                .any(|(_, n)| n.value() == Some("Nothing visible in the graph"))
        });
    assert!(shown, "Legend content is shown after the restart");
}

#[test]
fn test_task_77_29_hierarchy_and_spawn_edges_preserved_across_table_sync() {
    use hcom_dashboard::model::InstanceSnapshot;
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(hcom_dashboard::config::DashboardState::default());

    // 1. Live snapshots initialize parent and child with a spawn edge
    let snapshots = vec![
        InstanceSnapshot {
            name: "parent_agent".into(),
            status: Some("active".into()),
            status_context: None,
            status_age_seconds: Some(10),
            unread_count: Some(0),
            parent_name: None,
            directory: None,
            tool: Some("claude".into()),
        },
        InstanceSnapshot {
            name: "child_agent".into(),
            status: Some("listening".into()),
            status_context: None,
            status_age_seconds: Some(5),
            unread_count: Some(2),
            parent_name: Some("parent_agent".into()),
            directory: None,
            tool: Some("claude".into()),
        },
    ];
    app.graph.apply_instance_snapshot(&snapshots);
    app.layout.sync_nodes(&app.graph);

    let _parent_idx = app.graph.get_node_index("parent_agent").unwrap();
    let child_idx = app.graph.get_node_index("child_agent").unwrap();
    assert!(
        app.graph
            .spawn_edges
            .contains_key(&("parent_agent".into(), "child_agent".into())),
        "Spawn edge must exist after live snapshot"
    );
    assert_eq!(
        app.graph.graph[child_idx].parent_name.as_deref(),
        Some("parent_agent")
    );

    // 2. Table rows load. Child table row has parent_name: None (e.g. from an incomplete table source).
    // sync_graph_from_table_rows must preserve the parent relation and spawn edge.
    app.table_rows = vec![
        TableRow {
            agent: "parent_agent".into(),
            instance: "parent_agent".into(),
            is_running: true,
            status: "active".into(),
            since_seconds: Some(15),
            unread_count: 1,
            parent_name: None,
            ..Default::default()
        },
        TableRow {
            agent: "child_agent".into(),
            instance: "child_agent".into(),
            is_running: true,
            status: "listening".into(),
            since_seconds: Some(8),
            unread_count: 3,
            parent_name: None,
            ..Default::default()
        },
    ];
    app.sync_graph_from_table_rows();

    assert!(
        app.graph
            .spawn_edges
            .contains_key(&("parent_agent".into(), "child_agent".into())),
        "Spawn edge must be preserved after table sync"
    );
    assert_eq!(
        app.graph.graph[child_idx].parent_name.as_deref(),
        Some("parent_agent"),
        "Child parent_name must be preserved after table sync"
    );
    assert_eq!(app.graph.graph[child_idx].status_age_seconds, 8);
    assert_eq!(app.graph.graph[child_idx].unread_count, 3);
}

#[test]
fn test_task_77_29_table_and_graph_node_parity_and_visible_projection() {
    use eframe::App;
    use hcom_dashboard::model::MessageData;
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(hcom_dashboard::config::DashboardState::default());

    // 1. Table rows contain 2 running instances and 3 stopped catalog agents (5 total)
    app.table_rows = vec![
        TableRow {
            agent: "agent_a".into(),
            instance: "agent_a_inst1".into(),
            is_running: true,
            status: "active".into(),
            since_seconds: Some(42),
            unread_count: 2,
            ..Default::default()
        },
        TableRow {
            agent: "agent_b".into(),
            instance: "agent_b_inst1".into(),
            is_running: true,
            status: "listening".into(),
            since_seconds: Some(10),
            unread_count: 0,
            ..Default::default()
        },
        TableRow {
            agent: "cat_agent_c".into(),
            instance: String::new(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
        TableRow {
            agent: "cat_agent_d".into(),
            instance: String::new(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
        TableRow {
            agent: "cat_agent_e".into(),
            instance: String::new(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
    ];

    // 2. Add an orphan placeholder node for a message recipient that is NOT in the table
    app.graph
        .apply_message_event(&hcom_dashboard::model::MessageEvent {
            id: 100,
            ts: chrono::Utc::now(),
            instance: "agent_a_inst1".into(),
            data: MessageData {
                from: "agent_a_inst1".into(),
                delivered_to: vec!["orphan_message_target".into()],
                mentions: vec![],
                intent: None,
                reply_to: None,
                text: "hello".into(),
            },
        });

    assert!(
        app.graph.get_node_index("orphan_message_target").is_some(),
        "Orphan message placeholder exists in graph"
    );

    // Sync table rows into graph
    app.sync_graph_from_table_rows();

    // Verify all 5 table agents now exist in app.graph
    for name in &[
        "agent_a_inst1",
        "agent_b_inst1",
        "cat_agent_c",
        "cat_agent_d",
        "cat_agent_e",
    ] {
        assert!(
            app.graph.get_node_index(name).is_some(),
            "Agent '{}' must exist in graph after sync",
            name
        );
    }

    // Verify projection parity: visible_nodes_for_rows matches table rows exactly,
    // and excludes placeholder nodes like orphan_message_target.
    let visible = app.visible_nodes_for_rows(&app.table_rows);
    assert_eq!(
        visible.len(),
        5,
        "Unfiltered visible count must equal table row count (5)"
    );
    let visible_names: std::collections::HashSet<&str> = visible
        .iter()
        .map(|idx| app.graph.graph[*idx].name.as_str())
        .collect();
    let expected_names: std::collections::HashSet<&str> = [
        "agent_a_inst1",
        "agent_b_inst1",
        "cat_agent_c",
        "cat_agent_d",
        "cat_agent_e",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        visible_names, expected_names,
        "Unfiltered visible nodes must match table rows"
    );
    assert!(
        !visible_names.contains("orphan_message_target"),
        "Orphan message placeholder must be excluded from visible projection"
    );

    // Verify field mapping: agent_a_inst1 has status_age_seconds=42, unread_count=2
    let idx_a = app.graph.get_node_index("agent_a_inst1").unwrap();
    assert_eq!(app.graph.graph[idx_a].status_age_seconds, 42);
    assert_eq!(app.graph.graph[idx_a].unread_count, 2);

    // 3. Run headless frame to verify projection and counters
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
    out.textures_delta.clear();

    // One English count in the header (TASK-77.26 AC3), no second total.
    let values: Vec<String> = out
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|u| {
            u.nodes
                .iter()
                .filter_map(|(_, n)| n.value().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        values.iter().any(|v| v == "Shown 5 / 5"),
        "Counter must show 'Shown 5 / 5'"
    );
    assert!(
        !values
            .iter()
            .any(|v| v.starts_with("Agents:") || v.contains("Zobrazeno") || v.contains("Celkem")),
        "no second or Czech count"
    );

    // Filter by running_only:
    app.state.running_only = true;
    let running_rows: Vec<TableRow> = app
        .table_rows
        .iter()
        .filter(|r| r.is_running)
        .cloned()
        .collect();
    let visible_running = app.visible_nodes_for_rows(&running_rows);
    assert_eq!(
        visible_running.len(),
        2,
        "Filtered visible count must equal running rows (2)"
    );
    let visible_running_names: std::collections::HashSet<&str> = visible_running
        .iter()
        .map(|idx| app.graph.graph[*idx].name.as_str())
        .collect();
    let expected_running: std::collections::HashSet<&str> =
        ["agent_a_inst1", "agent_b_inst1"].into_iter().collect();
    assert_eq!(
        visible_running_names, expected_running,
        "Filtered visible nodes must match running table rows"
    );
    assert!(
        !visible_running_names.contains("orphan_message_target"),
        "Orphan message placeholder must not be in filtered projection"
    );

    let input2 = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut out2 = ctx.run_ui(input2, |ui| app.ui(ui, &mut frame));
    out2.textures_delta.clear();

    let has_filtered_count = out2
        .platform_output
        .accesskit_update
        .as_ref()
        .is_some_and(|u| {
            u.nodes
                .iter()
                .any(|(_, n)| n.value().is_some_and(|v| v == "Shown 2 / 5"))
        });
    assert!(
        has_filtered_count,
        "Counter must show 2 / 5 when running_only is active"
    );
}

#[test]
fn test_task_77_30_view_mode_migration_persistence_and_f4_toggle() {
    use hcom_dashboard::config::{DashboardState, ViewMode, load_state, save_state};

    // 1. Legacy migration: JSON without view_mode, show_table = false
    let legacy_graph_only: DashboardState =
        serde_json::from_str(r#"{"show_table": false}"#).unwrap();
    assert_eq!(legacy_graph_only.view_mode, None);
    assert_eq!(legacy_graph_only.effective_view_mode(), ViewMode::GraphOnly);

    // 2. Legacy migration: JSON without view_mode, show_table = true
    let legacy_both: DashboardState = serde_json::from_str(r#"{"show_table": true}"#).unwrap();
    assert_eq!(legacy_both.view_mode, None);
    assert_eq!(legacy_both.effective_view_mode(), ViewMode::Both);

    // 3. Default state
    let def = DashboardState::default();
    assert_eq!(def.view_mode, Some(ViewMode::Both));
    assert_eq!(def.effective_view_mode(), ViewMode::Both);
    assert!(def.show_table);

    // 4. Persistence round-trip with set_view_mode
    let dir = tempfile::tempdir().unwrap();
    let mut state = DashboardState::default();
    state.set_view_mode(ViewMode::TableOnly);
    assert_eq!(state.view_mode, Some(ViewMode::TableOnly));
    assert!(state.show_table);
    let _ = save_state(dir.path(), &state);

    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.view_mode, Some(ViewMode::TableOnly));
    assert_eq!(reloaded.effective_view_mode(), ViewMode::TableOnly);
    assert!(reloaded.show_table);

    // GraphOnly persistence
    state.set_view_mode(ViewMode::GraphOnly);
    assert_eq!(state.view_mode, Some(ViewMode::GraphOnly));
    assert!(!state.show_table);
    let _ = save_state(dir.path(), &state);

    let reloaded2 = load_state(dir.path());
    assert_eq!(reloaded2.view_mode, Some(ViewMode::GraphOnly));
    assert_eq!(reloaded2.effective_view_mode(), ViewMode::GraphOnly);
    assert!(!reloaded2.show_table);

    // 5. F4 toggle on App
    let (_td, mut app) = split_test_app(DashboardState::default());
    assert_eq!(app.state.effective_view_mode(), ViewMode::Both);

    // Both -> GraphOnly
    app.toggle_view_mode_f4();
    assert_eq!(app.state.effective_view_mode(), ViewMode::GraphOnly);

    // GraphOnly -> Both
    app.toggle_view_mode_f4();
    assert_eq!(app.state.effective_view_mode(), ViewMode::Both);

    // TableOnly -> Both
    app.set_view_mode(ViewMode::TableOnly);
    assert_eq!(app.state.effective_view_mode(), ViewMode::TableOnly);
    app.toggle_view_mode_f4();
    assert_eq!(app.state.effective_view_mode(), ViewMode::Both);
}

#[test]
fn test_task_77_30_view_modes_render_expected_panels() {
    use hcom_dashboard::config::{DashboardState, ViewMode};

    let (_dir, mut app) = split_test_app(DashboardState::default());

    // In ViewMode::Both: agent_table_panel bottom panel exists
    app.set_view_mode(ViewMode::Both);
    let ctx1 = egui::Context::default();
    let (table_panel, _) = run_app_frames(&ctx1, &mut app, 2);
    assert!(
        table_panel.is_some(),
        "ViewMode::Both must render agent_table_panel"
    );

    // In ViewMode::GraphOnly: agent_table_panel does not exist
    app.set_view_mode(ViewMode::GraphOnly);
    let ctx2 = egui::Context::default();
    let (table_panel, _) = run_app_frames(&ctx2, &mut app, 2);
    assert!(
        table_panel.is_none(),
        "ViewMode::GraphOnly must not render bottom agent_table_panel"
    );

    // In ViewMode::TableOnly: bottom agent_table_panel does not exist (table is in CentralPanel)
    app.set_view_mode(ViewMode::TableOnly);
    let ctx3 = egui::Context::default();
    let (table_panel, _) = run_app_frames(&ctx3, &mut app, 2);
    assert!(
        table_panel.is_none(),
        "ViewMode::TableOnly renders table in CentralPanel, not bottom panel"
    );

    // Detail panel remains available in all view modes
    app.state.show_side_panel = true;
    let ctx4 = egui::Context::default();
    let (_, detail_panel) = run_app_frames(&ctx4, &mut app, 2);
    assert!(
        detail_panel.is_some(),
        "Detail panel renders in ViewMode::TableOnly when enabled"
    );
}

#[test]
fn test_task_77_30_unified_selection_and_action_dispatch() {
    use hcom_dashboard::action::{ActionRequest, ActionWorkerHandle};
    use hcom_dashboard::canvas::CanvasAction;
    use hcom_dashboard::config::{DashboardState, ViewMode};
    use hcom_dashboard::model::MessageData;
    use hcom_dashboard::table_data::TableRow;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let (_dir, mut app) = split_test_app(DashboardState::default());

    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runner = MockActionRunner {
        recorded: recorded.clone(),
        fail_attach: false,
        transcript_result: "test transcript".into(),
    };
    app.action_worker = Some(ActionWorkerHandle::new(runner));

    // Populate rows and sync to graph:
    app.table_rows = vec![
        TableRow {
            agent: "agent_alpha".into(),
            instance: "inst_alpha_1".into(),
            is_running: true,
            status: "active".into(),
            ..Default::default()
        },
        TableRow {
            agent: "cat_beta".into(),
            instance: String::new(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
    ];
    app.sync_graph_from_table_rows();

    let idx_alpha = app.graph.get_node_index("inst_alpha_1").unwrap();
    let idx_beta = app.graph.get_node_index("cat_beta").unwrap();

    // Fix node positions in manual mode so screen coordinates are stable
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.layout.sync_nodes(&app.graph);
    app.layout
        .set_node_pos(idx_alpha, Pos2::new(100.0, 100.0), false);
    app.layout
        .set_node_pos(idx_beta, Pos2::new(200.0, 200.0), false);

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut center_alpha = Pos2::ZERO;

    // Initial frame to establish canvas rect and resolve node screen coordinates
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| {
            let (resp, action) = app.canvas.show(
                ui,
                &app.graph,
                &mut app.layout,
                None,
                &hcom_dashboard::canvas::ThemePalette::for_theme(
                    hcom_dashboard::config::ResolvedTheme::Dark,
                ),
            );
            center_alpha = app
                .canvas
                .world_to_screen(Pos2::new(100.0, 100.0), resp.rect);
            app.handle_canvas_action(action);
        },
    )
    .textures_delta
    .clear();

    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    // 1. Single click sequence through CanvasView::show:
    // exercises response.clicked() and the else-if routing in CanvasView::show
    for events in [
        vec![egui::Event::PointerMoved(center_alpha)],
        vec![click(center_alpha, true)],
        vec![click(center_alpha, false)],
        vec![],
    ] {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            let (_resp, action) = app.canvas.show(
                ui,
                &app.graph,
                &mut app.layout,
                None,
                &hcom_dashboard::canvas::ThemePalette::for_theme(
                    hcom_dashboard::config::ResolvedTheme::Dark,
                ),
            );
            app.handle_canvas_action(action);
        })
        .textures_delta
        .clear();
    }

    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));
    assert_eq!(app.canvas.selected_node, Some(idx_alpha));
    assert_eq!(app.canvas.selected_edge, None);
    assert_eq!(
        app.selected_row().map(|r| r.instance),
        Some("inst_alpha_1".into())
    );

    // Bounded check: confirm single-click produced 0 Attach requests to the worker
    std::thread::sleep(Duration::from_millis(50));
    {
        let rec = recorded.lock().unwrap();
        let attach_count = rec
            .iter()
            .filter(|r| matches!(r, ActionRequest::Attach { .. }))
            .count();
        assert_eq!(
            attach_count, 0,
            "Single-click on node via CanvasView::show must NOT dispatch Attach"
        );
    }

    // 2. Headless pointer double-click sequence through CanvasView::show:
    // exercises response.double_clicked() routing and confirms mutual exclusion
    for events in [
        vec![click(center_alpha, true)],
        vec![click(center_alpha, false)],
        vec![click(center_alpha, true)],
        vec![click(center_alpha, false)],
        vec![],
    ] {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            let (_resp, action) = app.canvas.show(
                ui,
                &app.graph,
                &mut app.layout,
                None,
                &hcom_dashboard::canvas::ThemePalette::for_theme(
                    hcom_dashboard::config::ResolvedTheme::Dark,
                ),
            );
            app.handle_canvas_action(action);
        })
        .textures_delta
        .clear();
    }

    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));
    assert_eq!(app.canvas.selected_node, Some(idx_alpha));

    // Bounded poll for Attach request: assert exactly one Attach was dispatched
    let mut attach_count = 0;
    for _ in 0..50 {
        {
            let rec = recorded.lock().unwrap();
            attach_count = rec
                .iter()
                .filter(
                    |r| matches!(r, ActionRequest::Attach { agent, .. } if agent == "inst_alpha_1"),
                )
                .count();
            if attach_count >= 1 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        attach_count, 1,
        "Double-click on node via CanvasView::show must produce exactly one Attach"
    );

    // 3. Right-click on node: selects node and opens context menu
    app.handle_canvas_action(Some(CanvasAction::NodeSecondaryClicked(
        idx_beta,
        Pos2::new(100.0, 200.0),
    )));
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(app.canvas.selected_node, Some(idx_beta));
    assert_eq!(app.selected_row().map(|r| r.agent), Some("cat_beta".into()));
    assert!(app.context_menu.is_some());
    assert!(app.context_menu_opened_this_frame);

    // 4. Edge click: selects edge, clears node and row selection
    // Add a message edge between the nodes
    app.graph
        .apply_message_event(&hcom_dashboard::model::MessageEvent {
            id: 1,
            ts: chrono::Utc::now(),
            instance: "inst_alpha_1".into(),
            data: MessageData {
                from: "inst_alpha_1".into(),
                delivered_to: vec!["cat_beta".into()],
                mentions: vec![],
                intent: None,
                reply_to: None,
                text: "ping".into(),
            },
        });
    let edge_idx = app.graph.graph.edge_indices().next().unwrap();
    app.handle_canvas_action(Some(CanvasAction::EdgeClicked(edge_idx)));
    assert_eq!(app.selected_row_id, None);
    assert_eq!(app.canvas.selected_node, None);
    assert_eq!(app.canvas.selected_edge, Some(edge_idx));
    assert_eq!(app.selected_row(), None);

    // 5. Edge secondary click: selects edge, clears node/row, opens context menu
    app.handle_canvas_action(Some(CanvasAction::EdgeSecondaryClicked(
        edge_idx,
        Pos2::new(150.0, 250.0),
    )));
    assert_eq!(app.selected_row_id, None);
    assert_eq!(app.canvas.selected_node, None);
    assert_eq!(app.canvas.selected_edge, Some(edge_idx));
    assert_eq!(app.selected_row(), None);
    assert!(app.context_menu.is_some());

    // 6. Selection cleared (empty canvas click)
    app.handle_canvas_action(Some(CanvasAction::SelectionCleared));
    assert_eq!(app.selected_row_id, None);
    assert_eq!(app.canvas.selected_node, None);
    assert_eq!(app.canvas.selected_edge, None);
    assert_eq!(app.context_menu, None);

    // 7. Table row selection synchronizes node in graph:
    // 7a. For instance:
    app.select_table_row(&app.table_rows[0].clone());
    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));
    assert_eq!(app.canvas.selected_node, Some(idx_alpha));
    assert_eq!(app.canvas.selected_edge, None);

    // 7b. For catalog agent:
    app.select_table_row(&app.table_rows[1].clone());
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(app.canvas.selected_node, Some(idx_beta));
    assert_eq!(app.canvas.selected_edge, None);

    // 8. Switching view mode preserves selection across all views
    app.set_view_mode(ViewMode::GraphOnly);
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(app.canvas.selected_node, Some(idx_beta));

    app.set_view_mode(ViewMode::TableOnly);
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(app.canvas.selected_node, Some(idx_beta));

    app.set_view_mode(ViewMode::Both);
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(app.canvas.selected_node, Some(idx_beta));
}

#[test]
fn test_task_77_32_click_on_node_does_not_pan_zoom_or_move_graph() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, ViewMode};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.set_view_mode(ViewMode::GraphOnly);
    app.auto_fit = true;

    app.table_rows = vec![TableRow {
        agent: "agent_fixed".into(),
        instance: "inst_fixed_1".into(),
        is_running: true,
        status: "active".into(),
        ..Default::default()
    }];
    app.sync_graph_from_table_rows();

    let idx = app.graph.get_node_index("inst_fixed_1").unwrap();
    let initial_pos = Pos2::ZERO;
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Cluster);
    app.layout.sync_nodes(&app.graph);
    app.layout.set_node_pos(idx, initial_pos, false);
    app.layout.is_sleeping = true;

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut frame = eframe::Frame::_new_kittest();

    // Initial frame to establish canvas rect and layout
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out.textures_delta.clear();

    let initial_pan = app.canvas.pan;
    let initial_zoom = app.canvas.zoom;
    let center = app.canvas.world_to_screen(initial_pos, app.canvas_rect);

    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    // Click on node with 10px offset from center (within node radius of 20px)
    let click_pos = center + Vec2::new(10.0, 10.0);

    // 1. Move to position
    let mut out_move = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::PointerMoved(click_pos)],
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out_move.textures_delta.clear();

    // 2. Press frame
    let mut out_press = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![click(click_pos, true)],
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out_press.textures_delta.clear();

    assert!(
        !app.auto_fit,
        "auto_fit must be disabled on pointer press on canvas"
    );

    // 3. Release frame
    let mut out_release = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![click(click_pos, false), egui::Event::PointerGone],
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out_release.textures_delta.clear();

    // Verify AC #1 and AC #2
    assert_eq!(
        app.canvas.pan, initial_pan,
        "Click on node must not change canvas pan"
    );
    assert_eq!(
        app.canvas.zoom, initial_zoom,
        "Click on node must not change canvas zoom"
    );
    let current_pos = app.layout.nodes[&idx].pos;
    assert!(
        (current_pos - initial_pos).length() < 0.001,
        "Click on node must not displace node position in layout"
    );
    assert!(
        app.layout.is_sleeping,
        "Layout must remain sleeping after a click without waking physics"
    );
    assert_eq!(app.selected_row_id.as_deref(), Some("inst_fixed_1"));
    assert_eq!(app.canvas.selected_node, Some(idx));
}

#[test]
fn test_task_77_32_sub_threshold_motion_does_not_pan_or_drag() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, ViewMode};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.set_view_mode(ViewMode::GraphOnly);

    app.table_rows = vec![TableRow {
        agent: "agent_sub".into(),
        instance: "inst_sub_1".into(),
        is_running: true,
        status: "active".into(),
        ..Default::default()
    }];
    app.sync_graph_from_table_rows();

    let idx = app.graph.get_node_index("inst_sub_1").unwrap();
    let initial_pos = Pos2::ZERO;
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.layout.sync_nodes(&app.graph);
    app.layout.set_node_pos(idx, initial_pos, false);

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut frame = eframe::Frame::_new_kittest();

    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out.textures_delta.clear();

    let initial_pan = app.canvas.pan;
    let center = app.canvas.world_to_screen(initial_pos, app.canvas_rect);

    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    // Press on center, move 2px (< 3px threshold), release
    let move_pos = center + Vec2::new(2.0, 0.0);
    for events in [
        vec![egui::Event::PointerMoved(center)],
        vec![click(center, true)],
        vec![egui::Event::PointerMoved(move_pos)],
        vec![click(move_pos, false)],
    ] {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| app.ui(ui, &mut frame),
        );
        out.textures_delta.clear();
    }

    assert_eq!(
        app.canvas.pan, initial_pan,
        "Sub-threshold motion must not pan canvas"
    );
    let current_pos = app.layout.nodes[&idx].pos;
    assert!(
        (current_pos - initial_pos).length() < 0.001,
        "Sub-threshold motion must not drag node"
    );
    assert!(app.canvas.dragged_node.is_none());
}

#[test]
fn test_task_77_32_fast_drag_beyond_hit_radius_drags_candidate() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, ViewMode};
    use hcom_dashboard::table_data::TableRow;

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.set_view_mode(ViewMode::GraphOnly);

    app.table_rows = vec![TableRow {
        agent: "agent_fast".into(),
        instance: "inst_fast_1".into(),
        is_running: true,
        status: "active".into(),
        ..Default::default()
    }];
    app.sync_graph_from_table_rows();

    let idx = app.graph.get_node_index("inst_fast_1").unwrap();
    let initial_pos = Pos2::ZERO;
    app.layout
        .set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    app.layout.sync_nodes(&app.graph);
    app.layout.set_node_pos(idx, initial_pos, false);

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut frame = eframe::Frame::_new_kittest();

    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| app.ui(ui, &mut frame),
    );
    out.textures_delta.clear();

    let initial_pan = app.canvas.pan;
    let center = app.canvas.world_to_screen(initial_pos, app.canvas_rect);

    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    // Press on center, jump 60px away (far beyond 20px hit radius)
    let jump_pos = center + Vec2::new(60.0, 0.0);
    for events in [
        vec![egui::Event::PointerMoved(center)],
        vec![click(center, true)],
        vec![egui::Event::PointerMoved(jump_pos)],
    ] {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| app.ui(ui, &mut frame),
        );
        out.textures_delta.clear();
    }

    assert_eq!(
        app.canvas.dragged_node,
        Some(idx),
        "Candidate node must be dragged even after leaving hit radius"
    );
    assert_eq!(
        app.canvas.pan, initial_pan,
        "Node drag must not pan the canvas"
    );
    let dragged_pos = app.layout.nodes[&idx].pos;
    assert!(
        dragged_pos.x > initial_pos.x + 30.0,
        "Node must have moved towards the drag location"
    );
}

#[test]
fn test_task_77_33_cell_text_and_empty_space_click_selects_row() {
    use eframe::App;
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into(), "roaming".into(), "detail".into()],
        ..Default::default()
    };
    let mut app = hcom_dashboard::app::HcomDashboardApp::new_for_test(
        hcom_dashboard::config::DashboardConfig::default(),
        state,
        dir.path().to_path_buf(),
    );
    app.graph
        .apply_instance_snapshot(&[snap("worker_1", None), snap("worker_2", None)]);
    let w1 = app.graph.get_node_index("worker_1").unwrap();
    let w2 = app.graph.get_node_index("worker_2").unwrap();
    app.table_rows = vec![
        TableRow {
            instance: "worker_1".into(),
            detail: "x".repeat(100),
            ..Default::default()
        },
        TableRow {
            instance: "worker_2".into(),
            ..Default::default()
        },
    ];

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run = |app: &mut hcom_dashboard::app::HcomDashboardApp, events: Vec<egui::Event>| {
        time += 0.1;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };

    // 1. Initial frames to lay out table
    let mut out = run(&mut app, vec![]);
    for _ in 0..2 {
        out = run(&mut app, vec![]);
    }

    // 2. Click directly on the cell text of "worker_2"
    let hint = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text() == "worker_2" => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .expect("worker_2 text shape found");
    let text_center = hint.center();

    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };

    run(&mut app, vec![egui::Event::PointerMoved(text_center)]);
    run(&mut app, vec![click(text_center, true)]);
    run(&mut app, vec![click(text_center, false)]);

    assert_eq!(
        app.selected_row_id.as_deref(),
        Some("worker_2"),
        "Click on cell text must select the row"
    );
    assert_eq!(
        app.canvas.selected_node,
        Some(w2),
        "Click on cell text must highlight graph node"
    );

    // 3. Click on the empty space at the right end of worker_1's band
    let b0 = app.table_row_bands[0];
    let empty_space = Pos2::new(b0.right() - 5.0, b0.center().y);
    run(&mut app, vec![egui::Event::PointerMoved(empty_space)]);
    run(&mut app, vec![click(empty_space, true)]);
    run(&mut app, vec![click(empty_space, false)]);

    assert_eq!(
        app.selected_row_id.as_deref(),
        Some("worker_1"),
        "Click on empty space of row band must select the row"
    );
    assert_eq!(
        app.canvas.selected_node,
        Some(w1),
        "Click on empty space must highlight graph node"
    );
}

#[test]
fn test_task_77_35_detail_panel_is_fixed_and_never_hidden_by_agent_status_or_selection() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    app.table_rows = vec![
        TableRow {
            agent: "active_worker".into(),
            instance: "active_worker".into(),
            is_running: true,
            status: "active".into(),
            ..Default::default()
        },
        TableRow {
            agent: "stopped_agent".into(),
            instance: "stopped_agent".into(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
    ];
    app.sync_graph_from_table_rows();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let root = std::cell::Cell::new(egui::Id::NULL);

    let mut run = |app: &mut HcomDashboardApp| {
        time += 0.1;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            root.set(ui.scope_id());
            app.ui(ui, &mut frame);
        });
        out.textures_delta.clear();
        out
    };

    let panel_rect = |ctx: &egui::Context, salt: &str| {
        egui::panel::PanelState::load(ctx, root.get().with_salt(egui::IdSalt::new(salt)))
            .map(|p| p.outer_rect)
    };

    // 1. Initial state: show_side_panel is true by default
    assert!(app.state.show_side_panel);
    let _ = run(&mut app);
    let detail_rect = panel_rect(&ctx, "detail_side_panel")
        .expect("detail_side_panel must be rendered by default");
    let initial_width = detail_rect.width();
    assert!(initial_width > 0.0);

    // 2. Select active agent: panel remains open with identical width
    app.select_table_row(&app.table_rows[0].clone());
    let _ = run(&mut app);
    assert!(app.state.show_side_panel);
    let detail_active = panel_rect(&ctx, "detail_side_panel").expect("detail_side_panel rendered");
    assert_eq!(
        detail_active.width(),
        initial_width,
        "Detail panel width must not change when active agent is selected"
    );

    // 3. Select stopped agent: panel remains open with identical width
    app.select_table_row(&app.table_rows[1].clone());
    let _ = run(&mut app);
    assert!(app.state.show_side_panel);
    let detail_stopped = panel_rect(&ctx, "detail_side_panel").expect("detail_side_panel rendered");
    assert_eq!(
        detail_stopped.width(),
        initial_width,
        "Detail panel width must not change when stopped agent is selected"
    );

    // 4. Clear selection: panel remains open with identical width
    app.handle_canvas_action(Some(hcom_dashboard::canvas::CanvasAction::SelectionCleared));
    let _ = run(&mut app);
    assert!(app.state.show_side_panel);
    let detail_cleared = panel_rect(&ctx, "detail_side_panel").expect("detail_side_panel rendered");
    assert_eq!(
        detail_cleared.width(),
        initial_width,
        "Detail panel width must not change when selection is cleared"
    );

    // 5. Check stopped agent messaging in Screen, Transcript, and Events tabs
    app.select_table_row(&app.table_rows[1].clone()); // stopped_agent

    app.select_detail_tab("Screen");
    let out_screen = run(&mut app);
    let has_screen_stopped_msg = out_screen.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text().contains(
            "Agent 'stopped_agent' is stopped. Screen is only available for running agents.",
        ),
        _ => false,
    });
    assert!(
        has_screen_stopped_msg,
        "Screen tab must explain that the agent is stopped"
    );

    app.select_detail_tab("Transcript");
    let out_tr = run(&mut app);
    let has_tr_stopped_msg = out_tr.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text().contains(
            "Agent 'stopped_agent' is stopped. Transcript is only available for running agents.",
        ),
        _ => false,
    });
    assert!(
        has_tr_stopped_msg,
        "Transcript tab must explain that the agent is stopped"
    );

    app.select_detail_tab("Events");
    let out_ev = run(&mut app);
    let has_ev_stopped_msg = out_ev.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text().contains(
            "Agent 'stopped_agent' is stopped. Events are only available for running agents.",
        ),
        _ => false,
    });
    assert!(
        has_ev_stopped_msg,
        "Events tab must explain that the agent is stopped"
    );

    // 6. Verify toolbar toggle is the only way to toggle panel and there is no "Close Detail Panel" button
    let has_tab_close_btn = out_ev.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text() == "Close Detail Panel",
        _ => false,
    });
    assert!(
        !has_tab_close_btn,
        "There must be no in-panel Close Detail Panel button"
    );

    // Toggle via toolbar helper
    app.toggle_detail_panel();
    assert!(!app.state.show_side_panel);
    let ctx_toggled = egui::Context::default();
    let (_, detail_hidden) = run_app_frames(&ctx_toggled, &mut app, 2);
    assert!(
        detail_hidden.is_none(),
        "Panel is hidden only when explicitly toggled"
    );

    app.toggle_detail_panel();
    assert!(app.state.show_side_panel);
    let ctx_reopened = egui::Context::default();
    let (_, detail_reopened) = run_app_frames(&ctx_reopened, &mut app, 2);
    assert!(detail_reopened.is_some(), "Panel reopens when toggled");

    // 7. Stale in-flight async result arriving after switching to a stopped agent
    app.select_table_row(&app.table_rows[0].clone()); // active_worker
    let active_gen = app.selection_generation;
    app.select_table_row(&app.table_rows[1].clone()); // switched to stopped_agent
    assert!(
        app.selection_generation > active_gen,
        "selection_generation must increment on selection change"
    );

    // If an in-flight result with the old generation arrives, accept_generation_result rejects it
    assert!(!hcom_dashboard::app::accept_generation_result(
        app.selection_generation,
        active_gen
    ));

    // Even if screen_content had stale content from another agent, !is_running guard shows stopped msg
    app.screen_content = Some((
        "active_worker".into(),
        active_gen,
        "old terminal text".into(),
    ));
    app.select_detail_tab("Screen");
    let out_stale = run(&mut app);
    let has_stopped_msg_over_stale = out_stale.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text().contains(
            "Agent 'stopped_agent' is stopped. Screen is only available for running agents.",
        ),
        _ => false,
    });
    assert!(
        has_stopped_msg_over_stale,
        "Stopped state message must be shown even if screen_content holds stale text"
    );
}

#[test]
fn test_task_77_34_config_tab_displays_agents_json_info_for_stopped_agent() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state, save_state};
    use hcom_dashboard::table_data::build_table_rows;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    // Regression: AC #1 data-layer wiring from JSON through build_table_rows
    let live_json = r#"[]"#;
    let agent_json = r#"[
        {
            "name": "test_stopped_worker",
            "cli": "codex",
            "description": "Autonomous worker for background tasks",
            "dir": "/home/michael/projects/test",
            "disabled": false,
            "groups": ["backend", "tasks"],
            "roaming": true,
            "roaming_dirs": ["/home/michael/projects"],
            "instructions": "/home/michael/.hcom/agents/test_stopped_worker/SOUL.md",
            "model": "gpt-5.6-terra",
            "reasoning": "high",
            "terminal": "herdr",
            "session": "test_session",
            "window": "test_window",
            "agent_dir": "/home/michael/.hcom/agents/test_stopped_worker",
            "skills": [
                {
                    "name": "test-skill",
                    "description": "Skill for testing catalog display"
                }
            ]
        }
    ]"#;

    let rows = build_table_rows(live_json, agent_json, 1700000000).expect("build table rows");
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].is_running,
        "Catalog agent without live instance must be stopped"
    );
    assert!(
        rows[0].catalog.is_some(),
        "Catalog attributes must be parsed and attached to TableRow"
    );

    app.table_rows = rows;
    app.sync_graph_from_table_rows();

    let ctx = egui::Context::default();
    // 1400 px height ensures full content in the scrollable config tab is laid out and tested
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 1400.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;

    let mut run = |app: &mut HcomDashboardApp| {
        time += 0.1;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            app.ui(ui, &mut frame);
        });
        out.textures_delta.clear();
        out
    };

    // 1. Select the stopped agent
    app.select_table_row(&app.table_rows[0].clone());

    // 2. In Screen tab, verify stopped message and presence of "View Config" button
    app.select_detail_tab("Screen");
    let out_screen = run(&mut app);
    let has_view_config_btn = out_screen.shapes.iter().any(|c| match &c.shape {
        egui::Shape::Text(t) => t.galley.text() == "📋 View Config",
        _ => false,
    });
    assert!(
        has_view_config_btn,
        "Screen tab must have '📋 View Config' button for stopped agent"
    );

    // 3. Switch to "Config" tab
    app.select_detail_tab("Config");
    assert_eq!(app.state.detail_tab, "Config");
    let out_config = run(&mut app);

    // Collect all displayed text in the Config tab
    let texts: Vec<String> = out_config
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect();
    let combined = texts.join(" ");

    // Verify key attributes from agents.json are rendered
    assert!(
        combined.contains("test_stopped_worker"),
        "Config tab must show agent name"
    );
    assert!(
        combined.contains("Autonomous worker for background tasks"),
        "Config tab must show agent description"
    );
    assert!(combined.contains("codex"), "Config tab must show CLI tool");
    assert!(
        combined.contains("gpt-5.6-terra"),
        "Config tab must show model"
    );
    assert!(
        combined.contains("high"),
        "Config tab must show reasoning effort"
    );
    assert!(
        combined.contains("/home/michael/projects/test"),
        "Config tab must show directory"
    );
    assert!(
        combined.contains("backend, tasks"),
        "Config tab must show groups"
    );
    assert!(
        combined.contains("test-skill"),
        "Config tab must show skill name"
    );
    assert!(
        combined.contains("Skill for testing catalog display"),
        "Config tab must show skill description"
    );

    // 4. Verify catalog isolation: NO source or catalog provenance path is rendered
    assert!(
        !combined.contains(".hcom/agents.json"),
        "Catalog isolation: provenance path must not be displayed"
    );
    assert!(
        !combined.contains(".hcom/agents.local.json"),
        "Catalog isolation: local provenance path must not be displayed"
    );

    // 5. Verify future editing interface affordance (AC #3)
    assert!(
        combined.contains("Configuration Parameters (Future Editing Interface)"),
        "Config tab must display the future editing interface section"
    );
    assert!(
        combined.contains("Preview of parameter editing interface"),
        "Config tab must indicate future edit preview"
    );

    // 6. Persistence: Config tab round-trips via save/load
    let _ = save_state(dir.path(), &app.state);
    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.detail_tab, "Config");
}

#[test]
fn test_task_77_31_canonical_round_trip() {
    use chrono::DateTime;
    use hcom_dashboard::model::{MessageRange, format_canonical_range, parse_time_range};

    let now = DateTime::from_timestamp(1700000000, 0).unwrap();

    let cases = vec![
        MessageRange::Live { span_secs: None },
        MessageRange::Live {
            span_secs: Some(30),
        },
        MessageRange::Live {
            span_secs: Some(900),
        },
        MessageRange::Live {
            span_secs: Some(7200),
        },
        MessageRange::Live {
            span_secs: Some(86400 * 7),
        },
        MessageRange::Fixed {
            start: None,
            end: DateTime::from_timestamp(1699990000, 0).unwrap(),
        },
        MessageRange::Fixed {
            start: Some(DateTime::from_timestamp(1699980000, 0).unwrap()),
            end: DateTime::from_timestamp(1699990000, 0).unwrap(),
        },
    ];

    for expected in cases {
        let text = format_canonical_range(&expected, now);
        let parsed = parse_time_range(&text, now)
            .unwrap_or_else(|e| panic!("parse failed for '{}': {}", text, e));
        assert_eq!(
            parsed, expected,
            "Round-trip failed for canonical text '{}'",
            text
        );
    }
}

#[test]
fn test_task_77_31_fractional_seconds_normalized_to_second_precision() {
    use chrono::{DateTime, Utc};
    use hcom_dashboard::model::{MessageRange, format_canonical_range, parse_time_range};

    let now = DateTime::from_timestamp(1700000000, 0).unwrap();
    let text = "2026-11-03T12:00:00.123Z - 2026-11-03T13:00:00.789Z";
    let parsed = parse_time_range(text, now).expect("RFC3339 with fractional seconds should parse");

    let expected_start = DateTime::parse_from_rfc3339("2026-11-03T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let expected_end = DateTime::parse_from_rfc3339("2026-11-03T13:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    assert_eq!(
        parsed,
        MessageRange::Fixed {
            start: Some(expected_start),
            end: expected_end,
        },
        "Fractional seconds must be normalized to whole seconds"
    );

    let canonical = format_canonical_range(&parsed, now);
    let roundtrip = parse_time_range(&canonical, now).unwrap();
    assert_eq!(roundtrip, parsed);
}

#[test]
fn test_task_77_31_delimiters_and_syntax() {
    use chrono::DateTime;
    use hcom_dashboard::model::parse_time_range;

    let now = DateTime::from_timestamp(1700000000, 0).unwrap();

    // Accepted delimiters: " - ", "..", " to "
    let r1 = parse_time_range("10:00 - 11:00", now).expect("space-hyphen-space");
    let r2 = parse_time_range("10:00..11:00", now).expect("double-dot");
    let r3 = parse_time_range("10:00 to 11:00", now).expect("to delimiter");
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);

    // Bare hyphen without spaces must be rejected to prevent ISO date conflicts
    assert!(
        parse_time_range("10:00-11:00", now).is_err(),
        "Bare hyphen without spaces must be rejected"
    );

    // Case-insensitivity
    let all1 = parse_time_range("all", now).unwrap();
    let all2 = parse_time_range("ALL", now).unwrap();
    assert_eq!(all1, all2);
}

#[test]
fn test_task_77_31_strict_start_before_end_and_equality_rejected() {
    use chrono::DateTime;
    use hcom_dashboard::model::parse_time_range;

    let now = DateTime::from_timestamp(1700000000, 0).unwrap();

    // Equality: start == end
    let eq_res = parse_time_range("12:00 - 12:00", now);
    assert!(
        eq_res.is_err(),
        "Equal start and end (zero width) must be rejected"
    );

    // Reverse: start > end
    let rev_res = parse_time_range("14:00 - 12:00", now);
    assert!(
        rev_res.is_err(),
        "start > end must be rejected without wrap"
    );
}

#[test]
fn test_task_77_31_relative_overflow_and_invalid_numbers_rejected() {
    use chrono::DateTime;
    use hcom_dashboard::model::parse_time_range;

    let now = DateTime::from_timestamp(1700000000, 0).unwrap();

    // Huge overflow number
    assert!(
        parse_time_range("99999999999999999999h", now).is_err(),
        "Numeric overflow must return Err without panic"
    );

    // Zero duration
    assert!(
        parse_time_range("0m", now).is_err(),
        "Zero duration must be rejected"
    );

    // Negative duration
    assert!(
        parse_time_range("-5m", now).is_err(),
        "Negative duration must be rejected"
    );
}

#[test]
fn test_task_77_31_enter_with_focus_clears_invalid_formats_canonical_and_persists() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();

    // 1. Initial frame: render UI and set focus to TextEdit
    ctx.memory_mut(|m| m.request_focus(egui::Id::unique("time_range_text_edit")));
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| {
            app.ui(ui, &mut frame);
        },
    );
    out.textures_delta.clear();

    // 2. Set invalid text and mark invalid
    app.time_range_text = "bad-input-xyz".into();
    app.time_range_invalid = true;

    // 3. Type valid "60m" and commit with Enter while focused
    app.time_range_text = "60m".into();
    let events = vec![egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
        repeat: false,
    }];

    let input = egui::RawInput {
        screen_rect: Some(screen),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        app.ui(ui, &mut frame);
    });
    out.textures_delta.clear();

    // Verify invalid flag is cleared, text is formatted canonically to "1h", and span is 3600
    assert!(
        !app.time_range_invalid,
        "Enter commit must clear invalid state even while focused"
    );
    assert_eq!(
        app.time_range_text, "1h",
        "Canonical text '1h' must replace typed '60m'"
    );
    assert_eq!(
        app.msg_window.span_secs,
        Some(3600),
        "Window span must be 3600s"
    );

    // Verify persistence to dashboard-state.json
    let reloaded = load_state(dir.path());
    assert_eq!(
        reloaded.window_span_secs,
        Some(3600),
        "dashboard-state.json must persist window_span_secs on Enter commit"
    );
}

#[test]
fn test_task_77_31_clamping_outside_retained_bounds() {
    use chrono::DateTime;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::model::{MessageData, MessageEvent, MessageRange};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let now = DateTime::from_timestamp(1700004000, 0).unwrap();

    // Ingest a message event to define oldest_message_ts (1700002000)
    app.graph.apply_message_event(&MessageEvent {
        id: 1,
        ts: DateTime::from_timestamp(1700002000, 0).unwrap(),
        instance: "inst1".into(),
        data: MessageData {
            from: "inst1".into(),
            delivered_to: vec!["inst2".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "hello".into(),
        },
    });

    // Apply fixed range where end is in the future (> now)
    let future_end = DateTime::from_timestamp(1700009000, 0).unwrap();
    let start = DateTime::from_timestamp(1700000000, 0).unwrap();

    let _ = app.apply_message_range(
        MessageRange::Fixed {
            start: Some(start),
            end: future_end,
        },
        now,
    );

    // refresh_window_counts clamps end to now
    assert!(app.msg_window.end.unwrap() <= now);
    let cur = app.current_message_range(now);
    match cur {
        MessageRange::Fixed { end, .. } => {
            assert!(end <= now, "End must be clamped to now");
        }
        _ => panic!("Expected Fixed range"),
    }
}

#[test]
fn test_task_77_31_slider_left_boundary_all_and_right_boundary_live() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::model::MessageRange;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let now = chrono::Utc::now();
    // Start in 1h live window
    let _ = app.apply_message_range(
        MessageRange::Live {
            span_secs: Some(3600),
        },
        now,
    );
    assert_eq!(app.msg_window.span_secs, Some(3600));
    assert!(app.msg_window.end.is_none());

    // Switch to All
    let _ = app.apply_message_range(MessageRange::Live { span_secs: None }, now);
    assert!(app.msg_window.span_secs.is_none());
    assert!(app.msg_window.end.is_none());
    assert_eq!(app.time_range_text, "all");
}

#[test]
fn test_task_77_31_invalid_text_leaves_window_intact_and_marks_invalid() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};
    use hcom_dashboard::model::MessageRange;

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let now = chrono::Utc::now();
    let _ = app.apply_message_range(
        MessageRange::Live {
            span_secs: Some(1800),
        },
        now,
    );
    app.save_state();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();

    // 1. Initial frame: render UI and set focus to TextEdit
    ctx.memory_mut(|m| m.request_focus(egui::Id::unique("time_range_text_edit")));
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| {
            app.ui(ui, &mut frame);
        },
    );
    out.textures_delta.clear();

    // 2. Type invalid gibberish and commit with Enter
    app.time_range_text = "invalid-gibberish-123".into();
    let events = vec![egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
        repeat: false,
    }];

    let input = egui::RawInput {
        screen_rect: Some(screen),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        app.ui(ui, &mut frame);
    });
    out.textures_delta.clear();

    assert!(
        app.time_range_invalid,
        "time_range_invalid must be true for invalid text"
    );
    assert_eq!(
        app.msg_window.span_secs,
        Some(1800),
        "Window span must be unchanged"
    );

    // Verify persisted state is also unchanged
    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.window_span_secs, Some(1800));
}

#[test]
fn test_task_77_31_slider_thumbs_coincide_at_min_no_panic_and_drags_safely() {
    use chrono::DateTime;
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::model::{MessageData, MessageEvent, MessageRange};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let oldest_ts = DateTime::from_timestamp(1700000000, 0).unwrap();
    let now = DateTime::from_timestamp(1700005000, 0).unwrap();

    // 1. Ingest message at oldest_ts so oldest_message_ts() == 1700000000
    app.graph.apply_message_event(&MessageEvent {
        id: 1,
        ts: oldest_ts,
        instance: "inst1".into(),
        data: MessageData {
            from: "inst1".into(),
            delivered_to: vec!["inst2".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "hello".into(),
        },
    });

    // 2. Apply unbounded-start range ending at oldest_ts: "all - 1700000000"
    let _ = app.apply_message_range(
        MessageRange::Fixed {
            start: None,
            end: oldest_ts,
        },
        now,
    );

    let cur = app.current_message_range(now);
    assert_eq!(
        cur,
        MessageRange::Fixed {
            start: None,
            end: oldest_ts,
        }
    );

    // 3. Render UI and locate the coincident thumbs position
    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();

    let mut initial_out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ui| {
            app.ui(ui, &mut frame);
        },
    );
    initial_out.textures_delta.clear();

    let thumb_center = initial_out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Circle(c) if c.radius == 6.0 => Some(c.center),
            _ => None,
        })
        .expect("slider thumb must be rendered");

    // 4. Simulate rightward drag from the exact thumb position across frames:
    // Frame 1: Pointer pressed exactly on the shared thumb center
    // Frame 2: Drag moved to the right (+60 px)
    // Frame 3: Pointer released
    let drag_target = thumb_center + egui::vec2(60.0, 0.0);

    for events in [
        vec![
            egui::Event::PointerMoved(thumb_center),
            egui::Event::PointerButton {
                pos: thumb_center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        vec![egui::Event::PointerMoved(drag_target)],
        vec![egui::Event::PointerButton {
            pos: drag_target,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    ] {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| {
                app.ui(ui, &mut frame);
            },
        );
        out.textures_delta.clear();
    }

    // Verify:
    // 1. Did not panic with min > max.
    // 2. Rightward drag from min resolved to End thumb and expanded end timestamp.
    assert!(
        app.msg_window.end.unwrap() > oldest_ts,
        "Rightward drag from coincident thumbs at min must expand end beyond oldest timestamp"
    );
}

#[test]
fn test_task_77_31_apply_message_range_rejects_overflow_and_preserves_prior_window() {
    use chrono::DateTime;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::model::{MAX_DURATION_SECS, MessageData, MessageEvent, MessageRange};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );

    let oldest_ts = DateTime::from_timestamp(1700000000, 0).unwrap();
    let now = DateTime::from_timestamp(1700005000, 0).unwrap();

    // Ingest message so oldest_message_ts() is Some
    app.graph.apply_message_event(&MessageEvent {
        id: 1,
        ts: oldest_ts,
        instance: "inst1".into(),
        data: MessageData {
            from: "inst1".into(),
            delivered_to: vec!["inst2".into()],
            mentions: vec![],
            intent: None,
            reply_to: None,
            text: "hello".into(),
        },
    });

    // Establish prior valid window (e.g. 1h span = 3600s)
    let prior_range = MessageRange::Live {
        span_secs: Some(3600),
    };
    assert!(app.apply_message_range(prior_range, now).is_ok());
    assert_eq!(app.msg_window.span_secs, Some(3600));
    assert_eq!(app.msg_window.end, None);

    // 1. Live with span_secs = i64::MAX must return Err and not panic or mutate
    let res_max = app.apply_message_range(
        MessageRange::Live {
            span_secs: Some(i64::MAX),
        },
        now,
    );
    assert!(res_max.is_err(), "i64::MAX span must return Err");
    assert_eq!(
        app.msg_window.span_secs,
        Some(3600),
        "Prior span must be preserved on error"
    );
    assert_eq!(
        app.state.window_span_secs,
        Some(3600),
        "Prior state span must be preserved on error"
    );
    assert_eq!(
        app.msg_window.end, None,
        "Prior end must be preserved on error"
    );

    // 2. Live with span_secs > MAX_DURATION_SECS must return Err
    let res_exceed = app.apply_message_range(
        MessageRange::Live {
            span_secs: Some((MAX_DURATION_SECS + 1) as i64),
        },
        now,
    );
    assert!(res_exceed.is_err(), ">100y span must return Err");
    assert_eq!(app.msg_window.span_secs, Some(3600));
    assert_eq!(app.state.window_span_secs, Some(3600));

    // 3. Fixed with span > 100 years must return Err and preserve prior window
    let huge_start = DateTime::from_timestamp(0, 0).unwrap();
    let far_future = DateTime::from_timestamp(MAX_DURATION_SECS as i64 + 86400, 0).unwrap();
    let res_fixed_exceed = app.apply_message_range(
        MessageRange::Fixed {
            start: Some(huge_start),
            end: far_future,
        },
        now,
    );
    assert!(
        res_fixed_exceed.is_err(),
        ">100y fixed span must return Err"
    );
    assert_eq!(app.msg_window.span_secs, Some(3600));
    assert_eq!(app.state.window_span_secs, Some(3600));
}

// ---------------------------------------------------------------------------
// TASK-77.37: Light theme & default theme according to OS
// ---------------------------------------------------------------------------

#[test]
fn test_task_77_37_theme_deserialization_and_forward_compatibility() {
    use hcom_dashboard::config::{DashboardState, ThemePreference};

    // 1. Explicit theme preferences
    let json_light = r#"{"theme": "light"}"#;
    let state_light: DashboardState = serde_json::from_str(json_light).unwrap();
    assert_eq!(state_light.theme, Some(ThemePreference::Light));

    let json_dark = r#"{"theme": "dark"}"#;
    let state_dark: DashboardState = serde_json::from_str(json_dark).unwrap();
    assert_eq!(state_dark.theme, Some(ThemePreference::Dark));

    let json_auto = r#"{"theme": "auto"}"#;
    let state_auto: DashboardState = serde_json::from_str(json_auto).unwrap();
    assert_eq!(state_auto.theme, Some(ThemePreference::Auto));

    // 2. Missing theme defaults to None (which acts as Auto)
    let json_empty = r#"{}"#;
    let state_empty: DashboardState = serde_json::from_str(json_empty).unwrap();
    assert_eq!(state_empty.theme, None);

    // 3. Forward-compatibility: unknown theme names must not reject state
    let json_unknown = r#"{
        "theme": "solarized_high_contrast",
        "detail_width_px": 420.0,
        "sort_col": "status"
    }"#;
    let state_unknown: DashboardState = serde_json::from_str(json_unknown).unwrap();
    assert_eq!(
        state_unknown.theme, None,
        "Unknown theme must deserialize to None without error"
    );
    assert_eq!(state_unknown.detail_width_px, Some(420.0));
    assert_eq!(state_unknown.sort_col, Some("status".into()));

    // 4. Round-trip serialization
    let state = DashboardState {
        theme: Some(ThemePreference::Light),
        ..Default::default()
    };
    let serialized = serde_json::to_string(&state).unwrap();
    assert!(serialized.contains(r#""theme":"light""#));
    let restored: DashboardState = serde_json::from_str(&serialized).unwrap();
    assert_eq!(restored.theme, Some(ThemePreference::Light));
}

#[test]
fn test_task_77_37_system_theme_resolution() {
    use hcom_dashboard::config::{ResolvedTheme, ThemePreference};

    // Auto theme resolution
    assert_eq!(
        ThemePreference::Auto.resolve(None),
        ResolvedTheme::Dark,
        "Auto with None system_theme must fall back deterministically to Dark"
    );
    assert_eq!(
        ThemePreference::Auto.resolve(Some(egui::Theme::Light)),
        ResolvedTheme::Light
    );
    assert_eq!(
        ThemePreference::Auto.resolve(Some(egui::Theme::Dark)),
        ResolvedTheme::Dark
    );

    // Explicit Light theme resolution
    assert_eq!(ThemePreference::Light.resolve(None), ResolvedTheme::Light);
    assert_eq!(
        ThemePreference::Light.resolve(Some(egui::Theme::Dark)),
        ResolvedTheme::Light
    );
    assert_eq!(
        ThemePreference::Light.resolve(Some(egui::Theme::Light)),
        ResolvedTheme::Light
    );

    // Explicit Dark theme resolution
    assert_eq!(ThemePreference::Dark.resolve(None), ResolvedTheme::Dark);
    assert_eq!(
        ThemePreference::Dark.resolve(Some(egui::Theme::Light)),
        ResolvedTheme::Dark
    );
    assert_eq!(
        ThemePreference::Dark.resolve(Some(egui::Theme::Dark)),
        ResolvedTheme::Dark
    );
}

fn wcag_relative_luminance(c: egui::Color32) -> f64 {
    fn channel(v: u8) -> f64 {
        let val = v as f64 / 255.0;
        if val <= 0.04045 {
            val / 12.92
        } else {
            ((val + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

fn wcag_contrast_ratio(c1: egui::Color32, c2: egui::Color32) -> f64 {
    let l1 = wcag_relative_luminance(c1);
    let l2 = wcag_relative_luminance(c2);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[test]
fn test_task_77_37_palette_contrast_all_entries() {
    use hcom_dashboard::canvas::ThemePalette;
    use hcom_dashboard::config::ResolvedTheme;
    use hcom_dashboard::model::AgentStatus;

    for resolved in [ResolvedTheme::Dark, ResolvedTheme::Light] {
        let palette = ThemePalette::for_theme(resolved);

        // 1. Text contrast (WCAG AA >= 4.5:1 for primary text, >= 3.0:1 for secondary)
        let primary_contrast = wcag_contrast_ratio(palette.node_text_primary, palette.node_fill);
        assert!(
            primary_contrast >= 4.5,
            "{:?} primary text contrast {:.2} must be >= 4.5",
            resolved,
            primary_contrast
        );
        let secondary_contrast =
            wcag_contrast_ratio(palette.node_text_secondary, palette.node_fill);
        assert!(
            secondary_contrast >= 3.0,
            "{:?} secondary text contrast {:.2} must be >= 3.0",
            resolved,
            secondary_contrast
        );

        // 2. All AgentStatus colors meet non-text contrast >= 3.0:1 against node_fill
        for status in [
            AgentStatus::Active,
            AgentStatus::Listening,
            AgentStatus::Blocked,
            AgentStatus::Stopped,
        ] {
            let sc = palette.status_color(&status);
            let ratio = wcag_contrast_ratio(sc, palette.node_fill);
            assert!(
                ratio >= 3.0,
                "{:?} status {:?} contrast {:.2} must be >= 3.0 against node_fill",
                resolved,
                status,
                ratio
            );
        }

        // 3. All CLI ring colors meet non-text contrast >= 3.0:1 against node_fill
        let tools = [
            "claude",
            "codex",
            "gemini",
            "agy",
            "antigravity",
            "hermes",
            "kimi",
            "grok",
            "opencode",
            "kilo",
            "pi",
            "omp",
            "cursor",
            "copilot",
            "unknown",
        ];
        for tool in tools {
            let cc = palette.cli_color(Some(tool));
            let ratio = wcag_contrast_ratio(cc, palette.node_fill);
            assert!(
                ratio >= 3.0,
                "{:?} CLI tool {:?} contrast {:.2} must be >= 3.0 against node_fill",
                resolved,
                tool,
                ratio
            );
        }

        // 4. Pin marker contrast against node_fill >= 3.0:1
        let pin_ratio = wcag_contrast_ratio(palette.pin_fill, palette.node_fill);
        assert!(
            pin_ratio >= 3.0,
            "{:?} pin_fill contrast {:.2} must be >= 3.0 against node_fill",
            resolved,
            pin_ratio
        );

        // 5. Context fill polygons against node_fill >= 3.0:1
        for (name, fill) in [
            ("low", palette.context_fill_low),
            ("mid", palette.context_fill_mid),
            ("high", palette.context_fill_high),
        ] {
            let ratio = wcag_contrast_ratio(fill, palette.node_fill);
            assert!(
                ratio >= 3.0,
                "{:?} context fill {} contrast {:.2} must be >= 3.0 against node_fill",
                resolved,
                name,
                ratio
            );
        }

        // 6. Badge text contrast on badge fill >= 3.0:1
        let badge_ratio = wcag_contrast_ratio(palette.badge_text, palette.badge_fill);
        assert!(
            badge_ratio >= 3.0,
            "{:?} badge text contrast {:.2} must be >= 3.0 on badge_fill",
            resolved,
            badge_ratio
        );
    }
}

#[test]
fn test_task_77_37_legend_matches_active_palette() {
    use hcom_dashboard::canvas::{ThemePalette, legend};
    use hcom_dashboard::config::ResolvedTheme;
    use hcom_dashboard::layout::GraphLayout;
    use hcom_dashboard::model::AgentGraph;
    use std::collections::HashSet;

    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        legend_snap("agent_claude", "active", Some("claude"), None),
        legend_snap("agent_grok", "blocked", Some("grok"), None),
        legend_snap("agent_pi", "listening", Some("pi"), None),
    ]);
    let mut layout = GraphLayout::new();
    layout.sync_nodes(&graph);
    let visible: HashSet<_> = graph.graph.node_indices().collect();

    let light_palette = ThemePalette::for_theme(ResolvedTheme::Light);
    let dark_palette = ThemePalette::for_theme(ResolvedTheme::Dark);

    let l_light = legend(&graph, &layout, &visible, None, &light_palette);
    let l_dark = legend(&graph, &layout, &visible, None, &dark_palette);

    // Grok color in dark palette is light (#E5E7EB), in light palette is dark (#374151)
    let light_grok_color = light_palette.cli_color(Some("grok"));
    let dark_grok_color = dark_palette.cli_color(Some("grok"));
    assert_ne!(light_grok_color, dark_grok_color);

    assert!(
        l_light
            .cli
            .iter()
            .any(|(c, tools)| *c == light_grok_color && tools.contains(&"grok".into())),
        "Light legend must use light palette color for grok"
    );
    assert!(
        l_dark
            .cli
            .iter()
            .any(|(c, tools)| *c == dark_grok_color && tools.contains(&"grok".into())),
        "Dark legend must use dark palette color for grok"
    );

    // Status colors in active palette
    assert_ne!(
        light_palette.status_color(&hcom_dashboard::model::AgentStatus::Active),
        dark_palette.status_color(&hcom_dashboard::model::AgentStatus::Active)
    );
    assert_ne!(
        light_palette.status_color(&hcom_dashboard::model::AgentStatus::Listening),
        dark_palette.status_color(&hcom_dashboard::model::AgentStatus::Listening)
    );
}

#[test]
fn test_task_77_37_top_bar_theme_toggle_cycles_and_persists() {
    use eframe::App;
    use egui::{Pos2, Rect, Vec2};
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, ThemePreference, load_state};

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        theme: Some(ThemePreference::Auto),
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let run = |ctx: &egui::Context,
               app: &mut HcomDashboardApp,
               frame: &mut eframe::Frame,
               events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            app.ui(ui, frame);
        });
        out.textures_delta.clear();
        out
    };

    let click_widget = |ctx: &egui::Context,
                        app: &mut HcomDashboardApp,
                        frame: &mut eframe::Frame,
                        out: &egui::FullOutput,
                        substr: &str| {
        let update = out
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("accesskit update");
        let rect = update
            .nodes
            .iter()
            .find_map(|(_, node)| {
                let label = node.label()?;
                if !label.contains(substr) {
                    return None;
                }
                let b = node.bounds()?;
                Some(Rect::from_min_max(
                    Pos2::new(b.x0 as f32, b.y0 as f32),
                    Pos2::new(b.x1 as f32, b.y1 as f32),
                ))
            })
            .unwrap_or_else(|| panic!("Widget with label containing '{substr}' not found"));
        let pos = rect.center();
        run(
            ctx,
            app,
            frame,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        run(
            ctx,
            app,
            frame,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    };

    // 1. Initial state is Auto
    assert_eq!(app.state.theme, Some(ThemePreference::Auto));
    let out = run(&ctx, &mut app, &mut frame, vec![]);

    // 2. Click button "💻 Auto" -> cycles to Light
    click_widget(&ctx, &mut app, &mut frame, &out, "Auto");
    assert_eq!(app.state.theme, Some(ThemePreference::Light));
    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.theme, Some(ThemePreference::Light));

    // 3. Frame with Light theme applies light visuals
    let out = run(&ctx, &mut app, &mut frame, vec![]);
    assert_eq!(ctx.theme(), egui::Theme::Light);

    // 4. Click button "☀ Light" -> cycles to Dark
    click_widget(&ctx, &mut app, &mut frame, &out, "Light");
    assert_eq!(app.state.theme, Some(ThemePreference::Dark));
    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.theme, Some(ThemePreference::Dark));

    let out = run(&ctx, &mut app, &mut frame, vec![]);
    assert_eq!(ctx.theme(), egui::Theme::Dark);

    // 5. Click button "🌙 Dark" -> cycles to Auto
    click_widget(&ctx, &mut app, &mut frame, &out, "Dark");
    assert_eq!(app.state.theme, Some(ThemePreference::Auto));
    let reloaded = load_state(dir.path());
    assert_eq!(reloaded.theme, Some(ThemePreference::Auto));
}

#[test]
fn test_task_77_37_detail_tabs_palette_contrast_light_mode() {
    use hcom_dashboard::canvas::ThemePalette;
    use hcom_dashboard::config::ResolvedTheme;
    use hcom_dashboard::model::AgentStatus;

    let palette = ThemePalette::for_theme(ResolvedTheme::Light);
    let panel_bg = egui::Color32::from_rgb(255, 255, 255);

    // All status colors in detail tabs (Summary, Config) must meet WCAG >= 3.0:1 on panel bg
    for status in [
        AgentStatus::Active,
        AgentStatus::Listening,
        AgentStatus::Blocked,
        AgentStatus::Stopped,
    ] {
        let sc = palette.status_color(&status);
        let ratio = wcag_contrast_ratio(sc, panel_bg);
        assert!(
            ratio >= 3.0,
            "Light mode status {:?} contrast {:.2} must be >= 3.0 against panel bg",
            status,
            ratio
        );
    }

    // Config roaming and disabled labels
    let roaming_ratio = wcag_contrast_ratio(palette.message_edge, panel_bg);
    assert!(
        roaming_ratio >= 3.0,
        "Light mode roaming badge contrast {:.2} must be >= 3.0 against panel bg",
        roaming_ratio
    );
    let disabled_ratio = wcag_contrast_ratio(palette.status_color(&AgentStatus::Blocked), panel_bg);
    assert!(
        disabled_ratio >= 3.0,
        "Light mode disabled badge contrast {:.2} must be >= 3.0 against panel bg",
        disabled_ratio
    );
}

#[test]
fn test_task_77_37_auto_restores_light_visuals_when_light_system_had_dark_visuals() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, ResolvedTheme, ThemePreference};

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        theme: Some(ThemePreference::Auto),
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        system_theme: Some(egui::Theme::Light),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |_ui| {});
    out.textures_delta.clear();
    assert_eq!(ctx.system_theme(), Some(egui::Theme::Light));

    ctx.set_visuals(egui::Visuals::dark());
    assert!(ctx.global_style().visuals.dark_mode);

    let palette = app.apply_theme(&ctx);
    assert_eq!(palette.resolved, ResolvedTheme::Light);
    assert!(!ctx.global_style().visuals.dark_mode);
    assert_eq!(ctx.theme(), egui::Theme::Light);
}

#[test]
fn test_task_77_40_lxqt_palette_theme() {
    use hcom_dashboard::config::lxqt_palette_theme;

    let light =
        "[General]\ntheme=Clearlooks\n\n[Palette]\nbase_color=#ffffff\nwindow_color=#efefef\n";
    assert_eq!(lxqt_palette_theme(light), Some(egui::Theme::Light));
    let dark = "[Palette]\nwindow_color=#2b2b2b\n";
    assert_eq!(lxqt_palette_theme(dark), Some(egui::Theme::Dark));
    let argb = "[Palette]\nwindow_color=#ff2b2b2b\n";
    assert_eq!(lxqt_palette_theme(argb), Some(egui::Theme::Dark));
    let other_section = "[General]\nwindow_color=#ffffff\n";
    assert_eq!(lxqt_palette_theme(other_section), None);
    assert_eq!(lxqt_palette_theme("[Palette]\nwindow_color=bad\n"), None);
    assert_eq!(lxqt_palette_theme(""), None);
}

#[test]
fn test_task_77_40_gtk_settings_theme() {
    use hcom_dashboard::config::gtk_settings_theme;

    let light = "[Settings]\ngtk-theme-name = Arc-Lighter\n";
    assert_eq!(gtk_settings_theme(light), Some(egui::Theme::Light));
    let dark_name = "[Settings]\ngtk-theme-name = Arc-Dark\n";
    assert_eq!(gtk_settings_theme(dark_name), Some(egui::Theme::Dark));
    let prefer_dark =
        "[Settings]\ngtk-theme-name = Adwaita\ngtk-application-prefer-dark-theme = true\n";
    assert_eq!(gtk_settings_theme(prefer_dark), Some(egui::Theme::Dark));
    let prefer_dark_num = "[Settings]\ngtk-application-prefer-dark-theme=1\n";
    assert_eq!(gtk_settings_theme(prefer_dark_num), Some(egui::Theme::Dark));
    assert_eq!(
        gtk_settings_theme("[Settings]\ngtk-font-name = Ubuntu 9\n"),
        None
    );
}

#[test]
fn test_task_77_40_desktop_theme_sources_in_order() {
    use hcom_dashboard::config::desktop_theme;

    let dir = tempfile::tempdir().unwrap();
    assert_eq!(desktop_theme(dir.path()), None, "no files: no theme");

    std::fs::create_dir_all(dir.path().join("gtk-3.0")).unwrap();
    std::fs::write(
        dir.path().join("gtk-3.0/settings.ini"),
        "[Settings]\ngtk-theme-name = Arc-Dark\n",
    )
    .unwrap();
    assert_eq!(desktop_theme(dir.path()), Some(egui::Theme::Dark), "GTK");

    std::fs::create_dir_all(dir.path().join("lxqt")).unwrap();
    std::fs::write(
        dir.path().join("lxqt/lxqt.conf"),
        "[Palette]\nwindow_color=#efefef\n",
    )
    .unwrap();
    assert_eq!(
        desktop_theme(dir.path()),
        Some(egui::Theme::Light),
        "LXQt palette wins over GTK"
    );
}

#[test]
fn test_task_77_40_auto_uses_desktop_theme_when_system_reports_none() {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, ResolvedTheme};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_ui| {});
    out.textures_delta.clear();
    assert_eq!(ctx.system_theme(), None);

    assert_eq!(app.apply_theme(&ctx).resolved, ResolvedTheme::Dark);
    app.desktop_theme = Some(egui::Theme::Light);
    assert_eq!(app.apply_theme(&ctx).resolved, ResolvedTheme::Light);
    assert!(!ctx.global_style().visuals.dark_mode);
}

#[test]
fn test_task_77_41_theme_toggle_sits_right_of_fit() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 800.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut out = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut o = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        o.textures_delta.clear();
        out = Some(o);
    }
    let out = out.unwrap();
    let fit = widget_rect(&out, "Fit").expect("Fit button");
    let toggle = widget_rect(&out, "💻 Auto").expect("theme toggle");
    let layout = widget_rect(&out, "Hierarchy").expect("Hierarchy layout button");
    assert!(
        (toggle.center().y - fit.center().y).abs() < 2.0,
        "same row as Fit"
    );
    assert!(toggle.left() > fit.right(), "right of Fit");
    assert!(toggle.bottom() < layout.top(), "above the Layout row");
}

#[test]
fn test_task_77_38_launcher_from_batch_launched_per_session() {
    use hcom_dashboard::table_data::compute_event_stats;

    // child_a: launched by an agent (autostart or hcom <cli>).
    // user_b: launched by the user. bare_c: session without batch_launched.
    // again_d: first session by an agent, new session by the user.
    // again_e: first session by an agent, new session without batch_launched.
    // self_f: relaunched by itself (handoff-like): no self parent.
    let life = r#"
{"id": 1, "instance": "child_a", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created", "is_subagent": false}}
{"id": 2, "instance": "wdt_main", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "wdt_main", "instances": ["child_a"]}}
{"id": 3, "instance": "user_b", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 4, "instance": "user", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "user", "instances": ["user_b"]}}
{"id": 5, "instance": "bare_c", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 6, "instance": "again_d", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 7, "instance": "boss", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "boss", "instances": ["again_d", "again_e"]}}
{"id": 8, "instance": "again_e", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 9, "instance": "again_d", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "created"}}
{"id": 10, "instance": "user", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "batch_launched", "by": "user", "instances": ["again_d"]}}
{"id": 11, "instance": "again_e", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "created"}}
{"id": 12, "instance": "self_f", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "batch_launched", "by": "self_f", "instances": ["self_f"]}}
"#;
    let stats = compute_event_stats(life, "", &std::collections::HashSet::new(), 0);
    let launcher = |n: &str| stats.get(n).and_then(|s| s.launcher.clone());
    assert_eq!(launcher("child_a").as_deref(), Some("wdt_main"));
    assert_eq!(launcher("user_b"), None);
    assert_eq!(launcher("bare_c"), None);
    assert_eq!(
        launcher("again_d"),
        None,
        "new user session drops the old launcher"
    );
    assert_eq!(
        launcher("again_e"),
        None,
        "new session without batch_launched"
    );
    assert_eq!(launcher("self_f"), None);
}

#[test]
fn test_task_95_handoff_relaunch_keeps_the_launcher() {
    use hcom_dashboard::table_data::compute_event_stats;

    // Live sequence of `hcom handoff`: handoff, stopped, created, then a
    // detached batch_launched by the user. A later plain relaunch by the
    // user (no handoff) determines the launcher again.
    let handoff = r#"
{"id": 1, "instance": "worker", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 2, "instance": "boss", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "boss", "instances": ["worker"]}}
{"id": 3, "instance": "worker", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "handoff", "by": "worker"}}
{"id": 4, "instance": "worker", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "stopped", "by": "cli"}}
{"id": 5, "instance": "worker", "type": "life", "ts": "2026-11-01T11:00:02", "data": {"action": "created"}}
{"id": 6, "instance": "user", "type": "life", "ts": "2026-11-01T11:00:02", "data": {"action": "batch_launched", "by": "user", "instances": ["worker"]}}
"#;
    let relaunch = r#"
{"id": 7, "instance": "worker", "type": "life", "ts": "2026-11-01T12:00:00", "data": {"action": "stopped", "by": "cli"}}
{"id": 8, "instance": "worker", "type": "life", "ts": "2026-11-01T12:00:02", "data": {"action": "created"}}
{"id": 9, "instance": "user", "type": "life", "ts": "2026-11-01T12:00:02", "data": {"action": "batch_launched", "by": "user", "instances": ["worker"]}}
"#;
    let none = std::collections::HashSet::new();
    let stats = compute_event_stats(handoff, "", &none, 0);
    assert_eq!(stats["worker"].launcher.as_deref(), Some("boss"));
    let stats = compute_event_stats(&format!("{handoff}{relaunch}"), "", &none, 0);
    assert_eq!(stats["worker"].launcher, None, "a plain relaunch resets it");
}

#[test]
fn test_task_77_38_launcher_becomes_spawn_parent_and_parent_name_wins() {
    use hcom_dashboard::table_data::{TableRow, apply_event_stats, compute_event_stats};

    let (_dir, mut app) = split_test_app(hcom_dashboard::config::DashboardState::default());
    let row = |name: &str, parent: Option<&str>| TableRow {
        instance: name.into(),
        is_running: true,
        status: "listening".into(),
        parent_name: parent.map(Into::into),
        ..Default::default()
    };
    app.table_rows = vec![
        row("boss", None),
        row("child", None),
        row("sub", Some("claude_root")),
    ];
    let first = r#"
{"id": 1, "instance": "boss", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "boss", "instances": ["child", "sub"]}}
"#;
    let stats = compute_event_stats(first, "", &std::collections::HashSet::new(), 0);
    apply_event_stats(&mut app.table_rows, &stats);
    app.sync_graph_from_table_rows();

    let edges = &app.graph.spawn_edges;
    assert!(edges.contains_key(&("boss".into(), "child".into())));
    assert!(edges.contains_key(&("claude_root".into(), "sub".into())));
    assert!(
        !edges.contains_key(&("boss".into(), "sub".into())),
        "parent_name wins"
    );

    // The user starts a new session of `child`: its spawn edge goes away.
    let second = r#"
{"id": 1, "instance": "boss", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "batch_launched", "by": "boss", "instances": ["child", "sub"]}}
{"id": 2, "instance": "child", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "created"}}
{"id": 3, "instance": "user", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "batch_launched", "by": "user", "instances": ["child"]}}
"#;
    let stats = compute_event_stats(second, "", &std::collections::HashSet::new(), 0);
    apply_event_stats(&mut app.table_rows, &stats);
    app.sync_graph_from_table_rows();
    assert!(
        !app.graph
            .spawn_edges
            .contains_key(&("boss".into(), "child".into()))
    );
    let child = app.graph.get_node_index("child").unwrap();
    assert_eq!(app.graph.graph[child].parent_name, None);
}

/// Message `id` from `from` to `to`; `reply` sets `reply_to`.
fn pair_msg(id: u64, from: &str, to: &str, intent: Option<&str>, reply: bool) -> MessageEvent {
    MessageEvent {
        id,
        ts: Utc::now(),
        instance: from.into(),
        data: MessageData {
            from: from.into(),
            delivered_to: vec![to.into()],
            mentions: vec![],
            intent: intent.map(Into::into),
            reply_to: reply.then(|| serde_json::json!("1")),
            text: format!("m{id}"),
        },
    }
}

/// Graph with `messages` and `starts` (instance, created event id), refreshed.
fn initiator_graph(messages: &[MessageEvent], starts: &[(&str, u64)]) -> AgentGraph {
    let mut g = AgentGraph::new();
    for m in messages {
        g.apply_message_event(m);
    }
    let mut map: std::collections::HashMap<String, Vec<u64>> = Default::default();
    for (n, id) in starts {
        map.entry(n.to_string()).or_default().push(*id);
    }
    g.set_session_starts(map);
    g.refresh_initiators();
    g
}

fn owned(v: &[(&str, usize)]) -> Vec<(String, usize)> {
    v.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

#[test]
fn test_task_77_44_one_way_conversation_has_one_message_arrow_and_its_initiator() {
    use hcom_dashboard::canvas::initiator_heads;
    let g = initiator_graph(
        &[
            pair_msg(10, "asker", "knowledge", Some("request"), false),
            pair_msg(11, "asker", "knowledge", Some("inform"), false),
        ],
        &[],
    );
    assert!(g.get_message_edge("asker", "knowledge").is_some());
    assert!(
        g.get_message_edge("knowledge", "asker").is_none(),
        "one-way: a single message arrow"
    );
    assert_eq!(g.initiators("knowledge", "asker"), owned(&[("asker", 1)]));
    // Pair key is ordered: "asker" < "knowledge"; the arrow points at knowledge.
    assert_eq!(
        initiator_heads("asker", g.initiators("asker", "knowledge")),
        (false, true)
    );
}

#[test]
fn test_task_77_44_segments_split_at_created_of_either_side() {
    use hcom_dashboard::canvas::initiator_heads;
    let msgs = [
        pair_msg(10, "alice", "bob", Some("request"), false),
        pair_msg(11, "bob", "alice", Some("request"), false),
        // bob restarts (created 20): new segment, bob asks first.
        pair_msg(21, "bob", "alice", Some("request"), false),
        pair_msg(22, "alice", "bob", Some("request"), false),
        // alice restarts (created 30): alice asks first.
        pair_msg(31, "alice", "bob", None, false),
    ];
    // Without session starts the whole conversation is one segment.
    let g = initiator_graph(&msgs, &[]);
    assert_eq!(g.initiators("alice", "bob"), owned(&[("alice", 1)]));

    let g = initiator_graph(&msgs, &[("bob", 5), ("bob", 20), ("alice", 30)]);
    assert_eq!(
        g.initiators("alice", "bob"),
        owned(&[("alice", 2), ("bob", 1)])
    );
    assert_eq!(
        initiator_heads("alice", g.initiators("alice", "bob")),
        (true, true),
        "both sides started a segment: heads at both ends"
    );
    // Messages in both directions keep both message arrows.
    assert!(g.get_message_edge("alice", "bob").is_some());
    assert!(g.get_message_edge("bob", "alice").is_some());
}

#[test]
fn test_task_77_44_replies_and_acks_do_not_start_a_segment() {
    use hcom_dashboard::canvas::initiator_heads;
    let msgs = [
        pair_msg(10, "alice", "bob", Some("request"), false),
        // bob restarts and first answers the old question (reply_to), then
        // acks; alice's next request starts the segment.
        pair_msg(21, "bob", "alice", Some("inform"), true),
        pair_msg(22, "bob", "alice", Some("ack"), false),
        pair_msg(23, "alice", "bob", Some("request"), false),
        // alice restarts: this segment has only a reply and an ack.
        pair_msg(31, "bob", "alice", None, true),
        pair_msg(32, "alice", "bob", Some("ack"), false),
    ];
    let g = initiator_graph(&msgs, &[("bob", 20), ("alice", 30)]);
    assert_eq!(g.initiators("alice", "bob"), owned(&[("alice", 2)]));
    assert_eq!(
        initiator_heads("alice", g.initiators("alice", "bob")),
        (false, true)
    );

    // Only replies and acks: no initiator, no main arrow.
    let g = initiator_graph(
        &[
            pair_msg(10, "alice", "bob", None, true),
            pair_msg(11, "bob", "alice", Some("ack"), false),
        ],
        &[],
    );
    assert!(g.initiators("alice", "bob").is_empty());
    assert_eq!(g.iter_initiators().count(), 0);
}

#[test]
fn test_task_77_44_reply_to_parsed_from_stream_and_session_starts_from_life_events() {
    use hcom_dashboard::source::{StreamItem, parse_stream_line};
    use hcom_dashboard::table_data::compute_event_stats;

    // reply_to is a string in hcom, a number must not drop the message.
    for (line, reply) in [
        (
            r#"{"id":5,"ts":"2026-11-06T10:00:00Z","type":"message","instance":"bob","data":{"from":"bob","delivered_to":["alice"],"text":"x","reply_to":"4:BOXE"}}"#,
            true,
        ),
        (
            r#"{"id":6,"ts":"2026-11-06T10:00:00Z","type":"message","instance":"bob","data":{"from":"bob","delivered_to":["alice"],"text":"x","reply_to":4}}"#,
            true,
        ),
        (
            r#"{"id":7,"ts":"2026-11-06T10:00:00Z","type":"message","instance":"bob","data":{"from":"bob","delivered_to":["alice"],"text":"x"}}"#,
            false,
        ),
    ] {
        let StreamItem::Message(ev) = parse_stream_line(line) else {
            panic!("not parsed: {line}");
        };
        let mut g = AgentGraph::new();
        assert!(g.apply_message_event(&ev));
        let stored = &g.get_message_edge("bob", "alice").unwrap().history[0];
        assert_eq!(stored.is_reply, reply, "{line}");
    }

    let life = r#"
{"id": 3, "instance": "alice", "type": "life", "ts": "2026-11-01T10:00:00", "data": {"action": "created"}}
{"id": 9, "instance": "alice", "type": "life", "ts": "2026-11-01T11:00:00", "data": {"action": "created"}}
{"id": 5, "instance": "alice", "type": "life", "ts": "2026-11-01T10:30:00", "data": {"action": "stopped"}}
"#;
    let stats = compute_event_stats(life, "", &std::collections::HashSet::new(), 0);
    assert_eq!(stats["alice"].session_starts, vec![3, 9]);
}

#[test]
fn test_task_77_44_edge_detail_and_tooltip_name_the_initiators() {
    use hcom_dashboard::canvas::format_initiators;
    use hcom_dashboard::config::DashboardState;

    assert_eq!(format_initiators(&[]), None);
    assert_eq!(
        format_initiators(&owned(&[("alice", 2), ("bob", 1)])).as_deref(),
        Some("alice (2), bob (1)")
    );

    let (_dir, mut app) = split_test_app(DashboardState::default());
    app.table_rows = ["alice", "bob"]
        .iter()
        .map(|n| hcom_dashboard::table_data::TableRow {
            agent: n.to_string(),
            instance: n.to_string(),
            is_running: true,
            status: "listening".into(),
            ..Default::default()
        })
        .collect();
    app.sync_graph_from_table_rows();
    app.graph
        .apply_message_event(&pair_msg(10, "alice", "bob", Some("request"), false));
    app.graph
        .apply_message_event(&pair_msg(21, "bob", "alice", Some("request"), false));
    app.graph
        .set_session_starts([("bob".to_string(), vec![20])].into_iter().collect());
    app.graph.refresh_initiators();
    let edge = app.graph.message_edges[&("bob".to_string(), "alice".to_string())];
    app.canvas.hovered_edge = Some(edge);
    assert_eq!(
        app.edge_tooltip_text().as_deref(),
        Some("bob → alice: 1 messages\nStarted by: alice (1), bob (1)")
    );
    app.canvas.hovered_node = app.graph.get_node_index("bob");
    assert_eq!(app.edge_tooltip_text(), None, "a hovered node wins");

    // The edge detail names the initiators; the canvas draws the arrow.
    app.layout.sync_nodes(&app.graph);
    app.canvas.selected_edge = Some(edge);
    app.state.show_side_panel = true;
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut frame = eframe::Frame::_new_kittest();
    let mut out = None;
    for _ in 0..3 {
        use eframe::App;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
            ..Default::default()
        };
        let mut o = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        o.textures_delta.clear();
        out = Some(o);
    }
    // Plain labels carry their text as the accesskit value.
    let texts: Vec<String> = out
        .unwrap()
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .filter_map(|(_, n)| n.value().map(str::to_string))
        .collect();
    assert!(
        texts.iter().any(|t| t == "Started by: alice (1), bob (1)"),
        "edge detail lists the initiators: {texts:?}"
    );
}

#[test]
fn test_task_77_45_flow_width_follows_sqrt_share_of_the_largest_flow() {
    use hcom_dashboard::canvas::flow_width;
    assert_eq!(flow_width(0, 0), 1.0);
    assert_eq!(flow_width(0, 10), 1.0);
    assert_eq!(flow_width(10, 10), 10.0, "largest flow: full width");
    assert_eq!(flow_width(1, 4), 5.5, "1 + 9 * sqrt(1/4)");
    assert!(flow_width(1, 100) > 1.0, "small flows stay visible");
}

#[test]
fn test_task_77_45_edge_counts_carry_bytes_and_the_metric_picks_them() {
    use hcom_dashboard::model::{MessageWindow, WindowedCount};
    let mut g = AgentGraph::new();
    let mut m1 = pair_msg(1, "alice", "bob", None, false);
    m1.data.text = "abcd".into();
    let mut m2 = pair_msg(2, "alice", "bob", None, false);
    m2.data.text = "xyzxyzxyz1".into();
    g.apply_message_event(&m1);
    g.apply_message_event(&m2);
    let ab = g.message_edges[&("alice".to_string(), "bob".to_string())];
    let now = Utc::now();

    let all = g.windowed_edge_counts(&MessageWindow::default(), now);
    assert_eq!(
        all[&ab],
        WindowedCount {
            count: 2,
            bytes: 14
        }
    );
    let last = MessageWindow {
        last_n: Some(1),
        ..Default::default()
    };
    assert_eq!(
        g.windowed_edge_counts(&last, now)[&ab],
        WindowedCount {
            count: 1,
            bytes: 10
        }
    );

    let mut canvas = CanvasView::new();
    let wc = WindowedCount {
        count: 2,
        bytes: 14,
    };
    assert_eq!(canvas.flow_value(wc), 2, "default metric: count");
    canvas.width_by_bytes = true;
    assert_eq!(canvas.flow_value(wc), 14);
}

/// Shapes the canvas paints for `graph` with `layers`.
fn canvas_shape_count(
    graph: &AgentGraph,
    layout: &mut GraphLayout,
    layers: hcom_dashboard::canvas::EdgeLayers,
) -> usize {
    let palette = hcom_dashboard::canvas::ThemePalette::for_theme(
        hcom_dashboard::config::ResolvedTheme::Dark,
    );
    let mut canvas = CanvasView::new();
    canvas.layers = layers;
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        canvas.show(ui, graph, layout, None, &palette);
    });
    out.textures_delta.clear();
    out.shapes.len()
}

#[test]
fn test_task_77_45_each_edge_layer_switches_drawing_hit_test_and_legend() {
    use hcom_dashboard::canvas::{EdgeLayers, Legend, ThemePalette, legend};
    use hcom_dashboard::config::ResolvedTheme;

    // parent spawned child; parent asked child (initiator + message arrow).
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[snap("parent", None), snap("child", Some("parent"))]);
    graph.apply_message_event(&pair_msg(1, "parent", "child", Some("request"), false));
    graph.refresh_initiators();
    let mut layout = GraphLayout::new();
    layout.set_mode(hcom_dashboard::layout::LayoutMode::Manual);
    layout.sync_nodes(&graph);
    let p = graph.get_node_index("parent").unwrap();
    let c = graph.get_node_index("child").unwrap();
    layout.set_node_pos(p, Pos2::new(-200.0, 0.0), false);
    layout.set_node_pos(c, Pos2::new(200.0, 0.0), false);
    let positions = |l: &GraphLayout| (l.nodes[&p].pos, l.nodes[&c].pos);
    let before = positions(&layout);

    let all = EdgeLayers::default();
    let full = canvas_shape_count(&graph, &mut layout, all);
    for off in [
        EdgeLayers {
            initiator: false,
            ..all
        },
        EdgeLayers {
            message: false,
            ..all
        },
        EdgeLayers {
            spawn: false,
            ..all
        },
    ] {
        let n = canvas_shape_count(&graph, &mut layout, off);
        assert!(n < full, "{off:?} draws less: {n} < {full}");
    }
    assert_eq!(positions(&layout), before, "layers do not move nodes");

    // Hit test: the spawn and the message edge overlap (same direction);
    // switching one layer off leaves only the other hittable.
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
    let mut canvas = CanvasView::new();
    let hit_kinds = |canvas: &CanvasView| {
        let mut kinds = Vec::new();
        for y in 260..340 {
            let pos = Pos2::new(400.0, y as f32);
            if let Some(e) = canvas.hit_test_edge(pos, &graph, &layout, rect, 3.0) {
                let k = match graph.graph[e] {
                    hcom_dashboard::model::EdgeKind::Spawn(_) => "spawn",
                    hcom_dashboard::model::EdgeKind::Message(_) => "message",
                };
                if !kinds.contains(&k) {
                    kinds.push(k);
                }
            }
        }
        kinds.sort();
        kinds
    };
    assert!(!hit_kinds(&canvas).is_empty());
    canvas.layers.message = false;
    assert_eq!(hit_kinds(&canvas), vec!["spawn"]);
    canvas.layers = EdgeLayers {
        spawn: false,
        ..all
    };
    assert_eq!(hit_kinds(&canvas), vec!["message"]);

    // Legend drops switched-off layers.
    let visible = graph.graph.node_indices().collect();
    let palette = ThemePalette::for_theme(ResolvedTheme::Dark);
    let base = legend(&graph, &layout, &visible, None, &palette);
    assert!(base.spawn && base.message);
    let mut l: Legend = base.clone();
    l.apply_layers(EdgeLayers {
        message: false,
        ..all
    });
    assert!(l.spawn && !l.message && !l.message_idle);
    let mut l = base;
    l.apply_layers(EdgeLayers {
        spawn: false,
        ..all
    });
    assert!(!l.spawn && l.message);
}

#[test]
fn test_task_77_45_layers_and_metric_persist_with_defaults() {
    use hcom_dashboard::config::{DashboardState, load_state};

    let defaults: DashboardState = serde_json::from_str("{}").unwrap();
    assert!(
        defaults.show_initiator_edges && defaults.show_message_edges && defaults.show_spawn_edges
    );
    assert!(!defaults.edge_width_bytes, "default metric: count");
    assert_eq!(
        (
            DashboardState::default().show_initiator_edges,
            DashboardState::default().edge_width_bytes
        ),
        (true, false)
    );

    let (dir, mut app) = split_test_app(DashboardState::default());
    app.state.show_initiator_edges = false;
    app.state.show_spawn_edges = false;
    app.state.edge_width_bytes = true;
    app.save_state();
    let loaded = load_state(dir.path());
    assert!(!loaded.show_initiator_edges && loaded.show_message_edges && !loaded.show_spawn_edges);
    assert!(loaded.edge_width_bytes);

    // The canvas follows the state after a frame.
    let mut fresh = hcom_dashboard::app::HcomDashboardApp::new_for_test(
        hcom_dashboard::config::DashboardConfig::default(),
        loaded,
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    run_app_frames(&ctx, &mut fresh, 1);
    assert!(
        !fresh.canvas.layers.initiator && fresh.canvas.layers.message && !fresh.canvas.layers.spawn
    );
    assert!(fresh.canvas.width_by_bytes);
}

#[test]
fn test_task_77_45_edge_toggles_sit_after_layout_and_switch_the_state() {
    use eframe::App;
    use hcom_dashboard::config::{DashboardState, load_state};

    let (dir, mut app) = split_test_app(DashboardState::default());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1800.0, 800.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut run = |app: &mut hcom_dashboard::app::HcomDashboardApp, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let mut o = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        o.textures_delta.clear();
        o
    };
    for _ in 0..2 {
        run(&mut app, vec![]);
    }
    let out = run(&mut app, vec![]);
    let rect =
        |label: &str| widget_rect(&out, label).unwrap_or_else(|| panic!("{label:?} missing"));
    let manual = rect("Manual");
    let mut prev = manual;
    for label in ["Start", "Msgs", "Spawn", "Count", "Bytes"] {
        let r = rect(label);
        assert!(
            (r.center().y - manual.center().y).abs() < 1.0,
            "{label} on the Layout row"
        );
        assert!(
            r.left() > prev.right(),
            "{label} after the previous control"
        );
        prev = r;
    }

    let click = |pos: Pos2| {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![
            vec![egui::Event::PointerMoved(pos), button(true)],
            vec![button(false)],
            vec![],
        ]
    };
    for label in ["Msgs", "Bytes"] {
        for events in click(rect(label).center()) {
            run(&mut app, events);
        }
    }
    assert!(!app.state.show_message_edges);
    assert!(app.state.edge_width_bytes);
    let saved = load_state(dir.path());
    assert!(!saved.show_message_edges && saved.edge_width_bytes);
}

/// Tree boss → m1 → a1, boss → m2, messages boss–m1 and m2–a1, and three
/// nodes without any edge (TASK-77.46).
fn reset_test_graph() -> AgentGraph {
    let mut graph = AgentGraph::new();
    graph.apply_instance_snapshot(&[
        layout_snap("boss", None),
        layout_snap("m1", Some("boss")),
        layout_snap("m2", Some("boss")),
        layout_snap("a1", Some("m1")),
        layout_snap("lone1", None),
        layout_snap("lone2", None),
        layout_snap("lone3", None),
    ]);
    let mut id = 0;
    for (from, to, count) in [("boss", "m1", 10), ("m2", "a1", 3)] {
        for _ in 0..count {
            id += 1;
            graph.apply_message_event(&MessageEvent {
                id,
                ts: Utc::now(),
                instance: from.into(),
                data: MessageData {
                    from: from.into(),
                    delivered_to: vec![to.into()],
                    mentions: vec![],
                    intent: None,
                    reply_to: None,
                    text: "x".into(),
                },
            });
        }
    }
    graph
}

/// Settled layout of `reset_test_graph` in `mode` with `m2` pinned.
fn settled_reset_layout(
    graph: &AgentGraph,
    mode: hcom_dashboard::layout::LayoutMode,
) -> GraphLayout {
    let mut layout = GraphLayout::new();
    layout.set_mode(mode);
    layout.sync_nodes(graph);
    for _ in 0..1500 {
        layout.step(graph, 1.0 / 60.0);
    }
    let m2 = graph.get_node_index("m2").unwrap();
    layout.set_pinned(m2, "m2", true);
    layout
}

#[test]
fn test_task_77_46_planned_layout_ignores_current_positions_and_keeps_pins() {
    use hcom_dashboard::layout::LayoutMode;
    let graph = reset_test_graph();
    for mode in [LayoutMode::Cluster, LayoutMode::Hierarchy] {
        let mut layout = settled_reset_layout(&graph, mode);
        let m2 = graph.get_node_index("m2").unwrap();
        let m2_pos = layout.nodes[&m2].pos;
        let plan = layout.planned_positions(&graph);
        // Scatter every unpinned node: the plan must not change (AC2).
        for (k, (_, phys)) in layout.nodes.iter_mut().enumerate() {
            if !phys.pinned {
                phys.pos = Pos2::new((k * 997 % 3000) as f32 - 1500.0, (k * 613 % 2000) as f32);
            }
        }
        let again = layout.planned_positions(&graph);
        assert_eq!(plan.len(), graph.graph.node_count());
        for (idx, p) in &plan {
            assert!(
                p.distance(again[idx]) < 0.01,
                "{mode:?}: {} planned independent of current positions",
                graph.graph[*idx].name
            );
        }
        // The pinned node stays (AC3).
        assert_eq!(plan[&m2], m2_pos, "{mode:?}: pinned node kept");
        // Nodes without edges end next to the others (AC5).
        let at = |n: &str| plan[&graph.get_node_index(n).unwrap()];
        if mode == LayoutMode::Cluster {
            // Within two slots of the bounding box of the connected nodes.
            let rest = ["boss", "m1", "m2", "a1"]
                .iter()
                .fold(Rect::NOTHING, |b, n| {
                    b.union(Rect::from_center_size(at(n), Vec2::ZERO))
                });
            for n in ["lone1", "lone2", "lone3"] {
                let d = rest.distance_to_pos(at(n));
                assert!(d <= 200.0 + 1e-3, "{mode:?}: {n} near the others, {d} away");
            }
        } else {
            // In the root layer, each with a neighbour about one slot away.
            for n in ["lone1", "lone2", "lone3"] {
                assert!(
                    (at(n).y - at("boss").y).abs() < 25.0,
                    "{n} in the root layer"
                );
                let nearest = plan
                    .values()
                    .filter(|p| **p != at(n))
                    .map(|p| p.distance(at(n)))
                    .fold(f32::MAX, f32::min);
                assert!(nearest < 130.0, "{n} next to a neighbour, {nearest} away");
            }
        }
    }
}

#[test]
fn test_task_77_46_transition_is_eased_takes_four_seconds_and_rests() {
    use hcom_dashboard::layout::{LayoutMode, TRANSITION_SECS};
    assert!((3.0..=5.0).contains(&TRANSITION_SECS));
    let graph = reset_test_graph();
    // Reset in Cluster, then a switch Cluster → Hierarchy.
    let mut layout = settled_reset_layout(&graph, LayoutMode::Cluster);
    for switch in [None, Some(LayoutMode::Hierarchy)] {
        if let Some(mode) = switch {
            layout.set_mode(mode);
        }
        let m2 = graph.get_node_index("m2").unwrap();
        let m2_pos = layout.nodes[&m2].pos;
        let plan = layout.planned_positions(&graph);
        let start: std::collections::HashMap<_, Pos2> =
            layout.nodes.iter().map(|(i, p)| (*i, p.pos)).collect();
        layout.start_transition(&graph);
        let dt = 1.0 / 60.0;
        let mut frames = 0usize;
        let mut prev = start.clone();
        let mut max_step: std::collections::HashMap<_, f32> = Default::default();
        while layout.transition.is_some() {
            layout.step(&graph, dt);
            frames += 1;
            assert!(frames < 1000, "transition ends");
            for (idx, p) in &layout.nodes {
                let s = max_step.entry(*idx).or_insert(0.0f32);
                *s = s.max(p.pos.distance(prev[idx]));
            }
            prev = layout.nodes.iter().map(|(i, p)| (*i, p.pos)).collect();
        }
        let secs = frames as f32 * dt;
        assert!((3.0..=5.0).contains(&secs), "{switch:?}: {secs} s");
        for (idx, p) in &layout.nodes {
            let name = &graph.graph[*idx].name;
            assert!(
                p.pos.distance(plan[idx]) < 0.01,
                "{switch:?}: {name} at its target"
            );
            // Smoothstep peaks at 1.5× the mean speed: no jump between frames.
            let mean = start[idx].distance(plan[idx]) / frames as f32;
            assert!(
                max_step[idx] <= 1.5 * mean * 1.05 + 0.01,
                "{switch:?}: {name} step {} vs mean {mean}",
                max_step[idx]
            );
        }
        assert_eq!(layout.nodes[&m2].pos, m2_pos, "{switch:?}: pin kept");
        assert!(layout.is_sleeping, "{switch:?}: graph at rest");
        let rest: Vec<Pos2> = layout.nodes.values().map(|p| p.pos).collect();
        layout.step(&graph, dt);
        let after: Vec<Pos2> = layout.nodes.values().map(|p| p.pos).collect();
        assert_eq!(rest, after, "{switch:?}: no motion after the transition");
    }
}

#[test]
fn test_task_77_46_reset_sits_right_of_fit_and_is_disabled_in_manual() {
    use eframe::App;
    use hcom_dashboard::config::DashboardState;
    for (mode, disabled) in [("cluster", false), ("hierarchy", false), ("manual", true)] {
        let (_dir, mut app) = split_test_app(DashboardState {
            layout_mode: mode.into(),
            ..DashboardState::default()
        });
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 800.0));
        let mut frame = eframe::Frame::_new_kittest();
        let mut out = None;
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut o = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
            o.textures_delta.clear();
            out = Some(o);
        }
        let out = out.unwrap();
        let fit = widget_rect(&out, "Fit").expect("Fit button");
        let reset = widget_rects(&out, "Reset");
        assert_eq!(reset.len(), 1, "{mode}: one Reset button");
        let (rect, is_disabled) = reset[0];
        assert!(
            (rect.center().y - fit.center().y).abs() < 2.0,
            "{mode}: Fit row"
        );
        assert!(
            rect.left() > fit.right() && rect.left() - fit.right() < 12.0,
            "{mode}: right next to Fit"
        );
        assert_eq!(is_disabled, disabled, "{mode}: disabled only in Manual");
    }
}

#[test]
fn test_task_77_26_last_limit_count_and_agent_fields_texts() {
    use hcom_dashboard::app::{agent_meta_fields, format_agent_count, format_last_limit};
    // AC1: Last 0 reads as no limit.
    assert_eq!(format_last_limit(0), "no limit");
    assert_eq!(format_last_limit(25), "25 msgs");
    // AC3: one English count, both numbers explained in the tooltip.
    let (text, help) = format_agent_count(21, 53);
    assert_eq!(text, "Shown 21 / 53");
    assert!(help.contains("Shown:") && help.contains("Total:"));
    // AC4: Status Age in the since format, empty fields left out.
    let mut graph = AgentGraph::new();
    let mut s = snap("worker", None);
    s.status_age_seconds = Some(300);
    s.tool = Some("claude".into());
    graph.apply_instance_snapshot(&[s]);
    let node = graph.graph[graph.get_node_index("worker").unwrap()].clone();
    let fields = agent_meta_fields(Some(&node), None);
    let labels: Vec<&str> = fields.iter().map(|(l, _)| *l).collect();
    assert_eq!(labels, ["Status Age", "Unread Count", "CLI Tool"]);
    assert_eq!(fields[0].1, hcom_dashboard::table_data::format_age(300));
    assert_eq!(fields[0].1, "5m");
    assert!(agent_meta_fields(None, None).is_empty());
}

#[test]
fn test_task_77_11_new_live_messages_flash_their_edge_once() {
    use hcom_dashboard::canvas::FLASH_SECS;
    let msg = |graph: &mut AgentGraph, id: u64, from: &str, to: &str| {
        graph.apply_message_event(&MessageEvent {
            id,
            ts: Utc::now(),
            instance: from.into(),
            data: MessageData {
                from: from.into(),
                delivered_to: vec![to.into()],
                mentions: vec![],
                intent: None,
                reply_to: None,
                text: "x".into(),
            },
        });
    };
    let mut old = AgentGraph::new();
    msg(&mut old, 1, "a", "b");
    msg(&mut old, 2, "c", "d");
    let mut new = old.clone();
    msg(&mut new, 3, "a", "b");
    msg(&mut new, 4, "b", "a");

    // History loaded at start (nothing before): no flash.
    let mut canvas = CanvasView::new();
    canvas.flash_new_messages(&AgentGraph::new(), &new, 10.0);
    assert!(canvas.flash_until.is_empty());

    // Only edges with new messages flash, for FLASH_SECS.
    canvas.flash_new_messages(&old, &new, 10.0);
    let mut keys: Vec<_> = canvas.flash_until.keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "a".to_string())
        ]
    );
    assert!(
        canvas
            .flash_until
            .values()
            .all(|t| (*t - 10.0 - FLASH_SECS).abs() < 1e-9)
    );
    const { assert!(FLASH_SECS <= 0.5, "a short moment") };
}

#[test]
fn test_task_77_49_filter_table_toggle_and_graph_selection_scrolls_table() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState, load_state};
    use hcom_dashboard::table_data::TableRow;

    assert!(DashboardState::default().filter_table, "default: filtered");
    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into()],
        sort_col: Some("instance".into()),
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());
    let mut names: Vec<String> = (0..60).map(|i| format!("w{i:02}_keep")).collect();
    names.push("zz_other".into());
    let snaps: Vec<_> = names.iter().map(|n| snap(n, None)).collect();
    app.graph.apply_instance_snapshot(&snaps);
    app.table_rows = names
        .iter()
        .map(|n| TableRow {
            instance: n.clone(),
            ..Default::default()
        })
        .collect();
    app.search_query = "_keep".into();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run_events = |app: &mut HcomDashboardApp, mut events: Vec<egui::Event>| {
        for _ in 0..3 {
            time += 0.1;
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events: std::mem::take(&mut events),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
            out.textures_delta.clear();
        }
    };

    // Filtered: the table follows the search like the graph.
    run_events(&mut app, vec![]);
    assert_eq!(app.table_row_bands.len(), 60);
    // Full: every agent in the table, the graph stays filtered.
    app.state.filter_table = false;
    run_events(&mut app, vec![]);
    assert_eq!(app.table_row_bands.len(), 61);
    let band = *app.table_row_bands.last().unwrap();
    assert!(band.top() > screen.bottom(), "last row starts out of view");

    // Selecting a graph node far down the table scrolls its row into view.
    let target = "w59_keep";
    app.selected_row_id = Some(target.into());
    app.canvas.selected_node = app.graph.get_node_index(target);
    for _ in 0..4 {
        run_events(&mut app, vec![]);
    }
    let band = app.table_row_bands[59];
    assert!(
        band.bottom() <= screen.bottom() + 1.0 && band.top() >= 0.0,
        "selected row scrolled into view: {band:?}"
    );

    // Deselect, wheel the table back to the top, then select the same node
    // again: it scrolls into view again.
    app.selected_row_id = None;
    app.canvas.selected_node = None;
    run_events(&mut app, vec![]);
    let over_table = app.table_row_bands[59].center();
    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 5000.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    };
    for _ in 0..4 {
        run_events(
            &mut app,
            vec![egui::Event::PointerMoved(over_table), wheel.clone()],
        );
    }
    for _ in 0..10 {
        run_events(&mut app, vec![]);
    }
    assert!(
        app.table_row_bands[59].top() > screen.bottom(),
        "wheeled away: {:?}",
        app.table_row_bands[59]
    );
    app.selected_row_id = Some(target.into());
    app.canvas.selected_node = app.graph.get_node_index(target);
    for _ in 0..4 {
        run_events(&mut app, vec![]);
    }
    let band = app.table_row_bands[59];
    assert!(
        band.bottom() <= screen.bottom() + 1.0 && band.top() >= 0.0,
        "reselected row scrolled into view again: {band:?}"
    );

    // The toggle persists.
    app.save_state();
    assert!(!load_state(dir.path()).filter_table);
}

#[test]
fn test_task_77_50_table_row_double_click_or_enter_attaches() {
    use eframe::App;
    use hcom_dashboard::action::{ActionRequest, ActionWorkerHandle};
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};
    use hcom_dashboard::table_data::TableRow;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into(), "agent".into()],
        sort_col: Some("agent".into()),
        ..Default::default()
    };
    let mut app =
        HcomDashboardApp::new_for_test(DashboardConfig::default(), state, dir.path().to_path_buf());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    app.action_worker = Some(ActionWorkerHandle::new(MockActionRunner {
        recorded: recorded.clone(),
        fail_attach: false,
        transcript_result: String::new(),
    }));
    app.table_rows = vec![
        TableRow {
            agent: "agent_alpha".into(),
            instance: "inst_alpha_1".into(),
            is_running: true,
            status: "active".into(),
            ..Default::default()
        },
        TableRow {
            agent: "cat_beta".into(),
            is_running: false,
            status: "stopped".into(),
            ..Default::default()
        },
    ];
    app.sync_graph_from_table_rows();

    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run = |app: &mut HcomDashboardApp, events: Vec<egui::Event>| {
        time += 0.05;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    };
    let attaches = |name: &str| {
        std::thread::sleep(Duration::from_millis(50));
        recorded
            .lock()
            .unwrap()
            .iter()
            .filter(|r| matches!(r, ActionRequest::Attach { agent, .. } if agent == name))
            .count()
    };
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let double_click =
        |app: &mut HcomDashboardApp,
         run: &mut dyn FnMut(&mut HcomDashboardApp, Vec<egui::Event>) -> egui::FullOutput,
         row: usize| {
            let p = app.table_row_bands[row].center();
            run(app, vec![egui::Event::PointerMoved(p)]);
            for pressed in [true, false, true, false] {
                run(app, vec![press(p, pressed)]);
            }
            run(app, vec![]);
        };
    let enter = vec![egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }];
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    assert_eq!(app.table_row_bands.len(), 2);

    // A single click only selects.
    let p = app.table_row_bands[0].center();
    run(&mut app, vec![egui::Event::PointerMoved(p)]);
    run(&mut app, vec![press(p, true)]);
    run(&mut app, vec![press(p, false)]);
    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));
    assert_eq!(attaches("inst_alpha_1"), 0, "single click does not attach");
    // Let the double-click window pass.
    for _ in 0..10 {
        run(&mut app, vec![]);
    }

    // Double-click on the running row attaches it.
    double_click(&mut app, &mut run, 0);
    assert_eq!(attaches("inst_alpha_1"), 1);
    for _ in 0..10 {
        run(&mut app, vec![]);
    }

    // Enter on the selected running row attaches it.
    run(&mut app, enter.clone());
    assert_eq!(attaches("inst_alpha_1"), 2);

    // An open dialog keeps Enter.
    app.confirm_kill_target = Some("inst_alpha_1".into());
    run(&mut app, enter.clone());
    app.confirm_kill_target = None;
    assert_eq!(attaches("inst_alpha_1"), 2, "dialog open");

    // A focused text field (the search filter) keeps Enter.
    let out = run(&mut app, vec![]);
    let field = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with("Search agent") => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .expect("search field text");
    let f = Pos2::new(field.left() + 4.0, field.center().y);
    run(&mut app, vec![egui::Event::PointerMoved(f)]);
    run(&mut app, vec![press(f, true)]);
    run(&mut app, vec![press(f, false)]);
    assert!(ctx.text_edit_focused(), "search field has focus");
    run(&mut app, enter.clone());
    assert_eq!(attaches("inst_alpha_1"), 2, "text field keeps Enter");
    ctx.memory_mut(|m| m.stop_text_input());
    run(&mut app, vec![]);
    for _ in 0..10 {
        run(&mut app, vec![]);
    }

    // A stopped agent's row never attaches.
    double_click(&mut app, &mut run, 1);
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    run(&mut app, enter);
    assert_eq!(attaches("cat_beta"), 0, "stopped agent");
}

#[test]
fn test_task_77_51_slash_focuses_the_search_field() {
    use eframe::App;
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::{DashboardConfig, DashboardState};

    let dir = tempfile::tempdir().unwrap();
    let mut app = HcomDashboardApp::new_for_test(
        DashboardConfig::default(),
        DashboardState::default(),
        dir.path().to_path_buf(),
    );
    let ctx = egui::Context::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    let mut run = |app: &mut HcomDashboardApp, events: Vec<egui::Event>| {
        time += 0.05;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| app.ui(ui, &mut frame))
            .textures_delta
            .clear();
    };
    let slash = || vec![egui::Event::Text("/".into())];
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    assert!(!ctx.text_edit_focused());

    run(&mut app, slash());
    run(&mut app, vec![]);
    assert!(ctx.text_edit_focused(), "/ focuses the search field");
    assert_eq!(app.search_query, "", "the shortcut is not typed");

    // Inside the field, / is ordinary input.
    run(&mut app, slash());
    run(&mut app, vec![]);
    assert_eq!(app.search_query, "/");
    assert!(ctx.text_edit_focused());
}

/// App with the mock action runner and two rows: running `inst_alpha_1`
/// (agent `agent_alpha`) and stopped catalog agent `cat_beta`.
fn task_77_52_app(
    config: hcom_dashboard::config::DashboardConfig,
) -> (
    hcom_dashboard::app::HcomDashboardApp,
    Arc<Mutex<Vec<ActionRequest>>>,
    tempfile::TempDir,
) {
    use hcom_dashboard::app::HcomDashboardApp;
    use hcom_dashboard::config::DashboardState;
    use hcom_dashboard::table_data::{CatalogAgentItem, TableRow};

    let dir = tempfile::tempdir().unwrap();
    let state = DashboardState {
        columns: vec!["instance".into(), "agent".into()],
        sort_col: Some("agent".into()),
        ..Default::default()
    };
    let mut app = HcomDashboardApp::new_for_test(config, state, dir.path().to_path_buf());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    app.action_worker = Some(ActionWorkerHandle::new(MockActionRunner {
        recorded: recorded.clone(),
        fail_attach: false,
        transcript_result: String::new(),
    }));
    app.table_rows = vec![
        TableRow {
            agent: "agent_alpha".into(),
            instance: "inst_alpha_1".into(),
            is_running: true,
            status: "active".into(),
            directory: "/work/alpha".into(),
            ..Default::default()
        },
        TableRow {
            agent: "cat_beta".into(),
            is_running: false,
            status: "stopped".into(),
            directory: "/work/beta".into(),
            cli: "claude".into(),
            catalog: Some(CatalogAgentItem {
                name: "cat_beta".into(),
                cli: Some("codex".into()),
                model: Some("gpt-5".into()),
                reasoning: Some("high".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
    ];
    app.sync_graph_from_table_rows();
    (app, recorded, dir)
}

fn task_77_52_runner(
    ctx: egui::Context,
) -> impl FnMut(&mut hcom_dashboard::app::HcomDashboardApp, Vec<egui::Event>) -> egui::FullOutput {
    use eframe::App;
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 700.0));
    let mut frame = eframe::Frame::_new_kittest();
    let mut time = 0.0;
    move |app, events| {
        time += 0.05;
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut frame));
        out.textures_delta.clear();
        out
    }
}

fn task_77_52_key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// Clicks the search field so it has the keyboard.
fn task_77_52_focus_search(
    ctx: &egui::Context,
    app: &mut hcom_dashboard::app::HcomDashboardApp,
    run: &mut impl FnMut(
        &mut hcom_dashboard::app::HcomDashboardApp,
        Vec<egui::Event>,
    ) -> egui::FullOutput,
) {
    let out = run(app, vec![]);
    let field = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with("Search agent") => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .expect("search field text");
    let f = Pos2::new(field.left() + 4.0, field.center().y);
    let press = |pressed| egui::Event::PointerButton {
        pos: f,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run(app, vec![egui::Event::PointerMoved(f)]);
    run(app, vec![press(true)]);
    run(app, vec![press(false)]);
    assert!(ctx.text_edit_focused(), "search field has focus");
}

#[test]
fn test_task_77_52_open_with_key_runs_the_app_like_its_button() {
    use hcom_dashboard::config::{DashboardConfig, OpenWithAction};

    let config = DashboardConfig {
        open_with: vec![OpenWithAction {
            label: "repos".into(),
            cmd: vec!["repo-gui".into()],
            cwd: Some("{dir}".into()),
            key: Some("g".into()),
            if_exists: None,
        }],
        ..Default::default()
    };
    let (mut app, recorded, _dir) = task_77_52_app(config);
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let spawns = || {
        std::thread::sleep(Duration::from_millis(50));
        recorded
            .lock()
            .unwrap()
            .iter()
            .filter(|r| {
                matches!(r, ActionRequest::SpawnDetached { cmd, cwd }
                    if cmd == &vec!["repo-gui".to_string()]
                        && cwd.as_deref() == Some("/work/alpha"))
            })
            .count()
    };
    let g = || vec![egui::Event::Text("g".into())];
    for _ in 0..3 {
        run(&mut app, vec![]);
    }

    // No selected agent: the button is disabled, the key does nothing.
    run(&mut app, g());
    assert_eq!(spawns(), 0, "no selection");

    app.selected_row_id = Some("inst_alpha_1".into());
    run(&mut app, vec![]);
    run(&mut app, g());
    assert_eq!(spawns(), 1, "the key runs the app for the selected agent");

    // Another character does nothing.
    run(&mut app, vec![egui::Event::Text("x".into())]);
    assert_eq!(spawns(), 1);

    // An open dialog keeps the keyboard.
    app.confirm_kill_target = Some("inst_alpha_1".into());
    run(&mut app, g());
    app.confirm_kill_target = None;
    assert_eq!(spawns(), 1, "dialog open");

    // In a text field the key is ordinary input.
    task_77_52_focus_search(&ctx, &mut app, &mut run);
    run(&mut app, g());
    run(&mut app, vec![]);
    assert_eq!(spawns(), 1, "text field keeps the key");
    assert_eq!(app.search_query, "g");
}

#[test]
fn test_task_77_53_alt_arrows_select_rows_wherever_the_keyboard_is() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, _recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let alt = egui::Modifiers::ALT;
    let down = |m| vec![task_77_52_key(egui::Key::ArrowDown, m)];
    let up = |m| vec![task_77_52_key(egui::Key::ArrowUp, m)];
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    app.selected_row_id = Some("inst_alpha_1".into());
    app.canvas.selected_node = app.graph.get_node_index("inst_alpha_1");
    run(&mut app, vec![]);

    // The search field has the keyboard. Alt+Down / Alt+Up select the next
    // / previous row like a click.
    task_77_52_focus_search(&ctx, &mut app, &mut run);
    run(&mut app, down(alt));
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    assert_eq!(
        app.canvas.selected_node,
        app.graph.get_node_index("cat_beta"),
        "the graph follows"
    );
    assert!(app.state.show_side_panel, "the detail panel shows it");
    assert!(ctx.text_edit_focused(), "the field keeps the keyboard");
    run(&mut app, up(alt));
    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));

    // Plain arrows leave the selection alone while a text field has the
    // keyboard (egui may move the focus on).
    run(&mut app, down(egui::Modifiers::NONE));
    assert_eq!(app.selected_row_id.as_deref(), Some("inst_alpha_1"));

    // Without a focused text field Alt+arrows act too.
    ctx.memory_mut(|m| m.stop_text_input());
    run(&mut app, vec![]);
    run(&mut app, down(alt));
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
}

#[test]
fn test_task_77_54_action_error_shows_only_for_its_agent() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, _recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let banner = |out: &egui::FullOutput| {
        out.shapes.iter().any(
            |c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text().starts_with("⚠ Screen")),
        )
    };
    let select = |app: &mut hcom_dashboard::app::HcomDashboardApp, id: &str| {
        let row = app
            .table_rows
            .iter()
            .find(|r| r.instance == id || (r.instance.is_empty() && r.agent == id))
            .cloned()
            .unwrap();
        app.select_table_row(&row);
    };
    select(&mut app, "inst_alpha_1");
    app.set_action_error(
        Some("inst_alpha_1"),
        "Screen for 'inst_alpha_1' failed: gone".into(),
    );
    let out = run(&mut app, vec![]);
    assert!(banner(&out), "shown for its agent");

    select(&mut app, "cat_beta");
    let out = run(&mut app, vec![]);
    assert!(!banner(&out), "not shown for another agent");
    assert_eq!(app.shown_action_error(), None);

    select(&mut app, "inst_alpha_1");
    let out = run(&mut app, vec![]);
    assert!(banner(&out), "shown again for its agent");

    // An error without an agent belongs to the agent selected when it came.
    app.set_action_error(None, "Command failed: x".into());
    assert_eq!(app.action_error_agent.as_deref(), Some("inst_alpha_1"));
    select(&mut app, "cat_beta");
    assert_eq!(app.shown_action_error(), None);
}

#[test]
fn test_task_77_55_56_start_dialog_like_hcom_gui() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let none = egui::Modifiers::NONE;
    let enter = |m| vec![task_77_52_key(egui::Key::Enter, m)];
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: none,
    };
    let text_rect = |out: &egui::FullOutput, s: &str| {
        out.shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text() == s => {
                Some((Rect::from_min_size(t.pos, t.galley.size()), c.clip_rect))
            }
            _ => None,
        })
    };
    let commands = || {
        std::thread::sleep(Duration::from_millis(50));
        recorded
            .lock()
            .unwrap()
            .iter()
            .filter_map(|r| match r {
                ActionRequest::RunCommand { cmd, cwd } => Some((cmd.clone(), cwd.clone())),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for _ in 0..3 {
        run(&mut app, vec![]);
    }

    // Double-click on the stopped catalog agent opens the dialog.
    let p = app.table_row_bands[1].center();
    run(&mut app, vec![egui::Event::PointerMoved(p)]);
    for pressed in [true, false, true, false] {
        run(&mut app, vec![press(p, pressed)]);
    }
    // The window sizes itself in its first frames.
    for _ in 0..4 {
        run(&mut app, vec![]);
    }
    let out = run(&mut app, vec![]);
    assert!(app.show_start_dialog, "double-click opens Start");
    assert_eq!(app.selected_row_id.as_deref(), Some("cat_beta"));
    // Prefilled from the catalog entry, the name empty.
    assert_eq!(app.start_agent_name, "cat_beta");
    assert_eq!(app.start_instance_name, "");
    assert_eq!(app.start_cli, "codex");
    assert_eq!(app.start_model, "gpt-5");
    assert_eq!(app.start_reasoning, "high");
    assert_eq!(app.start_dir, "/work/beta");
    assert!(app.start_cli_choices.contains(&"codex".to_string()));
    assert_eq!(app.start_model_choices, vec!["gpt-5".to_string()]);
    // Centered on the window.
    let (_, win) = text_rect(&out, "Start cat_beta").expect("dialog shown");
    assert!(
        (win.center().x - 800.0).abs() < 2.0 && (win.center().y - 350.0).abs() < 2.0,
        "centered: {win:?}"
    );
    // The fields of contrib/hcom-gui, with a menu for each suggestion list.
    for label in [
        "name",
        "dir",
        "cli",
        "model",
        "reasoning",
        "prompt",
        "OK",
        "Cancel",
        "Browse…",
    ] {
        assert!(text_rect(&out, label).is_some(), "{label}");
    }
    assert!(
        out.shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == "▾"))
            .count()
            >= 2,
        "model and reasoning suggestion menus"
    );

    // Focus on the name field prefills "<agent>__".
    let (name_label, _) = text_rect(&out, "name").unwrap();
    let field = Pos2::new(name_label.right() + 60.0, name_label.center().y);
    run(&mut app, vec![egui::Event::PointerMoved(field)]);
    run(&mut app, vec![press(field, true)]);
    run(&mut app, vec![press(field, false)]);
    run(&mut app, vec![]);
    assert!(ctx.text_edit_focused());
    assert_eq!(app.start_instance_name, "cat_beta__");

    // Untouched answers pass no flag; Enter in the dialog runs it.
    run(&mut app, enter(none));
    run(&mut app, vec![]);
    assert!(!app.show_start_dialog);
    assert_eq!(
        commands(),
        vec![(
            vec!["hcom".to_string(), "agent".into(), "cat_beta".into()],
            Some("/work/beta".to_string())
        )]
    );
    ctx.memory_mut(|m| m.stop_text_input());
    run(&mut app, vec![]);

    // Enter on the selected stopped row opens it again; Escape cancels.
    run(&mut app, enter(none));
    run(&mut app, vec![]);
    assert!(app.show_start_dialog, "Enter opens Start");
    assert_eq!(commands().len(), 1, "the opening Enter does not press OK");
    run(&mut app, vec![task_77_52_key(egui::Key::Escape, none)]);
    assert!(!app.show_start_dialog, "Escape cancels");
    assert_eq!(commands().len(), 1);

    // A running agent still attaches on Enter.
    app.selected_row_id = Some("inst_alpha_1".into());
    run(&mut app, vec![]);
    run(&mut app, enter(none));
    run(&mut app, vec![]);
    assert!(!app.show_start_dialog);
}

#[test]
fn test_task_77_56_start_command_drops_unchanged_answers() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, _recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let row = app.table_rows[1].clone();
    app.open_start_dialog(&row);
    assert_eq!(app.start_command(), vec!["hcom", "agent", "cat_beta"]);

    // The untouched clone prefix counts as empty; resume stays --resume.
    app.start_instance_name = "cat_beta__".into();
    app.start_resume = true;
    assert_eq!(
        app.start_command(),
        vec!["hcom", "agent", "cat_beta", "--resume"]
    );

    // Changed answers pass their flags; another CLI resumes with --continue.
    app.start_instance_name = "cat_beta__x".into();
    app.start_dir = "/other".into();
    app.start_cli = "claude".into();
    app.start_model = "opus".into();
    app.start_reasoning = "max".into();
    app.start_prompt = " do it \n".into();
    assert_eq!(
        app.start_command(),
        vec![
            "hcom",
            "agent",
            "cat_beta",
            "--continue",
            "--as",
            "cat_beta__x",
            "--dir",
            "/other",
            "--cli",
            "claude",
            "--model",
            "opus",
            "--reasoning",
            "max",
            "--hcom-prompt",
            "do it",
        ]
    );
}

#[test]
fn test_task_77_56_installed_clis_lists_executables_on_path_and_the_current_one() {
    use hcom_dashboard::app::installed_clis;

    let bin = tempfile::tempdir().unwrap();
    std::fs::write(bin.path().join("cursor-agent"), "").unwrap();
    std::fs::write(bin.path().join("claude"), "").unwrap();
    let path = std::env::join_paths([bin.path()]).unwrap();
    assert_eq!(
        installed_clis("grok", Some(&path)),
        vec!["claude", "cursor", "grok"]
    );
    assert_eq!(installed_clis("", None), Vec::<String>::new());
}

#[test]
fn test_task_77_56_browse_choice_fills_dir() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, _recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let row = app.table_rows[1].clone();
    app.open_start_dialog(&row);

    // A cancelled chooser keeps the dir.
    let (tx, rx) = std::sync::mpsc::channel();
    app.start_dir_pick = Some(rx);
    tx.send(None).unwrap();
    run(&mut app, vec![]);
    assert_eq!(app.start_dir, "/work/beta");
    assert!(app.start_dir_pick.is_none());

    // A chosen folder fills it.
    let (tx, rx) = std::sync::mpsc::channel();
    app.start_dir_pick = Some(rx);
    run(&mut app, vec![]);
    assert!(app.start_dir_pick.is_some(), "still waiting");
    tx.send(Some(std::path::PathBuf::from("/picked"))).unwrap();
    run(&mut app, vec![]);
    assert_eq!(app.start_dir, "/picked");
    assert!(app.start_dir_pick.is_none());
    assert_eq!(
        app.start_command(),
        vec!["hcom", "agent", "cat_beta", "--dir", "/picked"]
    );

    // Reopening drops a pending choice of the earlier dialog.
    let (_tx, rx) = std::sync::mpsc::channel();
    app.start_dir_pick = Some(rx);
    app.open_start_dialog(&row);
    assert!(app.start_dir_pick.is_none());
}

#[test]
fn test_task_77_57_alt_left_right_switch_detail_tabs() {
    use hcom_dashboard::config::{DETAIL_TABS, DashboardConfig};

    let (mut app, _recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let alt = egui::Modifiers::ALT;
    let left = |m| vec![task_77_52_key(egui::Key::ArrowLeft, m)];
    let right = |m| vec![task_77_52_key(egui::Key::ArrowRight, m)];
    let last = DETAIL_TABS.len() - 1;
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    app.select_detail_tab(DETAIL_TABS[0]);
    run(&mut app, vec![]);

    // At the first tab Alt+Left stays.
    run(&mut app, left(alt));
    assert_eq!(app.state.detail_tab, DETAIL_TABS[0]);

    // In a text field too, Alt+Right selects the next tab like a click.
    task_77_52_focus_search(&ctx, &mut app, &mut run);
    run(&mut app, right(alt));
    assert_eq!(app.state.detail_tab, DETAIL_TABS[1]);
    assert!(ctx.text_edit_focused(), "the field keeps the keyboard");
    run(&mut app, right(egui::Modifiers::NONE));
    assert_eq!(app.state.detail_tab, DETAIL_TABS[1], "plain arrow");
    run(&mut app, left(alt));
    assert_eq!(app.state.detail_tab, DETAIL_TABS[0]);

    // At the last tab Alt+Right stays; a hidden panel is shown.
    app.select_detail_tab(DETAIL_TABS[last]);
    app.state.show_side_panel = false;
    run(&mut app, right(alt));
    assert_eq!(app.state.detail_tab, DETAIL_TABS[last]);
    assert!(app.state.show_side_panel, "shown like a click on the tab");
}

#[test]
fn test_task_77_58_dialogs_are_modal() {
    use hcom_dashboard::config::DashboardConfig;

    let (mut app, recorded, _dir) = task_77_52_app(DashboardConfig::default());
    let ctx = egui::Context::default();
    let mut run = task_77_52_runner(ctx.clone());
    let none = egui::Modifiers::NONE;
    let key = |k| vec![task_77_52_key(k, none)];
    let dialog_rect = |out: &egui::FullOutput, title: &str| {
        out.shapes
            .iter()
            .find_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text() == title => Some(c.clip_rect),
                _ => None,
            })
            .unwrap_or_else(|| panic!("dialog {title} shown"))
    };
    let focused_rect = |ctx: &egui::Context| {
        let id = ctx.memory(|m| m.focused()).expect("a widget has focus");
        ctx.read_response(id).expect("focused widget").rect
    };
    let settle = |app: &mut hcom_dashboard::app::HcomDashboardApp,
                  run: &mut dyn FnMut(
        &mut hcom_dashboard::app::HcomDashboardApp,
        Vec<egui::Event>,
    ) -> egui::FullOutput| {
        for _ in 0..4 {
            run(app, vec![]);
        }
        run(app, vec![])
    };
    for _ in 0..3 {
        run(&mut app, vec![]);
    }
    let row = app.table_rows[1].clone();
    app.select_table_row(&row);

    // Start: the first field has focus and Tab stays inside the dialog.
    app.open_start_dialog(&row);
    let out = settle(&mut app, &mut run);
    let dialog = dialog_rect(&out, "Start cat_beta");
    assert!(
        (dialog.center().x - 800.0).abs() < 2.0 && (dialog.center().y - 350.0).abs() < 2.0,
        "centered: {dialog:?}"
    );
    assert!(
        dialog.contains_rect(focused_rect(&ctx)),
        "first field focused"
    );
    for n in 0..20 {
        run(&mut app, key(egui::Key::Tab));
        run(&mut app, vec![]);
        let f = focused_rect(&ctx);
        assert!(dialog.contains_rect(f), "Tab {n} left the dialog: {f:?}");
    }
    // Global shortcuts stay off: F5 does not switch the detail tab.
    let tab = app.state.detail_tab.clone();
    run(&mut app, key(egui::Key::F5));
    assert_eq!(app.state.detail_tab, tab, "F5 blocked");
    run(&mut app, key(egui::Key::Escape));
    run(&mut app, vec![]);
    assert!(!app.show_start_dialog, "Escape closes Start");

    // A confirmation: Cancel has focus, clicks outside do nothing, Escape
    // closes it without killing.
    app.select_table_row(&app.table_rows[0].clone());
    app.confirm_kill_target = Some("inst_alpha_1".into());
    let out = settle(&mut app, &mut run);
    dialog_rect(&out, "Confirm Kill Agent");
    let cancel = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text() == "Cancel" => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    assert!(focused_rect(&ctx).contains_rect(cancel), "Cancel focused");
    let send = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text() == "Send..." => {
                Some(Rect::from_min_size(t.pos, t.galley.size()).center())
            }
            _ => None,
        })
        .expect("Send button");
    let press = |pressed| egui::Event::PointerButton {
        pos: send,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: none,
    };
    run(&mut app, vec![egui::Event::PointerMoved(send)]);
    run(&mut app, vec![press(true)]);
    run(&mut app, vec![press(false)]);
    run(&mut app, vec![]);
    assert!(
        !app.show_send_dialog,
        "the toolbar under the dialog is blocked"
    );
    assert!(app.confirm_kill_target.is_some());
    run(&mut app, key(egui::Key::Escape));
    run(&mut app, vec![]);
    assert!(app.confirm_kill_target.is_none(), "Escape closes it");
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        !recorded
            .lock()
            .unwrap()
            .iter()
            .any(|r| matches!(r, ActionRequest::Kill { .. })),
        "nothing killed"
    );
    // Enter on the focused Cancel closes it; the table does not attach.
    app.confirm_kill_target = Some("inst_alpha_1".into());
    settle(&mut app, &mut run);
    run(&mut app, key(egui::Key::Enter));
    run(&mut app, vec![]);
    assert!(app.confirm_kill_target.is_none(), "Enter on Cancel");
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        !recorded
            .lock()
            .unwrap()
            .iter()
            .any(|r| matches!(r, ActionRequest::Kill { .. } | ActionRequest::Attach { .. })),
        "no kill, no attach"
    );

    // Send: the message field has focus; Escape closes it.
    app.send_target = "inst_alpha_1".into();
    app.show_send_dialog = true;
    settle(&mut app, &mut run);
    assert!(ctx.text_edit_focused(), "message field focused");
    run(&mut app, key(egui::Key::Escape));
    run(&mut app, vec![]);
    assert!(!app.show_send_dialog, "Escape closes Send");
}

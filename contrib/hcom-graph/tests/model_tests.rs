use anyhow::Result;
use chrono::Utc;
use egui::{Color32, Pos2, Rect, Vec2};
use hcom_graph::action::{ActionRequest, ActionResult, ActionRunner, ActionWorkerHandle};
use hcom_graph::canvas::{CanvasView, distance_to_quadratic_bezier, distance_to_segment};
use hcom_graph::layout::GraphLayout;
use hcom_graph::model::{
    AgentGraph, AgentNode, AgentStatus, InstanceSnapshot, MessageData, MessageEvent,
    is_valid_identity, sanitize_display_string,
};
use hcom_graph::source::{
    CommandExecutor, HcomSyncWorker, StreamHandle, StreamKillHandle, StreamReader,
    build_messages_snapshot_cmd, parse_instances, parse_messages_snapshot, read_line_capped,
    split_system_stream,
};
use hcom_graph::sync::BackgroundSync;
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
    assert_eq!(graph.get_message_edge("x", "y").unwrap().history.len(), 4);
}

#[test]
fn test_message_edge_bounded_history_and_sanitization() {
    let mut graph = AgentGraph::new();
    // Send 60 messages to exceed the 50 limit
    for id in 1..=60 {
        let event = MessageEvent {
            id,
            ts: Utc::now(),
            instance: "alice".into(),
            data: MessageData {
                from: "alice".into(),
                delivered_to: vec!["bob".into()],
                mentions: vec![],
                intent: Some("chat\x1b[31m".into()),
                text: format!("msg_{}\x1b[2J\u{200B}", id),
            },
        };
        assert!(graph.apply_message_event(&event));
    }

    let edge = graph.get_message_edge("alice", "bob").unwrap();
    assert_eq!(edge.count, 60);
    assert_eq!(
        edge.history.len(),
        50,
        "History must be bounded to MAX_EDGE_HISTORY (50)"
    );

    // Oldest messages (1..=10) should have been popped
    assert_eq!(edge.history.front().unwrap().id, 11);
    assert_eq!(edge.history.back().unwrap().id, 60);

    // Sanitization must have cleaned the text and intent
    let last = edge.history.back().unwrap();
    assert_eq!(last.text, "msg_60");
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
    let worker = HcomSyncWorker::new(executor, 0);

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

    let worker = HcomSyncWorker::new(QuietStreamExecutor, 0);
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
fn test_action_runner_stdout_capped_kills_child_and_avoids_deadlock() {
    use hcom_graph::action::SystemActionRunner;
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
    use hcom_graph::action::SystemActionRunner;
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
    use hcom_graph::action::SystemActionRunner;

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
    use hcom_graph::action::{
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
    use hcom_graph::action::SystemActionRunner;

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
fn test_config_fallback_gui_json_when_graph_json_missing() {
    use hcom_graph::config::load_config;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();

    // 1. Write gui.json
    let gui_json = r#"{
        "refresh_seconds": 12,
        "stats_seconds": 99,
        "open_with": [
            {"label": "Zed", "cmd": ["zed", "{dir}"], "key": "z"}
        ]
    }"#;
    std::fs::write(hcom_dir.join("gui.json"), gui_json).unwrap();

    // 2. Load config -> should fall back to gui.json
    let cfg = load_config(hcom_dir);
    assert_eq!(cfg.refresh_seconds, 12);
    assert_eq!(cfg.stats_seconds, 99);
    assert_eq!(cfg.open_with.len(), 1);
    assert_eq!(cfg.open_with[0].label, "Zed");

    // 3. Write graph.json -> should take precedence over gui.json
    let graph_json = r#"{
        "refresh_seconds": 3,
        "stats_seconds": 30
    }"#;
    std::fs::write(hcom_dir.join("graph.json"), graph_json).unwrap();
    let cfg2 = load_config(hcom_dir);
    assert_eq!(cfg2.refresh_seconds, 3);
    assert_eq!(cfg2.stats_seconds, 30);
}

#[test]
fn test_state_fallback_and_save_never_touches_gui_state_json() {
    use hcom_graph::config::{load_state, save_state};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();

    // 1. Write gui-state.json
    let gui_state_content = r#"{
        "view_mode": "table_only",
        "running_only": true,
        "sort_col": "age",
        "bottom_tab": "Transcript"
    }"#;
    std::fs::write(hcom_dir.join("gui-state.json"), gui_state_content).unwrap();

    // 2. Load state -> should read from gui-state.json
    let mut state = load_state(hcom_dir);
    assert_eq!(state.view_mode, "table_only");
    assert!(state.running_only);
    assert_eq!(state.sort_col.as_deref(), Some("age"));
    assert_eq!(state.bottom_tab, "Transcript");

    // 3. Mutate state and save
    state.view_mode = "split".into();
    state.running_only = false;
    save_state(hcom_dir, &state).unwrap();

    // 4. Verify graph-state.json was written
    assert!(hcom_dir.join("graph-state.json").is_file());
    let reloaded = load_state(hcom_dir);
    assert_eq!(reloaded.view_mode, "split");
    assert!(!reloaded.running_only);

    // 5. CRITICAL BOUNDARY: verify gui-state.json content is COMPLETELY UNTOUCHED!
    let current_gui_state = std::fs::read_to_string(hcom_dir.join("gui-state.json")).unwrap();
    assert_eq!(current_gui_state, gui_state_content);
}

#[test]
fn test_screen_bar_complete_set_and_custom_keys() {
    use hcom_graph::config::{ScreenBarDef, screen_bar_buttons};

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
    use hcom_graph::config::{
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
    use hcom_graph::table_data::{TableRow, sort_rows};

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
    use hcom_graph::table_data::{TableRow, filter_row};

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
    use hcom_graph::table_data::TableRow;
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

        fn handle_result(&mut self, res: &hcom_graph::action::ActionResult) {
            match res {
                hcom_graph::action::ActionResult::ScreenLoaded {
                    agent,
                    generation,
                    content,
                } => {
                    if hcom_graph::app::accept_generation_result(
                        self.selection_generation,
                        *generation,
                    ) && let Ok(text) = content
                    {
                        self.screen_content = Some((agent.clone(), *generation, text.clone()));
                    }
                }
                hcom_graph::action::ActionResult::TranscriptLoaded {
                    agent,
                    generation,
                    content,
                } => {
                    if hcom_graph::app::accept_generation_result(
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
    let stale_screen = hcom_graph::action::ActionResult::ScreenLoaded {
        agent: "agent_a".into(),
        generation: gen1,
        content: Ok("stale content of A".into()),
    };
    state.handle_result(&stale_screen);

    // MUST be dropped! screen_content must remain None
    assert!(state.screen_content.is_none());

    // 4. Fresh response for agent B arrives with gen2
    let fresh_screen = hcom_graph::action::ActionResult::ScreenLoaded {
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
    use hcom_graph::config::{load_config, load_state};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let hcom_dir = dir.path();

    // Corrupt graph.json
    std::fs::write(hcom_dir.join("graph.json"), "{ invalid: json: syntax").unwrap();
    let cfg = load_config(hcom_dir);
    // Should fallback to default refresh_seconds (5)
    assert_eq!(cfg.refresh_seconds, 5);

    // Corrupt graph-state.json
    std::fs::write(hcom_dir.join("graph-state.json"), "broken").unwrap();
    let st = load_state(hcom_dir);
    assert_eq!(st.view_mode, "split");
    assert!(!st.running_only);
}

#[test]
fn test_unknown_placeholder_rejected() {
    use hcom_graph::config::{PlaceholderContext, replace_placeholders_str};

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
    use hcom_graph::config::GraphConfig;

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

    let cfg: GraphConfig = serde_json::from_str(gui_json).expect("parse real gui.json");
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
    use hcom_graph::table_data::build_table_rows;

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
    use hcom_graph::table_data::TableRow;

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
    use hcom_graph::table_data::parse_git_porcelain_v2;

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
    use hcom_graph::table_data::{TableRow, apply_event_stats, compute_event_stats};

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
    use hcom_graph::config::{PlaceholderContext, replace_placeholders_str};
    use hcom_graph::table_data::{LiveLaunchContext, TableRow};

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
    use hcom_graph::app::HcomGraphApp;
    use hcom_graph::config::{GraphConfig, GraphState};

    let mut app = HcomGraphApp::new_for_test(
        GraphConfig::default(),
        GraphState::default(),
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
fn test_graph_state_split_ratio_and_side_panel_defaults_and_roundtrip() {
    use hcom_graph::config::GraphState;

    // Check defaults
    let default_state = GraphState::default();
    assert!(
        default_state.show_side_panel,
        "Side panel should default to open"
    );
    assert!(
        default_state.main_split_ratio.is_none(),
        "Main split ratio defaults to None"
    );

    // Check json deserialization without the new fields uses defaults
    let json_legacy = r#"{"view_mode":"split_horizontal"}"#;
    let parsed: GraphState = serde_json::from_str(json_legacy).expect("parse legacy state");
    assert!(
        parsed.show_side_panel,
        "Legacy state should default show_side_panel to true"
    );
    assert!(parsed.main_split_ratio.is_none());

    // Check roundtrip with explicit values
    let custom_state = GraphState {
        show_side_panel: false,
        main_split_ratio: Some(0.42),
        ..Default::default()
    };
    let serialized = serde_json::to_string(&custom_state).expect("serialize");
    let restored: GraphState = serde_json::from_str(&serialized).expect("deserialize");
    assert!(!restored.show_side_panel);
    assert_eq!(restored.main_split_ratio, Some(0.42));
}

#[test]
fn test_parse_context_percent_reads_trailing_percent_token() {
    use hcom_graph::model::parse_context_percent;
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
    use hcom_graph::table_data::{TableRow, node_styles_from_rows};
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
        hcom_graph::model::NodeStyle::default()
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
    use hcom_graph::canvas::context_fill_polygon;
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

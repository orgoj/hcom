use anyhow::Result;
use chrono::Utc;
use hcom_graph::model::{
    AgentGraph, AgentNode, AgentStatus, InstanceSnapshot, MessageData, MessageEvent,
};
use hcom_graph::source::{
    CommandExecutor, HcomSyncWorker, StreamHandle, build_messages_snapshot_cmd, parse_instances,
    parse_messages_snapshot,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    assert_eq!(graph.edge_count(), 1); // 1 spawn edge

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

    // 1. Initial agent under parent_a
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

    // 2. Reparent to parent_b
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

    // 3. Parent set to None
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

    // Setup hierarchy (parent -> child)
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

    // Setup messages
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
    assert_eq!(graph.edge_count(), 2); // 1 spawn + 1 message

    // Reset events
    graph.reset_events();

    // Message edge is gone, spawn edge remains intact and indexed
    assert!(graph.get_message_edge("parent", "child").is_none());
    assert!(
        graph.get_spawn_edge("parent", "child").is_some(),
        "Spawn edge must survive reset_events"
    );
    assert_eq!(graph.edge_count(), 1);

    // Subsequent upsert works properly with stable graph indices
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

    // 3 messages from Alice to Bob
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

    // 2 replies from Bob to Alice
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

    // Re-applying an older or identical ID must be ignored (idempotence)
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

    // Message count and watermark remain untouched
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

// Mock executor for full sync worker integration testing
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

    // Follower stream for epoch 1 emits msg 2, then db_reset
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

    // 1. Initial sync
    worker.initial_sync(&mut graph).unwrap();
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.get_message_edge("m1", "m2").unwrap().count, 1);
    assert_eq!(graph.last_event_id, 1);

    // 2. Start follower stream and step through msg 2
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

    // 3. Next step encounters {"db_reset": true}
    let reset_processed = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(reset_processed);

    // Verify step 1 of reset sequence: old child process was killed!
    assert!(
        killed_ref1.load(Ordering::SeqCst),
        "Old follower process must be killed upon db_reset"
    );

    // Verify step 2 and 3 of reset sequence: old messages cleared, new snapshot loaded
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

    // 4. Next step reads msg 2 from restarted new follower
    let new_msg_processed = worker.step_stream(&mut graph, &mut follower).unwrap();
    assert!(new_msg_processed);
    assert_eq!(graph.get_message_edge("m3", "m4").unwrap().count, 2);
    assert_eq!(graph.last_event_id, 2);
}

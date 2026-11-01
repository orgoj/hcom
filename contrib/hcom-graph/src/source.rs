use crate::model::{AgentGraph, InstanceSnapshot, MessageEvent};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};

#[derive(Debug, PartialEq)]
pub enum StreamItem {
    Message(MessageEvent),
    DbReset,
    Ignored,
}

pub fn build_instances_cmd() -> Vec<String> {
    vec!["hcom".into(), "list".into(), "--json".into()]
}

/// Builds command for initial snapshot. Strictly avoids `--all` to prevent
/// querying archived databases with disparate event ID sequences.
pub fn build_messages_snapshot_cmd(last_n: usize) -> Vec<String> {
    vec![
        "hcom".into(),
        "events".into(),
        "--type".into(),
        "message".into(),
        "--full".into(),
        "--last".into(),
        last_n.to_string(),
    ]
}

pub fn build_messages_follow_cmd(after_id: u64) -> Vec<String> {
    vec![
        "hcom".into(),
        "events".into(),
        "--follow".into(),
        "--full".into(),
        "--type".into(),
        "message".into(),
        "--after-id".into(),
        after_id.to_string(),
    ]
}

pub fn parse_instances(json_str: &str) -> Result<Vec<InstanceSnapshot>> {
    serde_json::from_str(json_str).context("failed to parse instance snapshots from hcom list")
}

/// Parses message events from `hcom events` JSON Lines output and sorts them
/// ascending by `event.id`. This is required because `hcom events` returns
/// newest-first by default, which would otherwise advance the watermark
/// immediately and discard older events.
pub fn parse_messages_snapshot(json_lines: &str) -> Result<Vec<MessageEvent>> {
    let mut events = Vec::new();
    for line in json_lines.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<MessageEvent>(trimmed) {
            events.push(event);
        }
    }
    // Ascending sort by ID ensures sequential watermark advancement
    events.sort_by_key(|e| e.id);
    Ok(events)
}

pub fn parse_stream_line(line: &str) -> StreamItem {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return StreamItem::Ignored;
    }

    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        if val.get("db_reset").and_then(Value::as_bool) == Some(true) {
            return StreamItem::DbReset;
        }
        if val.get("type").and_then(Value::as_str) == Some("message")
            && let Ok(event) = serde_json::from_value::<MessageEvent>(val)
        {
            return StreamItem::Message(event);
        }
    }

    StreamItem::Ignored
}

/// Abstraction for command execution to allow testing process flows
/// (snapshots, streaming follow, reconnects and db_reset) without real hcom binary.
pub trait CommandExecutor: Send + Sync {
    fn run_command(&self, argv: &[String]) -> Result<String>;
    fn spawn_stream(&self, argv: &[String]) -> Result<Box<dyn StreamHandle>>;
}

pub trait StreamHandle: Send {
    fn next_line(&mut self) -> Result<Option<String>>;
    fn kill(&mut self);
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run_command(&self, argv: &[String]) -> Result<String> {
        if argv.is_empty() {
            return Err(anyhow!("empty argv"));
        }
        let output = Command::new(&argv[0])
            .args(&argv[1..])
            .output()
            .with_context(|| format!("failed to execute {}", argv[0]))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "command {} failed (status {:?}): {}",
                argv.join(" "),
                output.status.code(),
                stderr
            ));
        }

        String::from_utf8(output.stdout).context("command output was not valid utf-8")
    }

    fn spawn_stream(&self, argv: &[String]) -> Result<Box<dyn StreamHandle>> {
        if argv.is_empty() {
            return Err(anyhow!("empty argv"));
        }
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn stream process {}", argv[0]))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture stdout for stream"))?;

        Ok(Box::new(SystemStreamHandle {
            child,
            reader: BufReader::new(stdout),
        }))
    }
}

pub struct SystemStreamHandle {
    child: Child,
    reader: BufReader<ChildStdout>,
}

impl StreamHandle for SystemStreamHandle {
    fn next_line(&mut self) -> Result<Option<String>> {
        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
        if bytes == 0 { Ok(None) } else { Ok(Some(line)) }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for SystemStreamHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Orchestrates synchronization between hcom CLI and AgentGraph.
pub struct HcomSyncWorker<E: CommandExecutor> {
    pub executor: E,
    pub history_last_n: usize,
}

impl HcomSyncWorker<SystemCommandExecutor> {
    pub fn new_system(history_last_n: usize) -> Self {
        Self {
            executor: SystemCommandExecutor,
            history_last_n,
        }
    }
}

impl<E: CommandExecutor> HcomSyncWorker<E> {
    pub fn new(executor: E, history_last_n: usize) -> Self {
        Self {
            executor,
            history_last_n,
        }
    }

    /// Fetches and applies instances from `hcom list --json`.
    pub fn refresh_instances(&self, graph: &mut AgentGraph) -> Result<()> {
        let argv = build_instances_cmd();
        let json_out = self.executor.run_command(&argv)?;
        let snapshots = parse_instances(&json_out)?;
        graph.apply_instance_snapshot(&snapshots);
        Ok(())
    }

    /// Fetches and applies message history snapshot from active DB.
    pub fn refresh_messages_snapshot(&self, graph: &mut AgentGraph) -> Result<()> {
        let argv = build_messages_snapshot_cmd(self.history_last_n);
        let lines_out = self.executor.run_command(&argv)?;
        let events = parse_messages_snapshot(&lines_out)?;
        for event in events {
            graph.apply_message_event(&event);
        }
        Ok(())
    }

    /// Performs complete initial sync: instances and message snapshot.
    pub fn initial_sync(&self, graph: &mut AgentGraph) -> Result<()> {
        self.refresh_instances(graph)?;
        self.refresh_messages_snapshot(graph)?;
        Ok(())
    }

    /// Spawns follower child process starting from `graph.last_event_id`.
    pub fn spawn_follower(&self, graph: &AgentGraph) -> Result<Box<dyn StreamHandle>> {
        let argv = build_messages_follow_cmd(graph.last_event_id);
        self.executor.spawn_stream(&argv)
    }

    /// Synchronous and deterministic recovery upon `db_reset`:
    /// 1. Kill existing follower process.
    /// 2. Clear graph events and reset watermark.
    /// 3. Fetch fresh instances and sorted messages snapshot from new DB.
    /// 4. Spawn fresh follower stream with updated watermark.
    pub fn handle_db_reset(
        &self,
        graph: &mut AgentGraph,
        follower: &mut Option<Box<dyn StreamHandle>>,
    ) -> Result<()> {
        // Step 1: Kill current follower immediately
        if let Some(mut handle) = follower.take() {
            handle.kill();
        }

        // Step 2: Reset events in graph
        graph.reset_events();

        // Step 3: Reload fresh snapshot
        self.initial_sync(graph)?;

        // Step 4: Restart follow stream with new watermark
        let new_follower = self.spawn_follower(graph)?;
        *follower = Some(new_follower);

        Ok(())
    }

    /// Processes a single stream line or handles reconnect / db_reset.
    pub fn step_stream(
        &self,
        graph: &mut AgentGraph,
        follower: &mut Option<Box<dyn StreamHandle>>,
    ) -> Result<bool> {
        let Some(handle) = follower else {
            // No active follower, spawn one
            let new_handle = self.spawn_follower(graph)?;
            *follower = Some(new_handle);
            return Ok(false);
        };

        match handle.next_line()? {
            Some(line) => match parse_stream_line(&line) {
                StreamItem::Message(msg) => Ok(graph.apply_message_event(&msg)),
                StreamItem::DbReset => {
                    self.handle_db_reset(graph, follower)?;
                    Ok(true)
                }
                StreamItem::Ignored => Ok(false),
            },
            None => {
                // Stream reached EOF or process died; reconnect
                let mut old = follower.take();
                if let Some(ref mut h) = old {
                    h.kill();
                }
                let new_handle = self.spawn_follower(graph)?;
                *follower = Some(new_handle);
                Ok(false)
            }
        }
    }
}

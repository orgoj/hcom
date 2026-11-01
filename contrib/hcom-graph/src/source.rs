use crate::model::{AgentGraph, InstanceSnapshot, MessageEvent};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_MAX_LINE_BYTES: usize = 1024 * 1024; // 1 MiB cap
pub const DEFAULT_MAX_CMD_OUTPUT_BYTES: usize = 16 * 1024 * 1024; // 16 MiB cap for command output

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
        if trimmed.len() > DEFAULT_MAX_LINE_BYTES {
            // Drop lines exceeding line limit
            continue;
        }
        if let Ok(event) = serde_json::from_str::<MessageEvent>(trimmed) {
            events.push(event);
        }
    }
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

/// Reads a single line from reader up to `cap` bytes.
/// If `cap` is reached without finding a newline `\n`, it consumes and discards
/// (drains) the rest of the oversized line until `\n` or EOF, returning an error.
pub fn read_line_capped(reader: &mut impl BufRead, cap: usize) -> Result<Option<String>> {
    let mut buf = Vec::new();
    let mut total_read = 0;

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if buf.is_empty() {
                return Ok(None);
            }
            if total_read >= cap {
                return Err(anyhow!(
                    "Line reached or exceeded limit of {} bytes without newline",
                    cap
                ));
            }
            break;
        }

        if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
            let to_take = newline_pos + 1;
            if total_read + to_take > cap {
                reader.consume(to_take);
                return Err(anyhow!(
                    "Line exceeded maximum allowed size of {} bytes",
                    cap
                ));
            }
            buf.extend_from_slice(&available[..to_take]);
            reader.consume(to_take);
            break;
        } else {
            let len = available.len();
            if total_read + len >= cap {
                reader.consume(len);
                // Drain rest of line until newline or EOF
                loop {
                    let next_buf = reader.fill_buf()?;
                    if next_buf.is_empty() {
                        break;
                    }
                    if let Some(pos) = next_buf.iter().position(|&b| b == b'\n') {
                        reader.consume(pos + 1);
                        break;
                    }
                    let nlen = next_buf.len();
                    reader.consume(nlen);
                }
                return Err(anyhow!(
                    "Line exceeded maximum allowed size of {} bytes; drained rest of line",
                    cap
                ));
            }
            buf.extend_from_slice(available);
            total_read += len;
            reader.consume(len);
        }
    }

    let s = String::from_utf8(buf).context("line was not valid utf-8")?;
    Ok(Some(s))
}

pub trait CommandExecutor: Send + Sync {
    fn run_command(&self, argv: &[String]) -> Result<String>;
    fn spawn_stream(&self, argv: &[String]) -> Result<Box<dyn StreamHandle>>;
    fn spawn_stream_split(
        &self,
        argv: &[String],
    ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)>;
}

pub trait StreamKillHandle: Send + Sync {
    fn kill(&self) -> Result<()>;
}

pub trait StreamReader: Send {
    fn next_line(&mut self, cap: usize) -> Result<Option<String>>;
}

pub struct SystemStreamKillHandle {
    child: Mutex<Option<Child>>,
}

impl StreamKillHandle for SystemStreamKillHandle {
    fn kill(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

pub struct SystemStreamReader {
    reader: BufReader<ChildStdout>,
}

impl StreamReader for SystemStreamReader {
    fn next_line(&mut self, cap: usize) -> Result<Option<String>> {
        read_line_capped(&mut self.reader, cap)
    }
}

pub fn split_system_stream(
    child: Child,
    stdout: ChildStdout,
) -> (Box<dyn StreamReader>, Arc<dyn StreamKillHandle>) {
    (
        Box::new(SystemStreamReader {
            reader: BufReader::new(stdout),
        }),
        Arc::new(SystemStreamKillHandle {
            child: Mutex::new(Some(child)),
        }),
    )
}

pub trait StreamHandle: Send {
    fn next_line(&mut self) -> Result<Option<String>>;
    fn kill(&mut self);
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run_command(&self, argv: &[String]) -> Result<String> {
        self.run_command_capped(argv, DEFAULT_MAX_CMD_OUTPUT_BYTES)
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
            cap: DEFAULT_MAX_LINE_BYTES,
        }))
    }

    fn spawn_stream_split(
        &self,
        argv: &[String],
    ) -> Result<(Box<dyn StreamReader>, Arc<dyn StreamKillHandle>)> {
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

        Ok(split_system_stream(child, stdout))
    }
}

impl SystemCommandExecutor {
    pub fn run_command_capped(&self, argv: &[String], max_bytes: usize) -> Result<String> {
        if argv.is_empty() {
            return Err(anyhow!("empty argv"));
        }
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to execute {}", argv[0]))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("missing stdout"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("missing stderr"))?;

        // Concurrently drain stderr in a thread to prevent OS pipe buffer deadlock
        let stderr_handle = std::thread::spawn(move || {
            let mut err_buf = Vec::new();
            let mut take = Read::take(&mut stderr, 64 * 1024);
            let _ = take.read_to_end(&mut err_buf);
            // Drain any remaining stderr to sink so child never blocks and never receives SIGPIPE
            let mut sink = std::io::sink();
            let _ = std::io::copy(&mut stderr, &mut sink);
            err_buf
        });

        let mut out_buf = Vec::new();
        let read_bytes =
            Read::take(&mut stdout, (max_bytes + 1) as u64).read_to_end(&mut out_buf)?;
        if read_bytes > max_bytes {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_handle.join();
            return Err(anyhow!(
                "command {} output exceeded maximum allowed size of {} bytes",
                argv[0],
                max_bytes
            ));
        }

        let status = child.wait()?;
        let err_buf = stderr_handle.join().unwrap_or_default();

        if !status.success() {
            let stderr_str = String::from_utf8_lossy(&err_buf);
            return Err(anyhow!(
                "command {} failed (status {:?}): {}",
                argv.join(" "),
                status.code(),
                stderr_str
            ));
        }

        String::from_utf8(out_buf).context("command output was not valid utf-8")
    }
}

pub struct SystemStreamHandle {
    child: Child,
    reader: BufReader<ChildStdout>,
    cap: usize,
}

impl StreamHandle for SystemStreamHandle {
    fn next_line(&mut self) -> Result<Option<String>> {
        read_line_capped(&mut self.reader, self.cap)
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
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub current_backoff: Mutex<Duration>,
    pub reconnect_deadline: Mutex<Option<Instant>>,
}

impl HcomSyncWorker<SystemCommandExecutor> {
    pub fn new_system(history_last_n: usize) -> Self {
        Self {
            executor: SystemCommandExecutor,
            history_last_n,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            current_backoff: Mutex::new(Duration::from_millis(500)),
            reconnect_deadline: Mutex::new(None),
        }
    }
}

impl<E: CommandExecutor> HcomSyncWorker<E> {
    pub fn new(executor: E, history_last_n: usize) -> Self {
        Self {
            executor,
            history_last_n,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            current_backoff: Mutex::new(Duration::from_millis(500)),
            reconnect_deadline: Mutex::new(None),
        }
    }

    pub fn refresh_instances(&self, graph: &mut AgentGraph) -> Result<()> {
        let argv = build_instances_cmd();
        let json_out = self.executor.run_command(&argv)?;
        let snapshots = parse_instances(&json_out)?;
        graph.apply_instance_snapshot(&snapshots);
        Ok(())
    }

    pub fn refresh_messages_snapshot(&self, graph: &mut AgentGraph) -> Result<()> {
        let argv = build_messages_snapshot_cmd(self.history_last_n);
        let lines_out = self.executor.run_command(&argv)?;
        let events = parse_messages_snapshot(&lines_out)?;
        for event in events {
            graph.apply_message_event(&event);
        }
        Ok(())
    }

    pub fn initial_sync(&self, graph: &mut AgentGraph) -> Result<()> {
        self.refresh_instances(graph)?;
        self.refresh_messages_snapshot(graph)?;
        Ok(())
    }

    pub fn spawn_follower(&self, graph: &AgentGraph) -> Result<Box<dyn StreamHandle>> {
        let argv = build_messages_follow_cmd(graph.last_event_id);
        self.executor.spawn_stream(&argv)
    }

    pub fn handle_db_reset(
        &self,
        graph: &mut AgentGraph,
        follower: &mut Option<Box<dyn StreamHandle>>,
    ) -> Result<()> {
        if let Some(mut handle) = follower.take() {
            handle.kill();
        }

        graph.reset_events();
        self.initial_sync(graph)?;

        let new_follower = self.spawn_follower(graph)?;
        *follower = Some(new_follower);

        // Reset backoff on full reset resync
        *self.current_backoff.lock().unwrap() = self.initial_backoff;
        *self.reconnect_deadline.lock().unwrap() = None;

        Ok(())
    }

    fn schedule_backoff(&self) {
        let mut b = self.current_backoff.lock().unwrap();
        *self.reconnect_deadline.lock().unwrap() = Some(Instant::now() + *b);
        *b = (*b * 2).min(self.max_backoff);
    }

    /// Processes a single stream line or handles non-blocking reconnect / db_reset.
    pub fn step_stream(
        &self,
        graph: &mut AgentGraph,
        follower: &mut Option<Box<dyn StreamHandle>>,
    ) -> Result<bool> {
        // If in backoff delay, check whether deadline passed
        if let Some(deadline) = *self.reconnect_deadline.lock().unwrap() {
            if Instant::now() < deadline {
                return Ok(false);
            }
            *self.reconnect_deadline.lock().unwrap() = None;
        }

        let Some(handle) = follower else {
            match self.spawn_follower(graph) {
                Ok(new_handle) => {
                    *follower = Some(new_handle);
                    return Ok(false);
                }
                Err(_) => {
                    self.schedule_backoff();
                    return Ok(false);
                }
            }
        };

        match handle.next_line() {
            Ok(Some(line)) => match parse_stream_line(&line) {
                StreamItem::Message(msg) => {
                    let updated = graph.apply_message_event(&msg);
                    if updated {
                        *self.current_backoff.lock().unwrap() = self.initial_backoff;
                        *self.reconnect_deadline.lock().unwrap() = None;
                    }
                    Ok(updated)
                }
                StreamItem::DbReset => {
                    self.handle_db_reset(graph, follower)?;
                    Ok(true)
                }
                StreamItem::Ignored => Ok(false),
            },
            Ok(None) => {
                // Stream reached EOF; kill old handle and schedule non-blocking backoff
                let mut old = follower.take();
                if let Some(ref mut h) = old {
                    h.kill();
                }
                self.schedule_backoff();
                Ok(false)
            }
            Err(_) => {
                // I/O error reading line; schedule backoff
                let mut old = follower.take();
                if let Some(ref mut h) = old {
                    h.kill();
                }
                self.schedule_backoff();
                Ok(false)
            }
        }
    }
}

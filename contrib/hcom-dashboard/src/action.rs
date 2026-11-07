use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRequest {
    Attach {
        agent: String,
        custom_cmd: Option<Vec<String>>,
    },
    Inject {
        agent: String,
        text: String,
        enter: bool,
    },
    Transcript {
        agent: String,
        generation: u64,
        last_lines: usize,
    },
    /// Transcript file of one session of `agent` (Sessions tab).
    SessionTranscript {
        agent: String,
        generation: u64,
        path: String,
    },
    /// Sessions of `agent` for the Sessions tab.
    Sessions {
        agent: String,
        generation: u64,
    },
    Screen {
        agent: String,
        generation: u64,
    },
    Events {
        agent: String,
        generation: u64,
        last_n: usize,
    },
    Kill {
        agent: String,
    },
    KillGroup {
        agents: Vec<String>,
    },
    RunCommand {
        cmd: Vec<String>,
        cwd: Option<String>,
    },
    SpawnDetached {
        cmd: Vec<String>,
        cwd: Option<String>,
    },
    ReloadTableData,
    ReloadStats,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    AttachFinished {
        agent: String,
        result: Result<(), String>,
    },
    InjectFinished {
        agent: String,
        result: Result<(), String>,
    },
    TranscriptLoaded {
        agent: String,
        generation: u64,
        content: Result<String, String>,
    },
    SessionsLoaded {
        agent: String,
        generation: u64,
        sessions: Result<Vec<crate::sessions::SessionInfo>, String>,
    },
    ScreenLoaded {
        agent: String,
        generation: u64,
        content: Result<String, String>,
    },
    EventsLoaded {
        agent: String,
        generation: u64,
        content: Result<String, String>,
    },
    KillFinished {
        agent: String,
        result: Result<(), String>,
    },
    KillGroupFinished {
        count: usize,
        result: Result<(), String>,
    },
    CommandFinished {
        result: Result<String, String>,
    },
    SpawnDetachedFinished {
        cmd: Vec<String>,
        result: Result<(), String>,
    },
    TableDataLoaded {
        rows: Vec<crate::table_data::TableRow>,
        catalog_groups: Vec<String>,
        tool_groups: Vec<String>,
    },
    TableDataFailed {
        error: String,
    },
    StatsLoaded {
        stats: std::collections::HashMap<String, crate::table_data::EventStatsEntry>,
    },
    StatsFailed {
        error: String,
    },
}

pub fn clamp_transcript_lines(lines: usize) -> usize {
    lines.clamp(1, 100)
}

pub fn build_transcript_argv(agent: &str, last_lines: usize) -> Vec<String> {
    vec![
        "transcript".to_string(),
        agent.to_string(),
        "--last".to_string(),
        clamp_transcript_lines(last_lines).to_string(),
    ]
}

pub fn build_attach_argv(agent: &str) -> Vec<String> {
    vec!["agent".to_string(), "attach".to_string(), agent.to_string()]
}

pub fn build_inject_argv(agent: &str, text: &str, enter: bool) -> Vec<String> {
    let mut argv = vec![
        "term".to_string(),
        "inject".to_string(),
        agent.to_string(),
        text.to_string(),
    ];
    if enter {
        argv.push("--enter".to_string());
    }
    argv
}

pub fn build_kill_argv(agent: &str) -> Vec<String> {
    vec!["kill".to_string(), agent.to_string()]
}

pub trait ActionRunner: Send + Sync {
    fn execute(&self, req: ActionRequest) -> Option<ActionResult>;
    fn cancel(&self) {}
}

type GitStatusCache = Arc<Mutex<std::collections::HashMap<String, (Instant, String, String)>>>;

/// `max_stdout` for output read in full (Events, Transcript, Screen tabs).
pub const NO_STDOUT_LIMIT: usize = usize::MAX - 1;

#[derive(Clone)]
pub struct SystemActionRunner {
    active_child: Arc<Mutex<Option<Child>>>,
    git_cache: GitStatusCache,
    usage_cache: Arc<Mutex<crate::usage::UsageCache>>,
    /// Time limit of `run_command_bounded` (30 s).
    pub command_timeout: Duration,
}

impl Default for SystemActionRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemActionRunner {
    pub fn new() -> Self {
        Self {
            active_child: Arc::new(Mutex::new(None)),
            git_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            usage_cache: Arc::default(),
            command_timeout: Duration::from_secs(30),
        }
    }

    pub fn run_command_bounded(
        &self,
        program: &str,
        argv: &[&str],
        max_stdout: usize,
        max_stderr: usize,
        cwd: Option<&str>,
    ) -> Result<String, String> {
        let mut cmd = Command::new(program);
        cmd.args(argv).stdout(Stdio::piped()).stderr(Stdio::piped());

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {} {}: {}", program, argv.join(" "), e))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Missing stdout pipe".to_string())?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Missing stderr pipe".to_string())?;

        // Put child in active_child so cancel() or drop can kill and reap it
        *self.active_child.lock().unwrap() = Some(child);

        // Concurrently drain stderr in a separate thread so child never blocks on stderr pipe
        let stderr_handle = spawn(move || {
            let mut err_buf = Vec::new();
            let mut take = Read::take(&mut stderr, (max_stderr + 1) as u64);
            let _ = take.read_to_end(&mut err_buf);
            let mut sink = std::io::sink();
            let _ = std::io::copy(&mut stderr, &mut sink);
            err_buf
        });

        // Read stdout up to max_stdout + 1 in a thread, so the timeout below
        // also covers a command that keeps writing.
        let mut stdout_handle = Some(spawn(move || {
            let mut buf = Vec::new();
            Read::take(&mut stdout, (max_stdout + 1) as u64)
                .read_to_end(&mut buf)
                .map(|_| buf)
        }));
        let mut stdout_buf: Option<Vec<u8>> = None;
        // Joins the stdout reader; an error kills the child.
        let finish_stdout = |handle: std::thread::JoinHandle<std::io::Result<Vec<u8>>>| {
            let res = match handle.join() {
                Ok(Ok(buf)) if buf.len() > max_stdout => Err(format!(
                    "Command {} output exceeded maximum allowed size of {} bytes",
                    program, max_stdout
                )),
                Ok(Ok(buf)) => Ok(buf),
                Ok(Err(e)) => Err(format!("Failed reading stdout: {}", e)),
                Err(_) => Err("Failed reading stdout: reader panicked".to_string()),
            };
            if res.is_err() {
                self.cancel();
            }
            res
        };

        // Wait for child process while keeping it cancellable until termination
        let start_time = Instant::now();
        let timeout = self.command_timeout;
        let status = loop {
            if stdout_buf.is_none() && stdout_handle.as_ref().is_some_and(|h| h.is_finished()) {
                match finish_stdout(stdout_handle.take().unwrap()) {
                    Ok(buf) => stdout_buf = Some(buf),
                    Err(e) => {
                        let _ = stderr_handle.join();
                        return Err(e);
                    }
                }
            }
            if start_time.elapsed() > timeout {
                self.cancel();
                let _ = stderr_handle.join();
                return Err(format!(
                    "Command {} timed out after {} seconds",
                    program,
                    timeout.as_secs()
                ));
            }
            let mut guard = self.active_child.lock().unwrap();
            if let Some(mut child) = guard.take() {
                match child.try_wait() {
                    Ok(Some(s)) => {
                        drop(guard);
                        break s;
                    }
                    Ok(None) => {
                        *guard = Some(child);
                        drop(guard);
                        sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        drop(guard);
                        let _ = stderr_handle.join();
                        return Err(format!(
                            "Wait failed on {} {}: {}",
                            program,
                            argv.join(" "),
                            e
                        ));
                    }
                }
            } else {
                let _ = stderr_handle.join();
                return Err("Command was cancelled".to_string());
            }
        };

        let stdout_buf = match (stdout_buf, stdout_handle) {
            (Some(buf), _) => buf,
            (None, Some(handle)) => match finish_stdout(handle) {
                Ok(buf) => buf,
                Err(e) => {
                    let _ = stderr_handle.join();
                    return Err(e);
                }
            },
            (None, None) => unreachable!("stdout is either read or still has its reader"),
        };
        let err_buf = stderr_handle.join().unwrap_or_default();
        let stdout_str = String::from_utf8_lossy(&stdout_buf).to_string();
        let stderr_str = String::from_utf8_lossy(&err_buf).trim().to_string();

        if status.success() {
            Ok(stdout_str)
        } else {
            let err = if !stderr_str.is_empty() {
                stderr_str
            } else if !stdout_str.trim().is_empty() {
                stdout_str.trim().to_string()
            } else {
                format!("Process exited with status {}", status)
            };
            Err(err)
        }
    }
}

impl ActionRunner for SystemActionRunner {
    fn execute(&self, req: ActionRequest) -> Option<ActionResult> {
        match req {
            ActionRequest::Attach { agent, custom_cmd } => {
                let res = if let Some(cmd) = custom_cmd {
                    if cmd.is_empty() {
                        Err("Empty attach command".into())
                    } else {
                        let argv_ref: Vec<&str> = cmd[1..].iter().map(|s| s.as_str()).collect();
                        self.run_command_bounded(&cmd[0], &argv_ref, 4096, 4096, None)
                    }
                } else {
                    let argv = build_attach_argv(&agent);
                    let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                    self.run_command_bounded("hcom", &argv_ref, 4096, 4096, None)
                };
                Some(ActionResult::AttachFinished {
                    agent,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::Inject { agent, text, enter } => {
                let argv = build_inject_argv(&agent, &text, enter);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 4096, 4096, None);
                Some(ActionResult::InjectFinished {
                    agent,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::Transcript {
                agent,
                generation,
                last_lines,
            } => {
                let argv = build_transcript_argv(&agent, last_lines);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, NO_STDOUT_LIMIT, 4096, None);
                Some(ActionResult::TranscriptLoaded {
                    agent,
                    generation,
                    content: res,
                })
            }
            ActionRequest::SessionTranscript {
                agent,
                generation,
                path,
            } => {
                let res = self.run_command_bounded(
                    "hcom",
                    &["transcript", "--path", &path, "--last", "100"],
                    NO_STDOUT_LIMIT,
                    4096,
                    None,
                );
                Some(ActionResult::TranscriptLoaded {
                    agent,
                    generation,
                    content: res,
                })
            }
            ActionRequest::Sessions { agent, generation } => {
                let life = self.run_command_bounded(
                    "hcom",
                    &[
                        "events", "--type", "life", "--agent", &agent, "--full", "--last", "100000",
                    ],
                    16 * 1024 * 1024,
                    4096,
                    None,
                );
                let msgs = self.run_command_bounded(
                    "hcom",
                    &[
                        "events",
                        "--type",
                        "message",
                        "--participant",
                        &agent,
                        "--full",
                        "--last",
                        "100000",
                    ],
                    16 * 1024 * 1024,
                    4096,
                    None,
                );
                let live = self
                    .run_command_bounded("hcom", &["list", "--json"], 1024 * 1024, 4096, None)
                    .ok()
                    .and_then(|out| {
                        serde_json::from_str::<Vec<crate::table_data::LiveInstanceItem>>(&out).ok()
                    })
                    .and_then(|items| {
                        items
                            .into_iter()
                            .find(|i| i.name == agent && i.status.as_deref() != Some("stopped"))
                    })
                    .map(|i| crate::sessions::LiveSession {
                        session_id: i.session_id.unwrap_or_default(),
                        transcript_path: i.transcript_path.unwrap_or_default(),
                        tool: i.tool.unwrap_or_default(),
                        directory: i.directory.unwrap_or_default(),
                    });
                let sessions = life.and_then(|life| {
                    let msgs = msgs?;
                    let now = chrono::Utc::now().timestamp() as u64;
                    let mut list =
                        crate::sessions::parse_sessions(&agent, &life, &msgs, live.as_ref(), now);
                    for s in list.iter_mut().filter(|s| !s.transcript_path.is_empty()) {
                        s.transcript_size =
                            std::fs::metadata(&s.transcript_path).ok().map(|m| m.len());
                    }
                    Ok(list)
                });
                Some(ActionResult::SessionsLoaded {
                    agent,
                    generation,
                    sessions,
                })
            }
            ActionRequest::Screen { agent, generation } => {
                let res = self.run_command_bounded(
                    "hcom",
                    &["term", &agent],
                    NO_STDOUT_LIMIT,
                    4096,
                    None,
                );
                Some(ActionResult::ScreenLoaded {
                    agent,
                    generation,
                    content: res,
                })
            }
            ActionRequest::Events {
                agent,
                generation,
                last_n,
            } => {
                let last_str = last_n.clamp(1, 100).to_string();
                let res = self.run_command_bounded(
                    "hcom",
                    &["events", "--agent", &agent, "--last", &last_str],
                    NO_STDOUT_LIMIT,
                    4096,
                    None,
                );
                Some(ActionResult::EventsLoaded {
                    agent,
                    generation,
                    content: res,
                })
            }
            ActionRequest::Kill { agent } => {
                let argv = build_kill_argv(&agent);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 4096, 4096, None);
                Some(ActionResult::KillFinished {
                    agent,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::KillGroup { agents } => {
                let count = agents.len();
                let mut argv: Vec<String> = vec!["kill".into()];
                argv.extend(agents);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 8192, 4096, None);
                Some(ActionResult::KillGroupFinished {
                    count,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::RunCommand { cmd, cwd } => {
                let res = if cmd.is_empty() {
                    Err("Empty command".into())
                } else {
                    let argv_ref: Vec<&str> = cmd[1..].iter().map(|s| s.as_str()).collect();
                    self.run_command_bounded(&cmd[0], &argv_ref, 32768, 4096, cwd.as_deref())
                };
                Some(ActionResult::CommandFinished { result: res })
            }
            ActionRequest::SpawnDetached { cmd, cwd } => {
                if cmd.is_empty() {
                    return Some(ActionResult::SpawnDetachedFinished {
                        cmd,
                        result: Err("Empty command".into()),
                    });
                }
                let mut command = Command::new(&cmd[0]);
                command.args(&cmd[1..]);

                let effective_cwd = if let Some(ref dir) = cwd {
                    if let Some(stripped) = dir.strip_prefix("~/") {
                        if let Ok(home) = std::env::var("HOME") {
                            format!("{}/{}", home, stripped)
                        } else {
                            dir.clone()
                        }
                    } else if dir == "~" {
                        std::env::var("HOME").unwrap_or_else(|_| dir.clone())
                    } else {
                        dir.clone()
                    }
                } else {
                    std::env::var("HOME").unwrap_or_default()
                };

                if !effective_cwd.is_empty() {
                    command.current_dir(effective_cwd);
                }
                command.stdin(Stdio::null());
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());
                let res = match command.spawn() {
                    Ok(mut child) => {
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                        Ok(())
                    }
                    Err(e) => Err(format!("Failed to spawn {}: {}", cmd.join(" "), e)),
                };
                Some(ActionResult::SpawnDetachedFinished { cmd, result: res })
            }
            ActionRequest::ReloadTableData => {
                let live_res =
                    self.run_command_bounded("hcom", &["list", "--json"], 1024 * 1024, 4096, None);
                let agent_res = self.run_command_bounded(
                    "hcom",
                    &["agent", "list", "--all", "--status", "--json"],
                    1024 * 1024,
                    4096,
                    None,
                );

                let (live_out, agent_out) = match (live_res, agent_res) {
                    (Ok(l), Ok(a)) => (l, a),
                    (Err(e), _) | (_, Err(e)) => {
                        return Some(ActionResult::TableDataFailed {
                            error: format!("Failed to fetch live/agent data: {}", e),
                        });
                    }
                };

                let now = chrono::Utc::now().timestamp() as u64;
                let mut rows = match crate::table_data::build_table_rows(&live_out, &agent_out, now)
                {
                    Ok(r) => r,
                    Err(e) => {
                        return Some(ActionResult::TableDataFailed {
                            error: format!("Failed to parse table rows: {}", e),
                        });
                    }
                };

                crate::usage::apply_usage(&mut rows, &mut self.usage_cache.lock().unwrap());

                // Enrich Git status for unique directories with 10s TTL
                let mut seen_dirs = std::collections::HashSet::new();
                for r in &rows {
                    if !r.directory.is_empty() {
                        seen_dirs.insert(r.directory.clone());
                    }
                }
                for dir in seen_dirs {
                    let cached = {
                        let cache = self.git_cache.lock().unwrap();
                        cache.get(&dir).and_then(|(inst, b, s)| {
                            if inst.elapsed() < Duration::from_secs(10) {
                                Some((b.clone(), s.clone()))
                            } else {
                                None
                            }
                        })
                    };

                    let (b, s) = if let Some(c) = cached {
                        c
                    } else if let Ok(git_out) = self.run_command_bounded(
                        "git",
                        &["-C", &dir, "status", "--porcelain=v2", "--branch"],
                        256 * 1024,
                        4096,
                        None,
                    ) {
                        let parsed = crate::table_data::parse_git_porcelain_v2(&git_out);
                        let mut cache = self.git_cache.lock().unwrap();
                        cache.insert(
                            dir.clone(),
                            (Instant::now(), parsed.0.clone(), parsed.1.clone()),
                        );
                        parsed
                    } else {
                        (String::new(), String::new())
                    };

                    for r in rows.iter_mut().filter(|r| r.directory == dir) {
                        if r.branch.is_empty() && !b.is_empty() {
                            r.branch = b.clone();
                        }
                        r.git_status = s.clone();
                    }
                }

                let mut groups_set = std::collections::HashSet::new();
                let mut tool_set = std::collections::HashSet::new();

                if let Ok(agents) =
                    serde_json::from_str::<Vec<crate::table_data::CatalogAgentItem>>(&agent_out)
                {
                    for a in agents {
                        for g in a.groups {
                            groups_set.insert(g);
                        }
                        if let Some(c) = a.cli
                            && !c.is_empty()
                        {
                            tool_set.insert(c);
                        }
                    }
                }
                for r in &rows {
                    if !r.cli.is_empty() {
                        tool_set.insert(r.cli.clone());
                    }
                }

                let mut catalog_groups: Vec<String> = groups_set.into_iter().collect();
                catalog_groups.sort();
                let mut tool_groups: Vec<String> = tool_set.into_iter().collect();
                tool_groups.sort();

                Some(ActionResult::TableDataLoaded {
                    rows,
                    catalog_groups,
                    tool_groups,
                })
            }
            ActionRequest::ReloadStats => {
                let life_res = self.run_command_bounded(
                    "hcom",
                    &["events", "--type", "life", "--last", "100000"],
                    16 * 1024 * 1024,
                    4096,
                    None,
                );
                let msg_res = self.run_command_bounded(
                    "hcom",
                    &["events", "--type", "message", "--full", "--last", "100000"],
                    16 * 1024 * 1024,
                    4096,
                    None,
                );
                let live_res =
                    self.run_command_bounded("hcom", &["list", "--json"], 1024 * 1024, 4096, None);

                let (life_events, msg_events) = match (life_res, msg_res) {
                    (Ok(l), Ok(m)) => (l, m),
                    (Err(e), _) | (_, Err(e)) => {
                        return Some(ActionResult::StatsFailed {
                            error: format!("Failed to load events for usage stats: {}", e),
                        });
                    }
                };

                let running_set: std::collections::HashSet<String> = live_res
                    .ok()
                    .and_then(|out| {
                        serde_json::from_str::<Vec<crate::table_data::LiveInstanceItem>>(&out).ok()
                    })
                    .map(|items| {
                        items
                            .into_iter()
                            .filter(|i| i.status.as_deref() != Some("stopped"))
                            .map(|i| i.name)
                            .collect()
                    })
                    .unwrap_or_default();

                let now = chrono::Utc::now().timestamp() as u64;
                let stats = crate::table_data::compute_event_stats(
                    &life_events,
                    &msg_events,
                    &running_set,
                    now,
                );
                Some(ActionResult::StatsLoaded { stats })
            }
            ActionRequest::Shutdown => None,
        }
    }

    fn cancel(&self) {
        if let Some(mut child) = self.active_child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub struct ActionWorkerHandle {
    tx: SyncSender<ActionRequest>,
    rx: Receiver<ActionResult>,
    thread_handle: Option<JoinHandle<()>>,
    cancel_fn: Option<Box<dyn Fn() + Send + Sync>>,
}

impl ActionWorkerHandle {
    pub fn new<R: ActionRunner + 'static>(runner: R) -> Self {
        let runner = Arc::new(runner);
        let (req_tx, req_rx) = sync_channel::<ActionRequest>(16);
        let (res_tx, res_rx) = std::sync::mpsc::channel::<ActionResult>();

        let runner_thread = Arc::clone(&runner);
        let thread_handle = spawn(move || {
            while let Ok(req) = req_rx.recv() {
                if req == ActionRequest::Shutdown {
                    break;
                }
                if let Some(res) = runner_thread.execute(req) {
                    let _ = res_tx.send(res);
                }
            }
        });

        let runner_cancel = Arc::clone(&runner);
        let cancel_fn = Box::new(move || {
            runner_cancel.cancel();
        });

        Self {
            tx: req_tx,
            rx: res_rx,
            thread_handle: Some(thread_handle),
            cancel_fn: Some(cancel_fn),
        }
    }

    pub fn try_send(&self, req: ActionRequest) -> Result<(), String> {
        self.tx
            .try_send(req)
            .map_err(|e| format!("Action channel full or disconnected: {}", e))
    }

    pub fn try_recv(&self) -> Option<ActionResult> {
        self.rx.try_recv().ok()
    }
}

impl Drop for ActionWorkerHandle {
    fn drop(&mut self) {
        let _ = self.tx.try_send(ActionRequest::Shutdown);
        if let Some(cancel) = self.cancel_fn.take() {
            cancel();
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRequest {
    Attach { agent: String },
    Inject { agent: String, text: String },
    Transcript { agent: String, last_lines: usize },
    Kill { agent: String },
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
        content: Result<String, String>,
    },
    KillFinished {
        agent: String,
        result: Result<(), String>,
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

pub fn build_inject_argv(agent: &str, text: &str) -> Vec<String> {
    vec![
        "term".to_string(),
        "inject".to_string(),
        agent.to_string(),
        text.to_string(),
        "--enter".to_string(),
    ]
}

pub fn build_kill_argv(agent: &str) -> Vec<String> {
    vec!["kill".to_string(), agent.to_string()]
}

pub trait ActionRunner: Send + Sync {
    fn execute(&self, req: ActionRequest) -> Option<ActionResult>;
    fn cancel(&self) {}
}

#[derive(Clone, Default)]
pub struct SystemActionRunner {
    active_child: Arc<Mutex<Option<Child>>>,
}

impl SystemActionRunner {
    pub fn new() -> Self {
        Self {
            active_child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn run_command_bounded(
        &self,
        program: &str,
        argv: &[&str],
        max_stdout: usize,
        max_stderr: usize,
    ) -> Result<String, String> {
        let mut child = Command::new(program)
            .args(argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
            // Drain remaining stderr to sink to prevent child from blocking on stderr write
            let mut sink = std::io::sink();
            let _ = std::io::copy(&mut stderr, &mut sink);
            err_buf
        });

        // Concurrently read stdout up to max_stdout + 1
        let mut stdout_buf = Vec::new();
        let read_bytes =
            match Read::take(&mut stdout, (max_stdout + 1) as u64).read_to_end(&mut stdout_buf) {
                Ok(n) => n,
                Err(e) => {
                    self.cancel();
                    let _ = stderr_handle.join();
                    return Err(format!("Failed reading stdout: {}", e));
                }
            };

        // If stdout exceeded limit, kill child immediately and reap
        if read_bytes > max_stdout {
            self.cancel();
            let _ = stderr_handle.join();
            return Err(format!(
                "Command {} output exceeded maximum allowed size of {} bytes",
                program, max_stdout
            ));
        }

        // Wait for child process while keeping it cancellable until termination
        let status = loop {
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
            ActionRequest::Attach { agent } => {
                let argv = build_attach_argv(&agent);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 4096, 4096);
                Some(ActionResult::AttachFinished {
                    agent,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::Inject { agent, text } => {
                let argv = build_inject_argv(&agent, &text);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 4096, 4096);
                Some(ActionResult::InjectFinished {
                    agent,
                    result: res.map(|_| ()),
                })
            }
            ActionRequest::Transcript { agent, last_lines } => {
                let argv = build_transcript_argv(&agent, last_lines);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 32768, 4096);
                Some(ActionResult::TranscriptLoaded {
                    agent,
                    content: res,
                })
            }
            ActionRequest::Kill { agent } => {
                let argv = build_kill_argv(&agent);
                let argv_ref: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                let res = self.run_command_bounded("hcom", &argv_ref, 4096, 4096);
                Some(ActionResult::KillFinished {
                    agent,
                    result: res.map(|_| ()),
                })
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
        // Bounded sync channel (capacity 16) to prevent UI spamming/backpressure
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

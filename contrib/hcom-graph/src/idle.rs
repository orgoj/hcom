//! Idle column: the class of each listening agent from
//! `contrib/idle-classifier/idle_check.py` (port of hcom-gui `idle_periods`,
//! `update_idle`, `run_idle`, `idle_check`).

use std::collections::HashMap;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

pub const IDLE_CLASSES: &[&str] = &["needs_user", "waiting", "done"];
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(180);
pub const IDLE_START_TOLERANCE: u64 = 2;
pub const IDLE_MAX_NAMES: usize = 32;
pub const QUEUED: &str = "…";
pub const NO_CLASS: &str = "-";
const MAX_STDOUT: usize = 1 << 20;
const MAX_STDERR: usize = 64 << 10;

/// hcom-graph is built only from this repository, next to the classifier.
pub const IDLE_CHECK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../idle-classifier/idle_check.py"
);

/// One listening period of an instance.
#[derive(Debug, Clone, PartialEq)]
pub struct IdlePeriod {
    /// Epoch seconds the period started.
    pub start: u64,
    /// Start counted from first sight (remote instances report age 0).
    pub fallback: bool,
    /// `None` not yet due, `"…"` queued, else the class or `"-"`.
    pub value: Option<String>,
}

/// Sort rank of an Idle cell (hcom-gui IDLE_RANK).
pub fn idle_rank(value: &str) -> u8 {
    match value {
        "needs_user" => 0,
        "waiting" => 1,
        "done" => 2,
        NO_CLASS => 3,
        QUEUED => 4,
        _ => 5,
    }
}

/// The listening period of each listening instance, kept from `periods` while
/// it goes on. `listening` is (instance, status age seconds) of every
/// listening row.
pub fn update_periods(
    periods: &HashMap<String, IdlePeriod>,
    listening: &[(String, u64)],
    now: u64,
) -> HashMap<String, IdlePeriod> {
    let mut current = HashMap::new();
    for (name, since) in listening {
        let prev = periods.get(name);
        let p = if *since > 0 {
            let start = now.saturating_sub(*since);
            match prev {
                Some(p)
                    if (p.fallback && start.abs_diff(p.start) <= IDLE_START_TOLERANCE)
                        || (!p.fallback && start <= p.start + IDLE_START_TOLERANCE) =>
                {
                    let mut p = p.clone();
                    // A local agent first seen at age 0: its real start from now on.
                    if p.fallback {
                        p.start = start;
                        p.fallback = false;
                    }
                    p
                }
                _ => IdlePeriod {
                    start,
                    fallback: false,
                    value: None,
                },
            }
        } else {
            match prev {
                Some(p) if p.fallback => p.clone(),
                // None yet, or a local agent that re-entered listening.
                _ => IdlePeriod {
                    start: now,
                    fallback: true,
                    value: None,
                },
            }
        };
        current.insert(name.clone(), p);
    }
    current
}

/// Queue every period that lasted `idle_seconds` without an attempt.
pub fn queue_due(periods: &mut HashMap<String, IdlePeriod>, now: u64, idle_seconds: u64) {
    for p in periods.values_mut() {
        if p.value.is_none() && now.saturating_sub(p.start) >= idle_seconds {
            p.value = Some(QUEUED.to_string());
        }
    }
}

/// Names of queued periods for the next run, sorted, at most IDLE_MAX_NAMES.
pub fn next_batch(periods: &HashMap<String, IdlePeriod>) -> Vec<String> {
    let mut names: Vec<String> = periods
        .iter()
        .filter(|(_, p)| p.value.as_deref() == Some(QUEUED))
        .map(|(n, _)| n.clone())
        .collect();
    names.sort();
    names.truncate(IDLE_MAX_NAMES);
    names
}

/// Each period of the run gets its class or `"-"` (no retry in the same
/// period); a period that ended meanwhile stays dropped.
pub fn apply_result(
    periods: &mut HashMap<String, IdlePeriod>,
    batch: &[String],
    classes: &HashMap<String, String>,
) {
    for name in batch {
        if let Some(p) = periods.get_mut(name)
            && p.value.as_deref() == Some(QUEUED)
        {
            p.value = Some(
                classes
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| NO_CLASS.to_string()),
            );
        }
    }
}

/// `{name: class}` from idle_check.py `--json` output; unknown classes are left out.
pub fn parse_output(stdout: &str) -> Result<HashMap<String, String>, String> {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(stdout).map_err(|_| "idle: output invalid".to_string())?;
    let mut classes = HashMap::new();
    for r in rows {
        let Some(name) = r["name"].as_str() else {
            return Err("idle: output invalid".to_string());
        };
        if let Some(cls) = r["class"].as_str()
            && IDLE_CLASSES.contains(&cls)
        {
            classes.insert(name.to_string(), cls.to_string());
        }
    }
    Ok(classes)
}

/// Error text from idle_check's own last `idle_check: ` stderr line only, never
/// other output (message text stays private).
pub fn error_text(stderr: &str, code: Option<i32>) -> String {
    match stderr
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("idle_check: "))
    {
        Some(msg) => format!("idle: {msg}"),
        None => match code {
            Some(c) => format!("idle: exit {c}"),
            None => "idle: killed".to_string(),
        },
    }
}

/// A running idle_check: one thread, the child in its own process group.
/// Timeout and drop kill the whole group.
pub struct IdleJob {
    pub batch: Vec<String>,
    rx: Receiver<Result<HashMap<String, String>, String>>,
    child: Arc<Mutex<Option<Child>>>,
}

impl IdleJob {
    pub fn start(batch: Vec<String>) -> Self {
        let mut argv = vec![
            "-I".to_string(),
            IDLE_CHECK.to_string(),
            "--json".to_string(),
        ];
        argv.push("--".to_string());
        argv.extend(batch.iter().cloned());
        Self::spawn("/usr/bin/python3", &argv, IDLE_TIMEOUT, batch)
    }

    /// `program argv` with the job's limits; the result is parsed idle_check output.
    pub fn spawn(program: &str, argv: &[String], timeout: Duration, batch: Vec<String>) -> Self {
        let (tx, rx) = channel();
        let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
        let mut cmd = Command::new(program);
        cmd.args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        if let Ok(home) = std::env::var("HOME") {
            cmd.current_dir(home);
        }
        match cmd.spawn() {
            Err(e) => {
                let _ = tx.send(Err(format!("idle: {}", e.kind())));
            }
            Ok(mut c) => {
                // The group id is the child's pid (process_group(0)).
                let pgid = c.id() as libc::pid_t;
                let pipes = (c.stdout.take(), c.stderr.take());
                *child.lock().unwrap() = Some(c);
                let job_child = Arc::clone(&child);
                spawn(move || {
                    let res = match pipes {
                        (Some(out), Some(err)) => run(job_child, pgid, out, err, timeout),
                        _ => {
                            kill_group(&job_child);
                            Err("idle: missing pipes".to_string())
                        }
                    };
                    let _ = tx.send(res);
                });
            }
        }
        Self { batch, rx, child }
    }

    /// The result once the run finished.
    pub fn try_result(&self) -> Option<Result<HashMap<String, String>, String>> {
        self.rx.try_recv().ok()
    }
}

impl Drop for IdleJob {
    fn drop(&mut self) {
        kill_group(&self.child);
    }
}

fn kill_group(child: &Mutex<Option<Child>>) {
    if let Some(mut c) = child.lock().unwrap().take() {
        // SAFETY: plain kill(2) on the job's own process group (pid of the
        // still unreaped leader, so the id cannot have been reused).
        unsafe {
            libc::kill(-(c.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = c.wait();
    }
}

/// Has the child exited? Observed with `WNOWAIT`, so the child stays an
/// unreaped zombie and its pid (the group id) stays reserved until `wait`.
fn leader_exited(pid: libc::pid_t) -> Result<bool, ()> {
    // SAFETY: waitid(2) with a zeroed siginfo_t out-parameter it fills in.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if rc != 0 {
        return Err(());
    }
    // WNOHANG with no state change leaves si_pid 0.
    Ok(unsafe { info.si_pid() } != 0)
}

/// Read a pipe to EOF; bytes past `cap` are discarded and set `over`.
fn drain(
    mut pipe: impl Read + Send + 'static,
    cap: usize,
    over: Arc<AtomicBool>,
) -> std::thread::JoinHandle<Vec<u8>> {
    spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) if kept.len() + n > cap => over.store(true, Ordering::SeqCst),
                Ok(n) => kept.extend_from_slice(&buf[..n]),
            }
        }
        kept
    })
}

fn run(
    child: Arc<Mutex<Option<Child>>>,
    pgid: libc::pid_t,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
    timeout: Duration,
) -> Result<HashMap<String, String>, String> {
    let over = Arc::new(AtomicBool::new(false));
    let out = drain(stdout, MAX_STDOUT, Arc::clone(&over));
    let err = drain(stderr, MAX_STDERR, Arc::clone(&over));
    let start = Instant::now();
    let outcome = loop {
        if over.load(Ordering::SeqCst) {
            kill_group(&child);
            break Err("idle: output too large".to_string());
        }
        if start.elapsed() > timeout {
            kill_group(&child);
            break Err(format!("idle: timeout after {} s", timeout.as_secs()));
        }
        let mut guard = child.lock().unwrap();
        let Some(c) = guard.as_mut() else {
            // Dropped: the job's owner killed the group.
            break Err("idle: cancelled".to_string());
        };
        match leader_exited(c.id() as libc::pid_t) {
            Ok(true) => {
                // The exited leader is still an unreaped zombie, so the group id
                // cannot be reused yet: kill leftover members (e.g. a scorer
                // child) that would outlive the run or hold the pipes, then reap.
                // SAFETY: plain kill(2) on the job's own process group.
                unsafe {
                    libc::kill(-pgid, libc::SIGKILL);
                }
                let status = guard.take().map(|mut c| c.wait());
                match status {
                    Some(Ok(s)) => break Ok(s),
                    _ => break Err("idle: wait failed".to_string()),
                }
            }
            Ok(false) => {}
            Err(()) => {
                drop(guard);
                kill_group(&child);
                break Err("idle: wait failed".to_string());
            }
        }
        drop(guard);
        sleep(Duration::from_millis(50));
    };
    let out_bytes = out.join().unwrap_or_default();
    let err_bytes = err.join().unwrap_or_default();
    let status = outcome?;
    if over.load(Ordering::SeqCst) {
        return Err("idle: output too large".to_string());
    }
    if !status.success() {
        return Err(error_text(
            &String::from_utf8_lossy(&err_bytes),
            status.code(),
        ));
    }
    parse_output(&String::from_utf8_lossy(&out_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(start: u64, fallback: bool, value: Option<&str>) -> IdlePeriod {
        IdlePeriod {
            start,
            fallback,
            value: value.map(str::to_string),
        }
    }

    #[test]
    fn periods_start_continue_and_end() {
        let p = update_periods(&HashMap::new(), &[("a".into(), 5)], 100);
        assert_eq!(p["a"], period(95, false, None));
        let mut p = p;
        p.get_mut("a").unwrap().value = Some("done".into());
        // Same period one refresh later (1 s of clock jitter allowed).
        let p = update_periods(&p, &[("a".into(), 16)], 110);
        assert_eq!(p["a"], period(95, false, Some("done")));
        // A new, later listening period starts fresh.
        let p2 = update_periods(&p, &[("a".into(), 1)], 200);
        assert_eq!(p2["a"], period(199, false, None));
        // Not listening any more: dropped.
        assert!(update_periods(&p, &[], 120).is_empty());
    }

    #[test]
    fn remote_age_zero_counts_from_first_sight_until_a_real_age_arrives() {
        let p = update_periods(&HashMap::new(), &[("r".into(), 0)], 100);
        assert_eq!(p["r"], period(100, true, None));
        let p = update_periods(&p, &[("r".into(), 0)], 130);
        assert_eq!(p["r"], period(100, true, None));
        // A local agent first seen at age 0 gets its real start.
        let p = update_periods(&p, &[("r".into(), 31)], 131);
        assert_eq!(p["r"], period(100, false, None));
    }

    #[test]
    fn due_periods_are_queued_batched_and_classified_once() {
        let mut p: HashMap<String, IdlePeriod> = (0..40)
            .map(|i| (format!("n{i:02}"), period(100, false, None)))
            .collect();
        p.insert("fresh".into(), period(105, false, None));
        queue_due(&mut p, 110, 10);
        assert_eq!(p["fresh"].value, None);
        let batch = next_batch(&p);
        assert_eq!(batch.len(), IDLE_MAX_NAMES);
        let classes = HashMap::from([("n00".to_string(), "needs_user".to_string())]);
        apply_result(&mut p, &batch, &classes);
        assert_eq!(p["n00"].value.as_deref(), Some("needs_user"));
        assert_eq!(p["n01"].value.as_deref(), Some(NO_CLASS));
        // The rest forms the next run.
        assert_eq!(next_batch(&p).len(), 40 - IDLE_MAX_NAMES);
        assert!(idle_rank("needs_user") < idle_rank(NO_CLASS));
        assert!(idle_rank(QUEUED) < idle_rank(""));
    }

    #[test]
    fn output_and_error_parsing() {
        let out = r#"[{"name":"a","class":"waiting"},{"name":"b","class":"bogus"},{"name":"c","class":null}]"#;
        let c = parse_output(out).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c["a"], "waiting");
        assert!(parse_output("not json").is_err());
        assert_eq!(
            error_text("secret message text\nidle_check: no model\n", Some(2)),
            "idle: no model"
        );
        assert_eq!(error_text("secret message text", Some(2)), "idle: exit 2");
    }

    fn sh(script: &str, timeout: Duration) -> IdleJob {
        IdleJob::spawn(
            "/bin/sh",
            &["-c".to_string(), script.to_string()],
            timeout,
            Vec::new(),
        )
    }

    fn wait_result(job: &IdleJob) -> Result<HashMap<String, String>, String> {
        let start = Instant::now();
        loop {
            if let Some(r) = job.try_result() {
                return r;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "no result");
            sleep(Duration::from_millis(20));
        }
    }

    /// Live (not gone, not a zombie) within 3 s?
    fn stays_alive(pid: &str) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(_) => return false,
                Ok(s)
                    if s.rsplit(')')
                        .next()
                        .unwrap_or("")
                        .trim_start()
                        .starts_with('Z') =>
                {
                    return false;
                }
                Ok(_) => sleep(Duration::from_millis(20)),
            }
        }
        true
    }

    fn grandchild_pid(file: &std::path::Path) -> String {
        let start = Instant::now();
        loop {
            if let Ok(s) = std::fs::read_to_string(file)
                && !s.trim().is_empty()
            {
                return s.trim().to_string();
            }
            assert!(start.elapsed() < Duration::from_secs(5), "no pid file");
            sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn success_output_is_parsed() {
        let job = sh(
            r#"echo '[{"name":"a","class":"done"}]'"#,
            Duration::from_secs(10),
        );
        assert_eq!(wait_result(&job).unwrap()["a"], "done");
    }

    #[test]
    fn timeout_kills_the_whole_group() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let job = sh(
            &format!("sleep 60 & echo $! > {}; sleep 60", pidfile.display()),
            Duration::from_secs(1),
        );
        let pid = grandchild_pid(&pidfile);
        assert_eq!(wait_result(&job).unwrap_err(), "idle: timeout after 1 s");
        assert!(!stays_alive(&pid));
    }

    #[test]
    fn drop_kills_the_whole_group() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let job = sh(
            &format!("sleep 60 & echo $! > {}; sleep 60", pidfile.display()),
            Duration::from_secs(60),
        );
        let pid = grandchild_pid(&pidfile);
        drop(job);
        assert!(!stays_alive(&pid));
    }

    #[test]
    fn oversized_output_kills_the_group_without_surfacing_it() {
        let job = sh(
            "head -c 2000000 /dev/zero; sleep 60",
            Duration::from_secs(60),
        );
        assert_eq!(wait_result(&job).unwrap_err(), "idle: output too large");
    }

    #[test]
    fn leftover_group_members_are_killed_after_the_leader_exits() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let job = sh(
            &format!("sleep 60 & echo $! > {}; echo '[]'", pidfile.display()),
            Duration::from_secs(60),
        );
        assert!(wait_result(&job).unwrap().is_empty());
        assert!(!stays_alive(&grandchild_pid(&pidfile)));
    }
}

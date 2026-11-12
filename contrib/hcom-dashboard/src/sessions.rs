//! Sessions of one agent built from its hcom life and message events
//! (TASK-77.69): one session runs from a `created` event to its `stopped`.

use crate::table_data::parse_event_ts;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Messages exchanged with one partner during a session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Partner {
    pub name: String,
    pub sent: u64,
    pub received: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionInfo {
    pub start_ts: u64,
    /// None while the session runs.
    pub end_ts: Option<u64>,
    /// Agent that launched the session, "user", or "handoff".
    pub launcher: String,
    /// `reason (by)` of the `stopped` event.
    pub end_reason: String,
    pub session_id: String,
    pub transcript_path: String,
    /// Transcript file size; None when the file is missing.
    pub transcript_size: Option<u64>,
    pub tool: String,
    pub directory: String,
    /// Most messages first.
    pub partners: Vec<Partner>,
    /// Session id this one continues by handoff (empty: none).
    pub handoff_from: String,
    /// Session id that continued this one by handoff (empty: none).
    pub handoff_to: String,
    /// Claude /rename or AI title, Codex thread name (empty: none).
    pub title: String,
    /// Arguments hcom launched the CLI with (empty: none recorded).
    pub launch_args: Vec<String>,
}

/// The running session as `hcom list --json` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveSession {
    pub instance_name: String,
    pub session_id: String,
    pub transcript_path: String,
    pub tool: String,
    pub directory: String,
    pub launch_args: Vec<String>,
}

/// Tools `hcom r <session-id>` and `hcom f <session-id>` accept.
const RESUME_TOOLS: &[&str] = &["claude", "codex", "gemini", "opencode", "kilo"];
const FORK_TOOLS: &[&str] = &["claude", "codex", "opencode", "kilo"];

/// `1.2 MB`-style size.
pub fn format_size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.0} KB", b as f64 / 1024.0),
        b => format!("{b} B"),
    }
}

fn s(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

fn jsonl(text: &str) -> Vec<Value> {
    let mut out: Vec<Value> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l.trim()).ok())
        .collect();
    out.sort_by_key(|e| e.get("id").and_then(Value::as_u64).unwrap_or(0));
    out
}

/// Checks whether an instance name belongs to a catalog agent (exact match,
/// roaming `<agent>_<project>_<hash>`, clone `<agent>__<suffix>`, or `<agent>-<num>`).
pub fn instance_belongs_to_agent(instance: &str, agent: &str) -> bool {
    instance == agent
        || instance.starts_with(&format!("{agent}_"))
        || instance.starts_with(&format!("{agent}__"))
        || instance.starts_with(&format!("{agent}-"))
}

/// Sessions of `agent`, newest first. `live_sessions` describes running sessions
/// (the last one without a `stopped` event); `now` ends it for partner counts.
pub fn parse_sessions(
    agent: &str,
    life_jsonl: &str,
    msg_jsonl: &str,
    live_sessions: &[LiveSession],
    now: u64,
) -> Vec<SessionInfo> {
    let mut sessions: Vec<SessionInfo> = Vec::new();
    let mut open: HashMap<String, SessionInfo> = HashMap::new();
    let mut handoffs: HashSet<String> = HashSet::new();
    for e in jsonl(life_jsonl) {
        let inst = s(e.get("instance"));
        let snap_agent = e
            .get("data")
            .and_then(|d| d.get("snapshot"))
            .and_then(|s| s.get("agent_name"))
            .and_then(Value::as_str)
            .or_else(|| {
                e.get("data")
                    .and_then(|d| d.get("launch_context"))
                    .and_then(|lc| lc.get("agent_name"))
                    .and_then(Value::as_str)
            });

        let matches_agent = instance_belongs_to_agent(&inst, agent)
            || snap_agent == Some(agent)
            || live_sessions.iter().any(|l| l.instance_name == inst);
        if !matches_agent {
            continue;
        }
        let ts = parse_event_ts(e.get("ts").and_then(Value::as_str).unwrap_or(""));
        let data = e.get("data");
        let field = |k: &str| s(data.and_then(|d| d.get(k)));
        match field("action").as_str() {
            "handoff" => {
                handoffs.insert(inst);
            }
            "created" => {
                let is_handoff = handoffs.remove(&inst);
                if let Some(mut prev) = open.remove(&inst) {
                    prev.end_ts = Some(ts);
                    sessions.push(prev);
                }
                open.insert(
                    inst.clone(),
                    SessionInfo {
                        start_ts: ts,
                        launcher: if is_handoff {
                            "handoff".into()
                        } else {
                            String::new()
                        },
                        ..Default::default()
                    },
                );
            }
            "ready" => {
                if let Some(cur) = open.get_mut(&inst)
                    && cur.launcher.is_empty()
                {
                    let by = field("by");
                    cur.launcher = if matches!(by.as_str(), "" | "api" | "unknown" | "cli") {
                        "user".into()
                    } else {
                        by
                    };
                }
            }
            "stopped" => {
                let is_handoff = handoffs.contains(&inst);
                let snap = data.and_then(|d| d.get("snapshot"));
                let snap_field = |k: &str| s(snap.and_then(|v| v.get(k)));
                let mut cur = open.remove(&inst).unwrap_or_else(|| SessionInfo {
                    start_ts: snap
                        .and_then(|v| v.get("created_at"))
                        .and_then(Value::as_f64)
                        .map(|f| f as u64)
                        .unwrap_or(ts),
                    ..Default::default()
                });
                cur.end_ts = Some(ts);
                let (reason, by) = (field("reason"), field("by"));
                cur.end_reason = if is_handoff {
                    "handoff".into()
                } else {
                    match (reason.is_empty(), by.is_empty()) {
                        (false, false) => format!("{reason} ({by})"),
                        (false, true) => reason,
                        _ => by,
                    }
                };
                cur.session_id = snap_field("session_id");
                cur.transcript_path = snap_field("transcript_path");
                cur.tool = snap_field("tool");
                cur.directory = snap_field("directory");
                // Stored as a JSON array encoded in a string.
                cur.launch_args =
                    serde_json::from_str(&snap_field("launch_args")).unwrap_or_default();
                sessions.push(cur);
            }
            _ => {}
        }
    }
    for (inst, mut cur) in open {
        if let Some(l) = live_sessions.iter().find(|l| {
            l.instance_name == inst || (l.instance_name.is_empty() && live_sessions.len() == 1)
        }) {
            cur.session_id = l.session_id.clone();
            cur.transcript_path = l.transcript_path.clone();
            cur.tool = l.tool.clone();
            cur.directory = l.directory.clone();
            cur.launch_args = l.launch_args.clone();
        }
        sessions.push(cur);
    }
    sessions.sort_by_key(|s| s.start_ts);
    // A handoff session continues the one before it.
    for i in 1..sessions.len() {
        if sessions[i].launcher == "handoff" {
            sessions[i].handoff_from = sessions[i - 1].session_id.clone();
            sessions[i - 1].handoff_to = sessions[i].session_id.clone();
        }
    }
    for cur in sessions.iter_mut() {
        if cur.launcher.is_empty() {
            cur.launcher = "user".into();
        }
    }

    let msgs: Vec<(u64, String, Vec<String>)> = jsonl(msg_jsonl)
        .iter()
        .map(|m| {
            let data = m.get("data");
            let ts = parse_event_ts(m.get("ts").and_then(Value::as_str).unwrap_or(""));
            let mut from = s(data.and_then(|d| d.get("from")));
            if from.is_empty() {
                from = s(m.get("instance"));
            }
            let to = data
                .and_then(|d| d.get("delivered_to"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            (ts, from, to)
        })
        .collect();
    for cur in sessions.iter_mut() {
        let end = cur.end_ts.unwrap_or(now);
        let mut partners: Vec<Partner> = Vec::new();
        let mut bump = |name: &str, sent: bool| {
            let idx = match partners.iter().position(|p| p.name == name) {
                Some(i) => i,
                None => {
                    partners.push(Partner {
                        name: name.to_string(),
                        ..Default::default()
                    });
                    partners.len() - 1
                }
            };
            if sent {
                partners[idx].sent += 1;
            } else {
                partners[idx].received += 1;
            }
        };
        let belongs = |name: &str| {
            instance_belongs_to_agent(name, agent)
                || live_sessions.iter().any(|l| l.instance_name == name)
        };
        for (ts, from, to) in &msgs {
            if *ts < cur.start_ts || *ts > end {
                continue;
            }
            if belongs(from) {
                for r in to.iter().filter(|r| !belongs(r)) {
                    bump(r, true);
                }
            } else if to.iter().any(|r| belongs(r)) {
                bump(from, false);
            }
        }
        partners.sort_by(|a, b| {
            (b.sent + b.received)
                .cmp(&(a.sent + a.received))
                .then_with(|| a.name.cmp(&b.name))
        });
        cur.partners = partners;
    }
    sessions.reverse();
    sessions
}

impl SessionInfo {
    /// The session overlaps `[start, end]` (None = unbounded); a running
    /// session lasts until `now`.
    pub fn overlaps(&self, start: Option<u64>, end: Option<u64>, now: u64) -> bool {
        let s_end = self.end_ts.unwrap_or(now);
        start.is_none_or(|w| s_end >= w) && end.is_none_or(|w| self.start_ts <= w)
    }

    fn launchable(&self, tools: &[&str]) -> Result<(), String> {
        if self.session_id.is_empty() {
            return Err("no session id recorded".into());
        }
        if self.transcript_size.is_none() {
            return Err("transcript not found on disk".into());
        }
        if !tools.contains(&self.tool.as_str()) {
            return Err(format!("{} sessions cannot be targeted by id", self.tool));
        }
        Ok(())
    }

    /// Why Resume is disabled, or Ok.
    pub fn can_resume(&self, agent_running: bool) -> Result<(), String> {
        if agent_running {
            return Err("the agent is running".into());
        }
        self.launchable(RESUME_TOOLS)
    }

    /// Why Fork is disabled, or Ok.
    pub fn can_fork(&self) -> Result<(), String> {
        self.launchable(FORK_TOOLS)
    }

    /// `codex · gpt-5 · medium`: the tool, then the model (`--model`, `-m`)
    /// and reasoning effort (`--effort`, `-c model_reasoning_effort=...`)
    /// from the launch arguments, each only when present.
    pub fn launch_summary(&self) -> String {
        let (mut model, mut effort) = (None, None);
        let mut args = self.launch_args.iter();
        while let Some(a) = args.next() {
            let (flag, inline) = match a.split_once('=') {
                Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
                _ => (a.as_str(), None),
            };
            let mut value = || inline.clone().or_else(|| args.next().cloned());
            match flag {
                "--model" | "-m" => model = value(),
                "--effort" => effort = value(),
                "-c" | "--config" => {
                    if let Some(v) = value()
                        && let Some(e) = v.strip_prefix("model_reasoning_effort=")
                    {
                        effort = Some(e.trim_matches(['"', '\'']).to_string());
                    }
                }
                _ => {}
            }
        }
        [Some(self.tool.clone()), model, effort]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Title in a Claude transcript: the last `/rename` (`custom-title`), else
/// the last `ai-title`.
pub fn claude_title(transcript: &str) -> Option<String> {
    let (mut custom, mut ai): (Option<String>, Option<String>) = (None, None);
    for line in transcript.lines().filter(|l| l.contains("-title\"")) {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (slot, key) = match v.get("type").and_then(Value::as_str) {
            Some("custom-title") => (&mut custom, "customTitle"),
            Some("ai-title") => (&mut ai, "aiTitle"),
            _ => continue,
        };
        if let Some(t) = v.get(key).and_then(Value::as_str).filter(|t| !t.is_empty()) {
            *slot = Some(t.to_string());
        }
    }
    custom.or(ai)
}

/// Codex thread names by session id from `~/.codex/session_index.jsonl`
/// (a later row wins).
pub fn codex_titles(index: &str) -> std::collections::HashMap<String, String> {
    index
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| (s(v.get("id")), s(v.get("thread_name"))))
        .filter(|(id, name)| !id.is_empty() && !name.is_empty())
        .collect()
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_COLUMNS: &[&str] = &[
    "agent",
    "instance",
    "roaming",
    "cli",
    "context",
    "quota",
    "age",
    "since",
    "status",
    "idle",
    "status:status",
    "status:done",
    "detail",
    "unread",
    "dir",
    "branch",
    "git",
];

pub fn format_age(seconds: u64) -> String {
    if seconds >= 86400 {
        format!("{}d", seconds / 86400)
    } else if seconds >= 3600 {
        format!("{}h", seconds / 3600)
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogStatusValue {
    pub name: String,
    #[serde(default)]
    pub updated_at: u64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogStatusColumns {
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub instances: HashMap<String, Vec<CatalogStatusValue>>,
    #[serde(default)]
    pub latest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogAgentItem {
    pub name: String,
    #[serde(default)]
    pub cli: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dir: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub roaming: bool,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub terminal: Option<String>,
    #[serde(default)]
    pub status_columns: Option<CatalogStatusColumns>,
    #[serde(default)]
    pub instances: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LiveLaunchContext {
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub herdr_tab: Option<String>,
    #[serde(default)]
    pub herdr_workspace: Option<String>,
    #[serde(default)]
    pub pane_id: Option<String>,
    #[serde(default)]
    pub terminal_preset_effective: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveInstanceItem {
    pub name: String,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_age_seconds: Option<u64>,
    #[serde(default)]
    pub status_context: Option<String>,
    #[serde(default)]
    pub status_detail: Option<String>,
    #[serde(default)]
    pub unread_count: Option<u64>,
    #[serde(default)]
    pub created_at: Option<f64>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub launch_context: Option<LiveLaunchContext>,
}

pub fn build_table_rows(
    live_json: &str,
    agent_json: &str,
    now_epoch: u64,
) -> Result<Vec<TableRow>, String> {
    let live_instances: Vec<LiveInstanceItem> = if live_json.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(live_json).map_err(|e| format!("parse live instances: {}", e))?
    };

    let catalog_agents: Vec<CatalogAgentItem> = if agent_json.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(agent_json).map_err(|e| format!("parse catalog agents: {}", e))?
    };

    let mut catalog_map = HashMap::new();
    for cat in &catalog_agents {
        catalog_map.insert(cat.name.clone(), cat);
    }

    let mut rows = Vec::new();
    let mut seen_catalog_agents = std::collections::HashSet::new();

    // 1. Convert live instances
    for live in &live_instances {
        let catalog_name = live
            .launch_context
            .as_ref()
            .and_then(|lc| lc.agent_name.clone())
            .unwrap_or_default();

        if !catalog_name.is_empty() {
            seen_catalog_agents.insert(catalog_name.clone());
        }

        let is_running = live.status.as_deref() != Some("stopped");
        let status_str = live.status.clone().unwrap_or_default();
        let since_text = live.status_age_seconds.map(format_age).unwrap_or_default();

        let age_seconds = live
            .created_at
            .map(|c| (now_epoch as f64 - c).max(0.0) as u64);
        let age_text = age_seconds.map(format_age).unwrap_or_default();

        let mut is_roaming = false;
        let mut is_disabled = false;
        let mut config_path = String::new();
        let mut status_values = HashMap::new();
        let mut groups = Vec::new();
        let mut model = String::new();
        let mut reasoning = String::new();

        if let Some(cat) = catalog_map.get(&catalog_name) {
            is_roaming = cat.roaming;
            is_disabled = cat.disabled;
            groups = cat.groups.clone();
            model = cat.model.clone().unwrap_or_default();
            reasoning = cat.reasoning.clone().unwrap_or_default();
            config_path = cat
                .instructions
                .clone()
                .or_else(|| cat.dir.clone())
                .unwrap_or_default();

            if let Some(sc) = &cat.status_columns
                && let Some(vals) = sc.instances.get(&live.name)
            {
                for v in vals {
                    status_values.insert(v.name.clone(), v.value.clone());
                }
            }
        }

        let branch = live
            .launch_context
            .as_ref()
            .and_then(|lc| lc.git_branch.clone())
            .unwrap_or_default();

        let where_term = if let Some(lc) = &live.launch_context {
            if let (Some(ws), Some(tab)) = (&lc.herdr_workspace, &lc.herdr_tab) {
                format!("herdr: {}/{}", ws, tab)
            } else if let Some(preset) = &lc.terminal_preset_effective {
                preset.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let detail = live
            .status_detail
            .clone()
            .or_else(|| live.status_context.clone())
            .unwrap_or_default();

        rows.push(TableRow {
            agent: catalog_name,
            instance: live.name.clone(),
            is_roaming,
            is_disabled,
            is_running,
            cli: live.tool.clone().unwrap_or_default(),
            status: status_str,
            since_seconds: live.status_age_seconds,
            since_text,
            age_seconds,
            age_text,
            idle: String::new(),
            context_fill: String::new(),
            quota_fill: String::new(),
            detail,
            unread_count: live.unread_count.unwrap_or(0) as u32,
            directory: live.directory.clone().unwrap_or_default(),
            branch,
            git_status: String::new(),
            where_term,
            config_path,
            starts_count: None,
            runtime_text: String::new(),
            handoffs_count: None,
            msgs_count: None,
            last_start_text: String::new(),
            stops_summary: String::new(),
            failed_count: None,
            status_values,
            groups,
            launch_context: live.launch_context.clone(),
            model,
            reasoning,
        });
    }

    // 2. Catalog agents without running instances
    for cat in &catalog_agents {
        if !seen_catalog_agents.contains(&cat.name) {
            let mut status_values = HashMap::new();
            if let Some(sc) = &cat.status_columns
                && let Some(latest) = &sc.latest
                && let Some(vals) = sc.instances.get(latest)
            {
                for v in vals {
                    status_values.insert(v.name.clone(), v.value.clone());
                }
            }

            rows.push(TableRow {
                agent: cat.name.clone(),
                instance: String::new(),
                is_roaming: cat.roaming,
                is_disabled: cat.disabled,
                is_running: false,
                cli: cat.cli.clone().unwrap_or_default(),
                status: String::new(),
                since_seconds: None,
                since_text: String::new(),
                age_seconds: None,
                age_text: String::new(),
                idle: String::new(),
                context_fill: String::new(),
                quota_fill: String::new(),
                detail: cat.description.clone().unwrap_or_default(),
                unread_count: 0,
                directory: cat.dir.clone().unwrap_or_default(),
                branch: String::new(),
                git_status: String::new(),
                where_term: cat.terminal.clone().unwrap_or_default(),
                config_path: cat
                    .instructions
                    .clone()
                    .or_else(|| cat.dir.clone())
                    .unwrap_or_default(),
                starts_count: None,
                runtime_text: String::new(),
                handoffs_count: None,
                msgs_count: None,
                last_start_text: String::new(),
                stops_summary: String::new(),
                failed_count: None,
                status_values,
                groups: cat.groups.clone(),
                launch_context: None,
                model: cat.model.clone().unwrap_or_default(),
                reasoning: cat.reasoning.clone().unwrap_or_default(),
            });
        }
    }

    Ok(rows)
}

pub const REPORT_FIELDS: &[&str] = &[
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
    "failed",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TableRow {
    pub agent: String,
    pub instance: String,
    pub is_roaming: bool,
    pub is_disabled: bool,
    pub is_running: bool,
    pub cli: String,
    pub status: String,
    pub since_seconds: Option<u64>,
    pub since_text: String,
    pub age_seconds: Option<u64>,
    pub age_text: String,
    pub idle: String,
    pub context_fill: String,
    pub quota_fill: String,
    pub detail: String,
    pub unread_count: u32,
    pub directory: String,
    pub branch: String,
    pub git_status: String,
    pub where_term: String,
    pub config_path: String,
    pub starts_count: Option<u64>,
    pub runtime_text: String,
    pub handoffs_count: Option<u64>,
    pub msgs_count: Option<u64>,
    pub last_start_text: String,
    pub stops_summary: String,
    pub failed_count: Option<u64>,
    pub status_values: HashMap<String, String>,
    pub groups: Vec<String>,
    pub launch_context: Option<LiveLaunchContext>,
    pub model: String,
    pub reasoning: String,
}

impl TableRow {
    pub fn from_agent_node(node: &crate::model::AgentNode) -> Self {
        let is_running = node.status != crate::model::AgentStatus::Stopped;
        let status_str = match node.status {
            crate::model::AgentStatus::Active => "active",
            crate::model::AgentStatus::Listening => "listening",
            crate::model::AgentStatus::Blocked => "blocked",
            crate::model::AgentStatus::Stopped => "stopped",
        };
        let since_text = if node.status_age_seconds > 0 {
            format!("{}s", node.status_age_seconds)
        } else {
            String::new()
        };
        Self {
            agent: String::new(),
            instance: node.name.clone(),
            is_roaming: false,
            is_disabled: false,
            is_running,
            cli: node.tool.clone().unwrap_or_default(),
            status: status_str.to_string(),
            since_seconds: Some(node.status_age_seconds),
            since_text,
            age_seconds: None,
            age_text: String::new(),
            idle: String::new(),
            context_fill: String::new(),
            quota_fill: String::new(),
            detail: node.status_context.clone().unwrap_or_default(),
            unread_count: node.unread_count as u32,
            directory: node.directory.clone().unwrap_or_default(),
            branch: String::new(),
            git_status: String::new(),
            where_term: String::new(),
            config_path: String::new(),
            starts_count: None,
            runtime_text: String::new(),
            handoffs_count: None,
            msgs_count: None,
            last_start_text: String::new(),
            stops_summary: String::new(),
            failed_count: None,
            status_values: HashMap::new(),
            groups: Vec::new(),
            launch_context: None,
            model: String::new(),
            reasoning: String::new(),
        }
    }

    pub fn get_cell_value(&self, col_key: &str) -> String {
        if let Some(status_field) = col_key.strip_prefix("status:") {
            return self
                .status_values
                .get(status_field)
                .cloned()
                .unwrap_or_default();
        }

        match col_key {
            "agent" => self.agent.clone(),
            "instance" => self.instance.clone(),
            "roaming" => {
                if self.is_roaming {
                    "↻".into()
                } else {
                    String::new()
                }
            }
            "cli" => self.cli.clone(),
            "status" => self.status.clone(),
            "since" => self.since_text.clone(),
            "idle" => self.idle.clone(),
            "age" => self.age_text.clone(),
            "context" => self.context_fill.clone(),
            "quota" => self.quota_fill.clone(),
            "detail" => self.detail.clone(),
            "unread" => {
                if self.unread_count > 0 {
                    self.unread_count.to_string()
                } else {
                    String::new()
                }
            }
            "dir" => self.directory.clone(),
            "branch" => self.branch.clone(),
            "git" => self.git_status.clone(),
            "where" => self.where_term.clone(),
            "config" => self.config_path.clone(),
            "starts" => self.starts_count.map(|n| n.to_string()).unwrap_or_default(),
            "runtime" => self.runtime_text.clone(),
            "handoffs" => self
                .handoffs_count
                .map(|n| n.to_string())
                .unwrap_or_default(),
            "msgs" => self.msgs_count.map(|n| n.to_string()).unwrap_or_default(),
            "last_start" => self.last_start_text.clone(),
            "stops" => self.stops_summary.clone(),
            "failed" => self.failed_count.map(|n| n.to_string()).unwrap_or_default(),
            _ => String::new(),
        }
    }
}

pub fn sort_rows(rows: &mut [TableRow], col_key: &str, reverse: bool, empty_last: bool) {
    rows.sort_by(|a, b| {
        let val_a = a.get_cell_value(col_key);
        let val_b = b.get_cell_value(col_key);

        let a_empty = val_a.trim().is_empty();
        let b_empty = val_b.trim().is_empty();

        if empty_last && (a_empty || b_empty) {
            if a_empty && b_empty {
                return std::cmp::Ordering::Equal;
            }
            if a_empty {
                return std::cmp::Ordering::Greater;
            }
            if b_empty {
                return std::cmp::Ordering::Less;
            }
        }

        // Try numeric comparison for numeric fields
        let cmp = match col_key {
            "since" => a.since_seconds.cmp(&b.since_seconds),
            "age" => a.age_seconds.cmp(&b.age_seconds),
            "unread" => a.unread_count.cmp(&b.unread_count),
            "starts" => a.starts_count.cmp(&b.starts_count),
            "handoffs" => a.handoffs_count.cmp(&b.handoffs_count),
            "msgs" => a.msgs_count.cmp(&b.msgs_count),
            "failed" => a.failed_count.cmp(&b.failed_count),
            _ => {
                // If both strings can be parsed as numbers, compare numerically
                if let (Ok(num_a), Ok(num_b)) =
                    (val_a.trim().parse::<f64>(), val_b.trim().parse::<f64>())
                {
                    num_a
                        .partial_cmp(&num_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    val_a
                        .to_lowercase()
                        .cmp(&val_b.to_lowercase())
                        .then_with(|| val_a.cmp(&val_b))
                }
            }
        };

        if reverse { cmp.reverse() } else { cmp }
    });
}

pub fn filter_row(
    row: &TableRow,
    search_query: &str,
    running_only: bool,
    blocked_only: bool,
    hide_disabled: bool,
    hide_adhoc: bool,
) -> bool {
    if running_only && !row.is_running {
        return false;
    }
    if blocked_only && row.status != "blocked" {
        return false;
    }
    if hide_disabled && row.is_disabled && !row.is_running {
        return false;
    }
    if hide_adhoc && row.agent.is_empty() {
        return false;
    }

    if !search_query.is_empty() {
        let q = search_query.to_lowercase();
        let matches = row.agent.to_lowercase().contains(&q)
            || row.instance.to_lowercase().contains(&q)
            || row.directory.to_lowercase().contains(&q)
            || row.cli.to_lowercase().contains(&q)
            || row.groups.iter().any(|g| g.to_lowercase().contains(&q));
        if !matches {
            return false;
        }
    }

    true
}

pub fn parse_git_porcelain_v2(output: &str) -> (String, String) {
    let mut branch = String::new();
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut dirty = false;

    for line in output.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            branch = b.trim().to_string();
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let parts: Vec<&str> = ab.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(a_str) = parts[0].strip_prefix('+') {
                    ahead = a_str.parse().unwrap_or(0);
                }
                if let Some(b_str) = parts[1].strip_prefix('-') {
                    behind = b_str.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            dirty = true;
        }
    }

    let mut state_parts = Vec::new();
    if dirty {
        state_parts.push("*");
    }
    let ahead_str = if ahead > 0 {
        format!("↑{}", ahead)
    } else {
        String::new()
    };
    if !ahead_str.is_empty() {
        state_parts.push(&ahead_str);
    }
    let behind_str = if behind > 0 {
        format!("↓{}", behind)
    } else {
        String::new()
    };
    if !behind_str.is_empty() {
        state_parts.push(&behind_str);
    }

    (branch, state_parts.join(" "))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventStatsEntry {
    pub starts: u64,
    pub runtime_seconds: u64,
    pub handoffs: u64,
    pub sent: u64,
    pub received: u64,
    pub msgs: u64,
    pub last_start: String,
    pub stops: u64,
    pub failed: u64,
    pub last_start_ts: Option<u64>,
}

pub fn parse_event_ts(ts_str: &str) -> u64 {
    let clean = ts_str.trim().trim_end_matches('Z');
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S") {
        return dt.and_utc().timestamp().max(0) as u64;
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(clean, "%Y-%m-%d %H:%M:%S") {
        return dt.and_utc().timestamp().max(0) as u64;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str.trim()) {
        return dt.timestamp().max(0) as u64;
    }
    0
}

pub fn compute_event_stats(
    life_events_jsonl: &str,
    msg_events_jsonl: &str,
    running_instances: &std::collections::HashSet<String>,
    now: u64,
) -> HashMap<String, EventStatsEntry> {
    let mut life_events: Vec<serde_json::Value> = Vec::new();
    for line in life_events_jsonl.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
            life_events.push(v);
        }
    }
    life_events.sort_by_key(|e| e.get("id").and_then(|i| i.as_u64()).unwrap_or(0));

    let mut opened: HashMap<String, u64> = HashMap::new();
    let mut stats: HashMap<String, EventStatsEntry> = HashMap::new();

    for e in life_events {
        let name = match e.get("instance").and_then(|i| i.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let ts_str = e.get("ts").and_then(|t| t.as_str()).unwrap_or("");
        let ts_sec = parse_event_ts(ts_str);
        let data = e.get("data");
        let action = data
            .and_then(|d| d.get("action"))
            .and_then(|a| a.as_str())
            .unwrap_or("");

        let entry = stats.entry(name.clone()).or_default();
        match action {
            "created" => {
                entry.starts += 1;
                opened.insert(name.clone(), ts_sec);
                if !ts_str.is_empty() {
                    entry.last_start = ts_str.to_string();
                }
            }
            "stopped" => {
                entry.stops += 1;
                if let Some(start_ts) = opened.remove(&name) {
                    entry.runtime_seconds += ts_sec.saturating_sub(start_ts);
                }
            }
            "handoff" => {
                entry.handoffs += 1;
            }
            "launch_failed" | "launch_blocked" => {
                entry.failed += 1;
            }
            _ => {}
        }
    }

    for (name, start_ts) in opened {
        if running_instances.contains(&name) {
            stats.entry(name).or_default().runtime_seconds += now.saturating_sub(start_ts);
        }
    }

    for line in msg_events_jsonl.lines() {
        let val: serde_json::Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let inst = val.get("instance").and_then(|i| i.as_str()).unwrap_or("");
        let data = val.get("data");
        let from = data
            .and_then(|d| d.get("from"))
            .and_then(|f| f.as_str())
            .unwrap_or(inst);
        if !from.is_empty() {
            stats.entry(from.to_string()).or_default().sent += 1;
        }
        if let Some(recipients) = data
            .and_then(|d| d.get("delivered_to"))
            .and_then(|r| r.as_array())
        {
            for rec in recipients {
                if let Some(rec_name) = rec.as_str() {
                    stats.entry(rec_name.to_string()).or_default().received += 1;
                }
            }
        }
    }

    for st in stats.values_mut() {
        st.msgs = st.sent + st.received;
    }

    stats
}

pub fn apply_event_stats(rows: &mut [TableRow], stats: &HashMap<String, EventStatsEntry>) {
    for r in rows.iter_mut() {
        let key = if !r.instance.is_empty() {
            &r.instance
        } else {
            &r.agent
        };
        if let Some(st) = stats.get(key) {
            r.starts_count = Some(st.starts);
            r.runtime_text = if st.runtime_seconds > 0 {
                format_age(st.runtime_seconds)
            } else {
                String::new()
            };
            r.handoffs_count = Some(st.handoffs);
            r.msgs_count = Some(st.sent + st.received);
            r.last_start_text = st.last_start.clone();
            r.stops_summary = format!("{}/{}", st.stops, st.failed);
            r.failed_count = Some(st.failed);
        }
    }
}

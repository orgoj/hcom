use chrono::{DateTime, Utc};
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableDiGraph};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

pub const MAX_EDGE_HISTORY: usize = 50;

/// Scans UTF-8 bytes and strips ECMA-48 CSI and OSC escape sequences
/// (both 7-bit and UTF-8 2-byte C1 encodings). Discards unterminated
/// sequences fail-closed.
pub fn strip_ansi_osc(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        // Check for 7-bit CSI: ESC [ (\x1b[)
        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            let mut found_final = false;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7E).contains(&b) {
                    found_final = true;
                    break;
                } else if (0x30..=0x3F).contains(&b) || (0x20..=0x2F).contains(&b) {
                    continue;
                } else {
                    break;
                }
            }
            if !found_final {
                // Fail-closed on unterminated sequence: discard to end
                break;
            }
            continue;
        }

        // Check for UTF-8 C1 CSI: \u{009B} = [0xC2, 0x9B]
        if bytes[i] == 0xC2 && i + 1 < bytes.len() && bytes[i + 1] == 0x9B {
            i += 2;
            let mut found_final = false;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7E).contains(&b) {
                    found_final = true;
                    break;
                } else if (0x30..=0x3F).contains(&b) || (0x20..=0x2F).contains(&b) {
                    continue;
                } else {
                    break;
                }
            }
            if !found_final {
                break;
            }
            continue;
        }

        // Check for 7-bit OSC: ESC ] (\x1b])
        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b']' {
            i += 2;
            let mut found_term = false;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    i += 1;
                    found_term = true;
                    break;
                } else if (bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\')
                    || (bytes[i] == 0xC2 && i + 1 < bytes.len() && bytes[i + 1] == 0x9C)
                {
                    i += 2;
                    found_term = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if !found_term {
                // Fail-closed on unterminated OSC: discard to end
                break;
            }
            continue;
        }

        // Check for UTF-8 C1 OSC: \u{009D} = [0xC2, 0x9D]
        if bytes[i] == 0xC2 && i + 1 < bytes.len() && bytes[i + 1] == 0x9D {
            i += 2;
            let mut found_term = false;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    i += 1;
                    found_term = true;
                    break;
                } else if (bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\')
                    || (bytes[i] == 0xC2 && i + 1 < bytes.len() && bytes[i + 1] == 0x9C)
                {
                    i += 2;
                    found_term = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if !found_term {
                break;
            }
            continue;
        }

        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Sanitizes display strings: removes ANSI/OSC escapes, ALL C0/C1 control chars
/// (including \n, \r, \t), bidi overrides, and zero-width characters.
pub fn sanitize_display_string(input: &str) -> String {
    let stripped = strip_ansi_osc(input);
    stripped
        .chars()
        .filter(|&c| {
            if c.is_control() {
                return false;
            }
            // Zero-width characters: U+200B (ZWSP), U+200C (ZWNJ), U+200D (ZWJ), U+FEFF (BOM)
            if matches!(c, '\u{200B}'..='\u{200D}' | '\u{FEFF}') {
                return false;
            }
            // Bidi override characters: U+202A..=U+202E, U+2066..=U+2069
            if matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}') {
                return false;
            }
            true
        })
        .collect()
}

/// Collision rule: identity string must be non-empty, must have no leading/trailing
/// whitespace, and must not be altered by sanitization.
pub fn is_valid_identity(name: &str) -> bool {
    if name.is_empty() || name != name.trim() {
        return false;
    }
    sanitize_display_string(name) == name
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Listening,
    Blocked,
    Stopped,
}

impl AgentStatus {
    pub fn from_raw(s: Option<&str>) -> Self {
        match s.map(|v| v.to_lowercase()).as_deref() {
            Some("active") => Self::Active,
            Some("listening") => Self::Listening,
            Some("blocked") => Self::Blocked,
            _ => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNode {
    pub name: String,
    pub status: AgentStatus,
    pub status_context: Option<String>,
    pub status_age_seconds: u64,
    pub unread_count: u64,
    pub parent_name: Option<String>,
    pub directory: Option<String>,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnEdge {
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: u64,
    pub ts: DateTime<Utc>,
    pub from: String,
    pub to: String,
    pub intent: Option<String>,
    pub preview: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEdge {
    pub from: String,
    pub to: String,
    pub count: usize,
    pub last_timestamp: DateTime<Utc>,
    pub last_intent: Option<String>,
    pub last_preview: String,
    #[serde(default)]
    pub history: VecDeque<StoredMessage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Spawn(SpawnEdge),
    Message(MessageEdge),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceSnapshot {
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
    pub status_context: Option<String>,
    pub status_age_seconds: Option<u64>,
    pub unread_count: Option<u64>,
    pub parent_name: Option<String>,
    pub directory: Option<String>,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageData {
    pub from: String,
    #[serde(default)]
    pub delivered_to: Vec<String>,
    #[serde(default)]
    pub mentions: Vec<String>,
    pub intent: Option<String>,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEvent {
    pub id: u64,
    pub ts: DateTime<Utc>,
    pub instance: String,
    pub data: MessageData,
}

/// Time window over retained message history. Bounds apply first, then
/// `last_n` keeps the newest N messages across all edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MessageWindow {
    /// Span in seconds before `end`; `None` = no lower bound (All).
    pub span_secs: Option<i64>,
    /// Window end; `None` = live, follows now.
    pub end: Option<DateTime<Utc>>,
    /// Keep only the newest N messages; `None` = no limit.
    pub last_n: Option<usize>,
}

impl MessageWindow {
    /// All messages, live, no limit: counts are the totals since load.
    pub fn is_unfiltered(&self) -> bool {
        self.span_secs.is_none() && self.end.is_none() && self.last_n.is_none()
    }

    /// Inclusive bounds `(start, end)`; `start` is `None` without a span.
    pub fn bounds(&self, now: DateTime<Utc>) -> (Option<DateTime<Utc>>, DateTime<Utc>) {
        let end = self.end.unwrap_or(now);
        let start = self
            .span_secs
            .filter(|s| *s > 0)
            .map(|s| end - chrono::Duration::seconds(s));
        (start, end)
    }

    pub fn contains(&self, ts: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        let (start, end) = self.bounds(now);
        // A live window has no upper bound, so messages stamped slightly
        // ahead (clock skew, relayed remotes) still count.
        (self.end.is_none() || ts <= end) && start.is_none_or(|s| ts >= s)
    }
}

/// Message count of one edge inside a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowedCount {
    pub count: usize,
    /// The edge history is full (MAX_EDGE_HISTORY) and its oldest retained
    /// message is inside the window, so older in-window messages may be lost.
    pub lower_bound: bool,
}

/// Visual enrichment of a node that `hcom list` snapshots do not carry
/// (catalog roaming flag, context window fill). Keyed by instance name in
/// `AgentGraph::node_styles` and replaced on every table data reload.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NodeStyle {
    pub is_roaming: bool,
    /// Context window fill in percent (0–100), `None` when unknown.
    pub context_percent: Option<f32>,
}

/// Percent from a context cell such as `"123k 45%"`: the last whitespace
/// separated token ending with `%`. Returns `None` when absent or invalid.
pub fn parse_context_percent(fill: &str) -> Option<f32> {
    let token = fill.split_whitespace().last()?;
    let pct: f32 = token.strip_suffix('%')?.parse().ok()?;
    pct.is_finite().then(|| pct.clamp(0.0, 100.0))
}

#[derive(Clone, Debug, Default)]
pub struct AgentGraph {
    pub graph: StableDiGraph<AgentNode, EdgeKind>,
    pub node_map: HashMap<String, NodeIndex>,
    pub spawn_edges: HashMap<(String, String), EdgeIndex>,
    pub message_edges: HashMap<(String, String), EdgeIndex>,
    pub last_event_id: u64,
    pub node_styles: HashMap<String, NodeStyle>,
}

impl AgentGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn get_agent(&self, name: &str) -> Option<&AgentNode> {
        self.node_map
            .get(name)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    pub fn node_style(&self, name: &str) -> NodeStyle {
        self.node_styles.get(name).copied().unwrap_or_default()
    }

    pub fn get_node_index(&self, name: &str) -> Option<NodeIndex> {
        self.node_map.get(name).copied()
    }

    pub fn upsert_agent(&mut self, mut node: AgentNode) -> Option<NodeIndex> {
        if !is_valid_identity(&node.name) {
            return None;
        }

        node.status_context = node.status_context.as_deref().map(sanitize_display_string);
        node.directory = node.directory.as_deref().map(sanitize_display_string);
        node.tool = node.tool.as_deref().map(sanitize_display_string);

        if let Some(ref p) = node.parent_name
            && !is_valid_identity(p)
        {
            node.parent_name = None;
        }

        let name = node.name.clone();
        let new_parent = node.parent_name.clone();

        let (node_idx, old_parent) = if let Some(&idx) = self.node_map.get(&name) {
            let old_parent = self
                .graph
                .node_weight(idx)
                .and_then(|n| n.parent_name.clone());
            if let Some(slot) = self.graph.node_weight_mut(idx) {
                *slot = node;
            }
            (idx, old_parent)
        } else {
            let idx = self.graph.add_node(node);
            self.node_map.insert(name.clone(), idx);
            (idx, None)
        };

        if old_parent != new_parent
            && let Some(old_p) = old_parent
        {
            let edge_key = (old_p, name.clone());
            if let Some(edge_idx) = self.spawn_edges.remove(&edge_key) {
                self.graph.remove_edge(edge_idx);
            }
        }

        if let Some(parent) = new_parent
            && !parent.is_empty()
            && parent != name
        {
            let parent_idx = self.ensure_node_placeholder(&parent);
            let edge_key = (parent.clone(), name.clone());
            if !self.spawn_edges.contains_key(&edge_key) {
                let edge_idx = self.graph.add_edge(
                    parent_idx,
                    node_idx,
                    EdgeKind::Spawn(SpawnEdge {
                        parent,
                        child: name,
                    }),
                );
                self.spawn_edges.insert(edge_key, edge_idx);
            }
        }

        Some(node_idx)
    }

    fn ensure_node_placeholder(&mut self, name: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(name) {
            idx
        } else {
            let placeholder = AgentNode {
                name: name.to_string(),
                status: AgentStatus::Stopped,
                status_context: None,
                status_age_seconds: 0,
                unread_count: 0,
                parent_name: None,
                directory: None,
                tool: None,
            };
            let idx = self.graph.add_node(placeholder);
            self.node_map.insert(name.to_string(), idx);
            idx
        }
    }

    pub fn apply_instance_snapshot(&mut self, snapshots: &[InstanceSnapshot]) {
        for s in snapshots {
            if !is_valid_identity(&s.name) {
                continue;
            }
            let node = AgentNode {
                name: s.name.clone(),
                status: AgentStatus::from_raw(s.status.as_deref()),
                status_context: s.status_context.clone(),
                status_age_seconds: s.status_age_seconds.unwrap_or(0),
                unread_count: s.unread_count.unwrap_or(0),
                parent_name: s.parent_name.clone(),
                directory: s.directory.clone(),
                tool: s.tool.clone(),
            };
            self.upsert_agent(node);
        }
    }

    pub fn apply_message_event(&mut self, event: &MessageEvent) -> bool {
        // Strict watermark check: ignore any event that is <= last_event_id.
        if event.id <= self.last_event_id {
            return false;
        }

        let sender = &event.data.from;
        if !is_valid_identity(sender) {
            // Safely skip invalid sender, but advance watermark so we don't re-loop on reconnect
            self.last_event_id = event.id;
            return false;
        }

        let mut recipients = event.data.delivered_to.clone();
        if recipients.is_empty() {
            recipients = event.data.mentions.clone();
        }

        // Retain only valid, distinct non-self recipients
        recipients.sort();
        recipients.dedup();
        recipients.retain(|r| is_valid_identity(r) && r != sender);

        if recipients.is_empty() {
            // Still advance watermark even if no valid recipients.
            self.last_event_id = event.id;
            return false;
        }

        let sender_idx = self.ensure_node_placeholder(sender);
        let preview = sanitize_preview(&event.data.text);
        let intent = event.data.intent.as_deref().map(sanitize_display_string);
        let sanitized_text = sanitize_display_string(&event.data.text);

        for recipient in recipients {
            let recipient_idx = self.ensure_node_placeholder(&recipient);
            let edge_key = (sender.clone(), recipient.clone());
            let stored_msg = StoredMessage {
                id: event.id,
                ts: event.ts,
                from: sender.clone(),
                to: recipient.clone(),
                intent: intent.clone(),
                preview: preview.clone(),
                text: sanitized_text.clone(),
            };

            if let Some(&edge_idx) = self.message_edges.get(&edge_key) {
                if let Some(EdgeKind::Message(msg_edge)) = self.graph.edge_weight_mut(edge_idx) {
                    msg_edge.count += 1;
                    msg_edge.last_timestamp = event.ts;
                    msg_edge.last_intent = intent.clone();
                    msg_edge.last_preview = preview.clone();
                    if msg_edge.history.len() >= MAX_EDGE_HISTORY {
                        msg_edge.history.pop_front();
                    }
                    msg_edge.history.push_back(stored_msg);
                }
            } else {
                let mut history = VecDeque::with_capacity(MAX_EDGE_HISTORY);
                history.push_back(stored_msg);
                let msg_edge = MessageEdge {
                    from: sender.clone(),
                    to: recipient,
                    count: 1,
                    last_timestamp: event.ts,
                    last_intent: intent.clone(),
                    last_preview: preview.clone(),
                    history,
                };
                let edge_idx =
                    self.graph
                        .add_edge(sender_idx, recipient_idx, EdgeKind::Message(msg_edge));
                self.message_edges.insert(edge_key, edge_idx);
            }
        }

        self.last_event_id = event.id;
        true
    }

    /// Returns the combined conversation history between two agents (a -> b and b -> a),
    /// sorted in ascending chronological order, strictly capped at the last 50 messages.
    pub fn get_conversation_history(&self, a: &str, b: &str) -> Vec<StoredMessage> {
        let mut messages = Vec::new();
        if let Some(edge) = self.get_message_edge(a, b) {
            messages.extend(edge.history.iter().cloned());
        }
        if let Some(edge) = self.get_message_edge(b, a) {
            messages.extend(edge.history.iter().cloned());
        }
        messages.sort_by(|m1, m2| m1.ts.cmp(&m2.ts).then_with(|| m1.id.cmp(&m2.id)));
        if messages.len() > 50 {
            messages.drain(0..messages.len() - 50);
        }
        messages
    }

    /// Retained messages inside `window`, newest first (by ts, then id).
    /// Bounds apply first, then `last_n` keeps the newest N across all edges.
    pub fn messages_in_window(
        &self,
        window: &MessageWindow,
        now: DateTime<Utc>,
    ) -> Vec<(EdgeIndex, &StoredMessage)> {
        let mut msgs: Vec<(EdgeIndex, &StoredMessage)> = self
            .message_edges
            .values()
            .filter_map(|&idx| match self.graph.edge_weight(idx) {
                Some(EdgeKind::Message(m)) => Some((idx, m)),
                _ => None,
            })
            .flat_map(|(idx, m)| {
                m.history
                    .iter()
                    .filter(|s| window.contains(s.ts, now))
                    .map(move |s| (idx, s))
            })
            .collect();
        msgs.sort_by(|(_, a), (_, b)| b.ts.cmp(&a.ts).then_with(|| b.id.cmp(&a.id)));
        if let Some(n) = window.last_n {
            msgs.truncate(n);
        }
        msgs
    }

    /// Per message edge count inside `window`. Edges without in-window
    /// messages are absent (count 0). An unfiltered window returns the totals.
    pub fn windowed_edge_counts(
        &self,
        window: &MessageWindow,
        now: DateTime<Utc>,
    ) -> HashMap<EdgeIndex, WindowedCount> {
        let mut counts: HashMap<EdgeIndex, WindowedCount> = HashMap::new();
        if window.is_unfiltered() {
            for &idx in self.message_edges.values() {
                if let Some(EdgeKind::Message(m)) = self.graph.edge_weight(idx) {
                    counts.insert(
                        idx,
                        WindowedCount {
                            count: m.count,
                            lower_bound: false,
                        },
                    );
                }
            }
            return counts;
        }
        let msgs = self.messages_in_window(window, now);
        let limited = window.last_n.is_some_and(|n| msgs.len() >= n);
        for (idx, _) in &msgs {
            counts.entry(*idx).or_default().count += 1;
        }
        if !limited {
            for (idx, wc) in counts.iter_mut() {
                if let Some(EdgeKind::Message(m)) = self.graph.edge_weight(*idx) {
                    wc.lower_bound = m.history.len() >= MAX_EDGE_HISTORY
                        && m.count > m.history.len()
                        && m.history
                            .front()
                            .is_some_and(|f| window.contains(f.ts, now));
                }
            }
        }
        counts
    }

    /// Conversation between `a` and `b` inside `window` (same selection as
    /// `messages_in_window`), ascending, capped at the last 50 messages.
    pub fn windowed_conversation_history(
        &self,
        a: &str,
        b: &str,
        window: &MessageWindow,
        now: DateTime<Utc>,
    ) -> Vec<StoredMessage> {
        let edges: Vec<EdgeIndex> = [(a, b), (b, a)]
            .iter()
            .filter_map(|(f, t)| {
                self.message_edges
                    .get(&(f.to_string(), t.to_string()))
                    .copied()
            })
            .collect();
        let mut messages: Vec<StoredMessage> = self
            .messages_in_window(window, now)
            .into_iter()
            .filter(|(idx, _)| edges.contains(idx))
            .map(|(_, m)| m.clone())
            .collect();
        messages.reverse();
        if messages.len() > 50 {
            messages.drain(0..messages.len() - 50);
        }
        messages
    }

    /// Timestamp of the oldest retained message, for the timeline scrubber.
    pub fn oldest_message_ts(&self) -> Option<DateTime<Utc>> {
        self.iter_message_edges()
            .filter_map(|m| m.history.front().map(|s| s.ts))
            .min()
    }

    pub fn reset_events(&mut self) {
        self.graph
            .retain_edges(|frozen, edge_idx| !matches!(frozen[edge_idx], EdgeKind::Message(_)));
        self.message_edges.clear();
        self.last_event_id = 0;

        for node in self.graph.node_weights_mut() {
            node.unread_count = 0;
        }
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &AgentNode> {
        self.graph.node_weights()
    }

    pub fn iter_message_edges(&self) -> impl Iterator<Item = &MessageEdge> {
        self.graph.edge_weights().filter_map(|e| match e {
            EdgeKind::Message(m) => Some(m),
            _ => None,
        })
    }

    pub fn iter_spawn_edges(&self) -> impl Iterator<Item = &SpawnEdge> {
        self.graph.edge_weights().filter_map(|e| match e {
            EdgeKind::Spawn(s) => Some(s),
            _ => None,
        })
    }

    pub fn get_message_edge(&self, from: &str, to: &str) -> Option<&MessageEdge> {
        self.message_edges
            .get(&(from.to_string(), to.to_string()))
            .and_then(|&idx| self.graph.edge_weight(idx))
            .and_then(|e| match e {
                EdgeKind::Message(m) => Some(m),
                _ => None,
            })
    }

    pub fn get_spawn_edge(&self, parent: &str, child: &str) -> Option<&SpawnEdge> {
        self.spawn_edges
            .get(&(parent.to_string(), child.to_string()))
            .and_then(|&idx| self.graph.edge_weight(idx))
            .and_then(|e| match e {
                EdgeKind::Spawn(s) => Some(s),
                _ => None,
            })
    }
}

fn sanitize_preview(text: &str) -> String {
    let sanitized = sanitize_display_string(text);
    let single_line = sanitized.trim();
    if single_line.chars().count() > 120 {
        let truncated: String = single_line.chars().take(117).collect();
        format!("{truncated}...")
    } else {
        single_line.to_string()
    }
}

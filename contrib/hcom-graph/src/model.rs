use chrono::{DateTime, Utc};
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableDiGraph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct MessageEdge {
    pub from: String,
    pub to: String,
    pub count: usize,
    pub last_timestamp: DateTime<Utc>,
    pub last_intent: Option<String>,
    pub last_preview: String,
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

#[derive(Debug, Default)]
pub struct AgentGraph {
    pub graph: StableDiGraph<AgentNode, EdgeKind>,
    pub node_map: HashMap<String, NodeIndex>,
    pub spawn_edges: HashMap<(String, String), EdgeIndex>,
    pub message_edges: HashMap<(String, String), EdgeIndex>,
    pub last_event_id: u64,
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

    pub fn get_node_index(&self, name: &str) -> Option<NodeIndex> {
        self.node_map.get(name).copied()
    }

    pub fn upsert_agent(&mut self, node: AgentNode) -> NodeIndex {
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

        // If parent changed or was removed, remove old SpawnEdge
        if old_parent != new_parent
            && let Some(old_p) = old_parent
        {
            let edge_key = (old_p, name.clone());
            if let Some(edge_idx) = self.spawn_edges.remove(&edge_key) {
                self.graph.remove_edge(edge_idx);
            }
        }

        // Add new SpawnEdge if parent is specified
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

        node_idx
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
        if sender.is_empty() {
            return false;
        }

        let mut recipients = event.data.delivered_to.clone();
        if recipients.is_empty() {
            recipients = event.data.mentions.clone();
        }

        // Retain only valid, distinct non-self recipients.
        recipients.sort();
        recipients.dedup();
        recipients.retain(|r| !r.is_empty() && r != sender);

        if recipients.is_empty() {
            // Still advance watermark even if no valid recipients.
            self.last_event_id = event.id;
            return true;
        }

        let sender_idx = self.ensure_node_placeholder(sender);

        for recipient in recipients {
            let recipient_idx = self.ensure_node_placeholder(&recipient);
            let edge_key = (sender.clone(), recipient.clone());

            if let Some(&edge_idx) = self.message_edges.get(&edge_key) {
                if let Some(EdgeKind::Message(msg_edge)) = self.graph.edge_weight_mut(edge_idx) {
                    msg_edge.count += 1;
                    msg_edge.last_timestamp = event.ts;
                    msg_edge.last_intent = event.data.intent.clone();
                    msg_edge.last_preview = sanitize_preview(&event.data.text);
                }
            } else {
                let msg_edge = MessageEdge {
                    from: sender.clone(),
                    to: recipient,
                    count: 1,
                    last_timestamp: event.ts,
                    last_intent: event.data.intent.clone(),
                    last_preview: sanitize_preview(&event.data.text),
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

    pub fn reset_events(&mut self) {
        // StableDiGraph retains stable edge indices for surviving edges!
        self.graph
            .retain_edges(|frozen, edge_idx| !matches!(frozen[edge_idx], EdgeKind::Message(_)));
        self.message_edges.clear();
        self.last_event_id = 0;

        // Reset unread counts on nodes
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
    let single_line = text.lines().next().unwrap_or("").trim();
    if single_line.chars().count() > 120 {
        let truncated: String = single_line.chars().take(117).collect();
        format!("{truncated}...")
    } else {
        single_line.to_string()
    }
}

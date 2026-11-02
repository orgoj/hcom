use crate::model::{AgentGraph, EdgeKind};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

/// Golden angle in radians, used for deterministic spiral and offset placement.
const GOLDEN_ANGLE: f32 = 2.399_963;
/// Margin included in `collision_radius` beyond the node rings (body 26 +
/// rings 13 + this margin).
const COLLISION_MARGIN: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct NodePhysics {
    pub pos: egui::Pos2,
    pub vel: egui::Vec2,
    pub pinned: bool,
}

/// How nodes are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    /// Spawn tree: layers by spawn depth, parents centered over children.
    Hierarchy,
    /// Force-directed: heavy communicators are pulled together.
    #[default]
    Cluster,
    /// No simulation: nodes move only by dragging.
    Manual,
}

impl LayoutMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hierarchy => "hierarchy",
            Self::Cluster => "cluster",
            Self::Manual => "manual",
        }
    }

    /// Parses a persisted value; unknown values fall back to Cluster.
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "hierarchy" => Self::Hierarchy,
            "manual" => Self::Manual,
            _ => Self::Cluster,
        }
    }
}

/// Aggregated spring between an unordered node pair.
#[derive(Debug, Clone, Copy)]
struct Spring {
    a: NodeIndex,
    b: NodeIndex,
    /// Messages in both directions (total since load, not the time window).
    msgs: usize,
}

pub struct GraphLayout {
    pub nodes: HashMap<NodeIndex, NodePhysics>,
    pub sorted_indices: Vec<NodeIndex>,
    pub cursor_i: usize,
    pub cursor_j: usize,
    pub edge_cursor: usize,
    pub integrate_cursor: usize,
    pub k_rep: f32,
    pub k_att: f32,
    pub l_0: f32,
    pub k_center: f32,
    pub damping: f32,
    pub max_speed: f32,
    pub epsilon_sleep: f32,
    pub max_step_duration: std::time::Duration,
    pub is_sleeping: bool,
    /// Consecutive steps below `epsilon_sleep`; the layout sleeps after
    /// `sleep_after_calm_steps`, so a system at rest that is just starting to
    /// accelerate does not fall asleep.
    calm_steps: u32,
    pub sleep_after_calm_steps: u32,

    pub mode: LayoutMode,
    /// Collision radius of a node (body 26 + rings 13 + 1).
    pub collision_radius: f32,
    /// Vertical distance of spawn layers in Hierarchy mode.
    pub h_layer: f32,
    pub k_layer: f32,
    pub k_tree_x: f32,
    pub k_msg_base: f32,
    pub msg_beta: f32,
    pub l_min: f32,
    /// Rest length of a one-message spring before the ln(1 + count) shrink.
    pub l_msg_0: f32,

    /// Collision pass cursor (pair i, j) and pairs done in the current pass.
    pub collision_i: usize,
    pub collision_j: usize,
    collision_done: usize,
    collision_pass_clean: bool,
    /// The last complete collision pass found no separable overlap.
    pub last_collision_pass_clean: bool,

    tree_key: Option<u64>,
    pub tree_targets: HashMap<NodeIndex, egui::Pos2>,
    springs_key: Option<(usize, u64, usize, u64)>,
    springs: Vec<Spring>,
    /// Start positions of nodes by name (persisted drags); not used in Hierarchy.
    pub saved_positions: HashMap<String, egui::Pos2>,
}

impl Default for GraphLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphLayout {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            sorted_indices: Vec::new(),
            cursor_i: 0,
            cursor_j: 1,
            edge_cursor: 0,
            integrate_cursor: 0,
            k_rep: 2.0e6,
            k_att: 20.0,
            l_0: 120.0,
            k_center: 0.05,
            damping: 0.7,
            max_speed: 400.0,
            epsilon_sleep: 0.2,
            max_step_duration: std::time::Duration::from_millis(2),
            is_sleeping: false,
            calm_steps: 0,
            sleep_after_calm_steps: 20,
            mode: LayoutMode::default(),
            collision_radius: 40.0,
            h_layer: 130.0,
            k_layer: 200.0,
            k_tree_x: 60.0,
            k_msg_base: 20.0,
            msg_beta: 0.5,
            l_min: 85.0,
            l_msg_0: 180.0,
            collision_i: 0,
            collision_j: 1,
            collision_done: 0,
            collision_pass_clean: true,
            last_collision_pass_clean: false,
            tree_key: None,
            tree_targets: HashMap::new(),
            springs_key: None,
            springs: Vec::new(),
            saved_positions: HashMap::new(),
        }
    }

    /// Switches the mode, resets every pass cursor so no half-finished pass
    /// of the previous mode is mixed in, and wakes the simulation.
    pub fn set_mode(&mut self, mode: LayoutMode) {
        self.mode = mode;
        self.reset_cursors();
        self.tree_key = None;
        self.is_sleeping = false;
        self.calm_steps = 0;
    }

    fn reset_cursors(&mut self) {
        self.cursor_i = 0;
        self.cursor_j = 1;
        self.edge_cursor = 0;
        self.integrate_cursor = 0;
        self.collision_i = 0;
        self.collision_j = 1;
        self.collision_done = 0;
        self.collision_pass_clean = true;
        self.last_collision_pass_clean = false;
    }

    pub fn sync_nodes(&mut self, graph: &AgentGraph) {
        let before = self.nodes.len();
        // Remove deleted nodes
        self.nodes.retain(|idx, _| graph.graph.contains_node(*idx));
        let removed = self.nodes.len() != before;

        let mut new_nodes: Vec<NodeIndex> = graph
            .graph
            .node_indices()
            .filter(|idx| !self.nodes.contains_key(idx))
            .collect();

        let added = !new_nodes.is_empty();
        if added {
            // Place parents before children so a child can sit next to its parent.
            let depths = spawn_depths(graph);
            new_nodes.sort_by_key(|idx| (depths.get(idx).copied().unwrap_or(0), *idx));
            if self.mode == LayoutMode::Hierarchy {
                self.refresh_tree_targets(graph);
            }
            for idx in new_nodes {
                let pos = self.initial_position(graph, idx);
                self.nodes.insert(
                    idx,
                    NodePhysics {
                        pos,
                        vel: egui::Vec2::ZERO,
                        pinned: false,
                    },
                );
            }
            self.is_sleeping = false;
            self.calm_steps = 0;
        }

        self.sorted_indices = self.nodes.keys().copied().collect();
        self.sorted_indices.sort();

        if removed || added {
            // The pair count changed: start a new collision pass.
            self.collision_done = 0;
            self.collision_pass_clean = true;
            self.last_collision_pass_clean = false;
        }
        // Reset cursors to safe bounds
        let count = self.sorted_indices.len();
        if self.cursor_i >= count.saturating_sub(1)
            || self.cursor_j <= self.cursor_i
            || self.cursor_j >= count
        {
            self.cursor_i = 0;
            self.cursor_j = 1;
        }
        if self.integrate_cursor >= count {
            self.integrate_cursor = 0;
        }
        if self.collision_i >= count.saturating_sub(1)
            || self.collision_j <= self.collision_i
            || self.collision_j >= count
        {
            self.collision_i = 0;
            self.collision_j = 1;
        }
    }

    /// Position of a new node: its tree target in Hierarchy mode; otherwise
    /// next to its placed parent or first placed message partner; otherwise
    /// a deterministic spiral position (no fixed circle).
    fn initial_position(&self, graph: &AgentGraph, idx: NodeIndex) -> egui::Pos2 {
        if self.mode == LayoutMode::Hierarchy
            && let Some(t) = self.tree_targets.get(&idx)
        {
            return *t;
        }
        if self.mode != LayoutMode::Hierarchy
            && let Some(p) = self.saved_positions.get(&graph.graph[idx].name)
        {
            return *p;
        }
        let k = idx.index() as f32;
        let parent = graph.graph[idx]
            .parent_name
            .as_deref()
            .and_then(|p| graph.get_node_index(p));
        let anchor = parent
            .and_then(|p| self.nodes.get(&p))
            .or_else(|| {
                graph
                    .graph
                    .neighbors_undirected(idx)
                    .filter_map(|n| self.nodes.get(&n))
                    .next()
            })
            .map(|p| p.pos);
        let angle = k * GOLDEN_ANGLE;
        match anchor {
            Some(a) => {
                let r = 2.0 * self.collision_radius + 30.0;
                a + egui::vec2(r * angle.cos(), r * angle.sin())
            }
            None => {
                let r = 2.0 * self.collision_radius * (k + 1.0).sqrt();
                egui::pos2(r * angle.cos(), r * angle.sin())
            }
        }
    }

    /// Recomputes the tree targets only when the node set or the spawn
    /// structure changed.
    fn refresh_tree_targets(&mut self, graph: &AgentGraph) {
        let key = structure_key(graph);
        if self.tree_key != Some(key) {
            self.tree_targets =
                tree_targets(graph, self.h_layer, 2.0 * self.collision_radius + 20.0);
            self.tree_key = Some(key);
        }
    }

    /// Rebuilds the aggregated springs when edges or message counts changed.
    fn refresh_springs(&mut self, graph: &AgentGraph) {
        let key = (
            graph.graph.edge_count(),
            graph.last_event_id,
            graph.graph.node_count(),
            // A re-parent swaps spawn edges without changing the counts.
            structure_key(graph),
        );
        if self.springs_key == Some(key) {
            return;
        }
        let mut pairs: HashMap<(NodeIndex, NodeIndex), usize> = HashMap::new();
        for e in graph.graph.edge_references() {
            let (s, t) = (e.source(), e.target());
            if s == t {
                continue;
            }
            let pair = if s < t { (s, t) } else { (t, s) };
            let msgs = match e.weight() {
                EdgeKind::Message(m) => m.count,
                EdgeKind::Spawn(_) => 0,
            };
            *pairs.entry(pair).or_default() += msgs;
        }
        let mut springs: Vec<Spring> = pairs
            .into_iter()
            .map(|((a, b), msgs)| Spring { a, b, msgs })
            .collect();
        springs.sort_by_key(|s| (s.a, s.b));
        self.springs = springs;
        self.springs_key = Some(key);
        self.edge_cursor = 0;
    }

    /// Stiffness and rest length of a spring: sublinear in the message count;
    /// a spawn-only pair uses the base spring.
    pub fn spring_params(&self, msgs: usize) -> (f32, f32) {
        if msgs == 0 {
            return (self.k_att, self.l_0);
        }
        let w = (1.0 + msgs as f32).ln();
        let k = self.k_msg_base * w;
        let l = (self.l_msg_0 / (1.0 + self.msg_beta * w)).max(self.l_min);
        (k, l)
    }

    /// Drops cached tree targets and springs, e.g. after the graph object was
    /// replaced and node or edge indices may be reused in another order.
    pub fn invalidate_caches(&mut self) {
        self.tree_key = None;
        self.springs_key = None;
    }

    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
        self.calm_steps = 0;
    }

    pub fn hit_test(&self, world_pos: egui::Pos2, radius: f32) -> Option<NodeIndex> {
        let mut closest = None;
        let mut min_dist_sq = radius * radius;

        for (&idx, phys) in &self.nodes {
            let dist_sq = phys.pos.distance_sq(world_pos);
            if dist_sq <= min_dist_sq {
                min_dist_sq = dist_sq;
                closest = Some(idx);
            }
        }

        closest
    }

    pub fn set_node_pos(&mut self, idx: NodeIndex, pos: egui::Pos2, pinned: bool) {
        if let Some(phys) = self.nodes.get_mut(&idx) {
            phys.pos = pos;
            phys.vel = egui::Vec2::ZERO;
            phys.pinned = pinned;
            self.is_sleeping = false;
            self.calm_steps = 0;
        }
    }

    /// Performs time-sliced force-directed simulation with a strict frame budget clamp.
    /// Resumes exact cursors across frames for:
    /// 1. Coulomb repulsion (cursor_i, cursor_j)
    /// 2. Hooke springs aggregated per node pair (edge_cursor)
    /// 3. Velocity/position integration (integrate_cursor)
    /// 4. Collision separation (collision_i, collision_j)
    ///
    /// Cumulative budget shares: forces 55 %, integration 80 %, collision 100 %.
    /// Manual mode does not simulate. The layout sleeps only when motion is
    /// below the threshold and the last complete collision pass was clean.
    pub fn step(&mut self, graph: &AgentGraph, dt: f32) -> f32 {
        if self.is_sleeping || self.mode == LayoutMode::Manual {
            return 0.0;
        }

        let start_time = std::time::Instant::now();
        let clamped_dt = dt.clamp(0.001, 0.033);
        let sub_dt = clamped_dt * 0.5;

        let mut budget_exceeded = false;

        if self.sorted_indices.len() != self.nodes.len() {
            self.sorted_indices = self.nodes.keys().copied().collect();
            self.sorted_indices.sort();
        }

        let n = self.sorted_indices.len();
        if n < 2 {
            return 0.0;
        }

        // Validate cursors
        if self.cursor_i >= n.saturating_sub(1)
            || self.cursor_j <= self.cursor_i
            || self.cursor_j >= n
        {
            self.cursor_i = 0;
            self.cursor_j = 1;
        }
        if self.integrate_cursor >= n {
            self.integrate_cursor = 0;
        }

        let start_pos: Vec<egui::Pos2> = self
            .sorted_indices
            .iter()
            .map(|idx| self.nodes[idx].pos)
            .collect();

        let force_budget = self.max_step_duration.saturating_mul(55) / 100;
        let integrate_budget = self.max_step_duration.saturating_mul(80) / 100;
        let hierarchy = self.mode == LayoutMode::Hierarchy;
        if hierarchy {
            self.refresh_tree_targets(graph);
        }
        self.refresh_springs(graph);

        let mut forces = HashMap::<NodeIndex, egui::Vec2>::new();

        // 1. Coulomb repulsion with exact pair cursor
        let total_pairs = n * (n - 1) / 2;
        let mut processed_pairs = 0usize;
        let mut cur_i = self.cursor_i;
        let mut cur_j = self.cursor_j;

        while processed_pairs < total_pairs && cur_i < n - 1 {
            if processed_pairs > 0
                && processed_pairs.is_multiple_of(32)
                && start_time.elapsed() >= force_budget
            {
                budget_exceeded = true;
                break;
            }

            let idx_i = self.sorted_indices[cur_i];
            let idx_j = self.sorted_indices[cur_j];
            let pos_i = self.nodes[&idx_i].pos;
            let pos_j = self.nodes[&idx_j].pos;

            let d = pos_j - pos_i;
            let dist_sq = d.length_sq().max(1.0);
            let dist = dist_sq.sqrt();

            let force_mag = self.k_rep / dist_sq;
            let f = (d / dist) * force_mag;

            *forces.entry(idx_i).or_default() -= f;
            *forces.entry(idx_j).or_default() += f;

            processed_pairs += 1;
            cur_j += 1;
            if cur_j >= n {
                cur_i += 1;
                cur_j = cur_i + 1;
            }
        }

        if cur_i >= n - 1 {
            self.cursor_i = 0;
            self.cursor_j = 1;
        } else {
            self.cursor_i = cur_i;
            self.cursor_j = cur_j;
        }

        // 2. Springs aggregated per unordered pair, with spring cursor
        if !budget_exceeded {
            let mut i = self.edge_cursor.min(self.springs.len());
            let mut done = 0usize;
            while i < self.springs.len() {
                if done > 0 && done.is_multiple_of(32) && start_time.elapsed() >= force_budget {
                    budget_exceeded = true;
                    break;
                }
                let s = self.springs[i];
                // Hierarchy: the tree places spawn pairs; messages act on x only, weaker.
                if !(hierarchy && s.msgs == 0)
                    && let (Some(pa), Some(pb)) = (self.nodes.get(&s.a), self.nodes.get(&s.b))
                {
                    let (k, l) = self.spring_params(s.msgs);
                    let d = pb.pos - pa.pos;
                    let dist = d.length().max(1.0);
                    let mut f = (d / dist) * (k * (dist - l));
                    if hierarchy {
                        f = egui::vec2(f.x * 0.5, 0.0);
                    }
                    *forces.entry(s.a).or_default() += f;
                    *forces.entry(s.b).or_default() -= f;
                }
                i += 1;
                done += 1;
            }
            self.edge_cursor = if budget_exceeded { i } else { 0 };
        }

        // 3. Integration with integrate cursor and deadline clamp
        let mut integrated_count = 0usize;
        let mut k = self.integrate_cursor;

        while integrated_count < n {
            if integrated_count > 0
                && integrated_count.is_multiple_of(64)
                && start_time.elapsed() >= integrate_budget
            {
                budget_exceeded = true;
                break;
            }

            let idx = self.sorted_indices[k];
            let target = if hierarchy {
                self.tree_targets.get(&idx).copied()
            } else {
                None
            };
            if let Some(phys) = self.nodes.get_mut(&idx) {
                if phys.pinned {
                    phys.vel = egui::Vec2::ZERO;
                } else {
                    let pair_f = forces.get(&idx).copied().unwrap_or(egui::Vec2::ZERO);
                    let f = match target {
                        // Hierarchy: anchor to the tree slot instead of center gravity.
                        Some(t) => {
                            pair_f
                                + egui::vec2(
                                    -self.k_tree_x * (phys.pos.x - t.x),
                                    -self.k_layer * (phys.pos.y - t.y),
                                )
                        }
                        None => pair_f - self.k_center * phys.pos.to_vec2(),
                    };
                    phys.vel = (phys.vel + f * sub_dt) * self.damping;

                    let speed = phys.vel.length();
                    if speed > self.max_speed {
                        phys.vel = (phys.vel / speed) * self.max_speed;
                    }

                    phys.pos += phys.vel * sub_dt;
                }
            }

            integrated_count += 1;
            k = (k + 1) % n;
        }

        self.integrate_cursor = k;

        // 4. Collision separation (position-based) with its own resume cursor
        if !self.collide(start_time, total_pairs) {
            budget_exceeded = true;
        }

        // Motion is the largest real displacement of a node after collision:
        // a node pressed into a contact keeps its velocity, but the collision
        // cancels it. The maximum keeps the threshold independent of the node
        // count and keeps a travelling node awake while the rest only creeps.
        let total_motion = self
            .sorted_indices
            .iter()
            .zip(&start_pos)
            .map(|(idx, p0)| self.nodes[idx].pos.distance(*p0))
            .fold(0.0f32, f32::max);

        if !budget_exceeded && total_motion < self.epsilon_sleep && self.last_collision_pass_clean {
            self.calm_steps += 1;
            if self.calm_steps >= self.sleep_after_calm_steps {
                self.is_sleeping = true;
            }
        } else {
            self.calm_steps = 0;
        }

        if budget_exceeded || !self.last_collision_pass_clean {
            total_motion.max(self.epsilon_sleep + 0.1)
        } else {
            total_motion
        }
    }

    /// Separates overlapping pairs, resuming at the collision cursor. Returns
    /// false when the frame budget stopped the pass early. A pass is complete
    /// after `total_pairs` consecutive pairs; its result is recorded in
    /// `last_collision_pass_clean`. Pairs that cannot move (both pinned) do
    /// not count as penetration.
    fn collide(&mut self, start_time: std::time::Instant, total_pairs: usize) -> bool {
        let n = self.sorted_indices.len();
        let min_dist = 2.0 * self.collision_radius;
        let mut processed = 0usize;
        while processed < total_pairs {
            if processed > 0
                && processed.is_multiple_of(64)
                && start_time.elapsed() >= self.max_step_duration
            {
                return false;
            }
            let (ci, cj) = (self.collision_i, self.collision_j);
            let (idx_i, idx_j) = (self.sorted_indices[ci], self.sorted_indices[cj]);
            let (pi, pin_i) = {
                let p = &self.nodes[&idx_i];
                (p.pos, p.pinned)
            };
            let (pj, pin_j) = {
                let p = &self.nodes[&idx_j];
                (p.pos, p.pinned)
            };
            let d = pj - pi;
            let dist = d.length();
            if dist < min_dist && !(pin_i && pin_j) {
                let dir = if dist > 1e-3 {
                    d / dist
                } else {
                    // Coincident nodes: deterministic direction from the pair.
                    let a = (ci * 7 + cj * 13) as f32 * GOLDEN_ANGLE;
                    egui::vec2(a.cos(), a.sin())
                };
                let delta = min_dist - dist;
                let (move_i, move_j) = match (pin_i, pin_j) {
                    (true, _) => (0.0, delta),
                    (_, true) => (delta, 0.0),
                    _ => (delta * 0.5, delta * 0.5),
                };
                if let Some(p) = self.nodes.get_mut(&idx_i) {
                    p.pos -= dir * move_i;
                }
                if let Some(p) = self.nodes.get_mut(&idx_j) {
                    p.pos += dir * move_j;
                }
                // Contact within the 1 px margin of both nodes leaves the rings
                // apart; only a deeper overlap keeps the layout awake.
                if delta > 2.0 * COLLISION_MARGIN {
                    self.collision_pass_clean = false;
                }
            }

            processed += 1;
            self.collision_done += 1;
            self.collision_j += 1;
            if self.collision_j >= n {
                self.collision_i += 1;
                self.collision_j = self.collision_i + 1;
                if self.collision_i >= n - 1 {
                    self.collision_i = 0;
                    self.collision_j = 1;
                }
            }
            if self.collision_done >= total_pairs {
                self.last_collision_pass_clean = self.collision_pass_clean;
                self.collision_pass_clean = true;
                self.collision_done = 0;
            }
        }
        true
    }
}

/// Order-independent key of the node set and the spawn structure.
fn structure_key(graph: &AgentGraph) -> u64 {
    let mut sum = graph.graph.node_count() as u64;
    for (parent, child) in graph.spawn_edges.keys() {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (parent, child).hash(&mut h);
        sum = sum.wrapping_add(h.finish());
    }
    for idx in graph.graph.node_indices() {
        sum = sum.wrapping_add((idx.index() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    }
    sum
}

/// Spawn tree children per node, sorted by name for a stable layout.
fn spawn_children(graph: &AgentGraph) -> HashMap<NodeIndex, Vec<NodeIndex>> {
    let mut children: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for (parent, child) in graph.spawn_edges.keys() {
        if let (Some(p), Some(c)) = (graph.get_node_index(parent), graph.get_node_index(child))
            && p != c
        {
            children.entry(p).or_default().push(c);
        }
    }
    for list in children.values_mut() {
        list.sort_by(|a, b| graph.graph[*a].name.cmp(&graph.graph[*b].name));
    }
    children
}

/// Roots in name order: nodes without an in-graph spawn parent (placeholder
/// parents are roots), then, for nodes only reachable through a cycle, the
/// lowest index of each unreached component.
fn spawn_forest(graph: &AgentGraph) -> (Vec<NodeIndex>, HashMap<NodeIndex, Vec<NodeIndex>>) {
    let children = spawn_children(graph);
    let has_parent: HashSet<NodeIndex> = children.values().flatten().copied().collect();
    let mut roots: Vec<NodeIndex> = graph
        .graph
        .node_indices()
        .filter(|i| !has_parent.contains(i))
        .collect();
    roots.sort_by(|a, b| graph.graph[*a].name.cmp(&graph.graph[*b].name));
    (roots, children)
}

/// Spawn depth per node: 0 for roots, parent + 1 for children (BFS). Nodes
/// on a spawn cycle that no root reaches start a new tree at depth 0.
pub fn spawn_depths(graph: &AgentGraph) -> HashMap<NodeIndex, usize> {
    let (roots, children) = spawn_forest(graph);
    let mut depths = HashMap::new();
    let mut all: Vec<NodeIndex> = graph.graph.node_indices().collect();
    all.sort();
    let starts = roots.into_iter().chain(all);
    for start in starts {
        if depths.contains_key(&start) {
            continue;
        }
        depths.insert(start, 0);
        let mut queue = VecDeque::from([start]);
        while let Some(cur) = queue.pop_front() {
            let d = depths[&cur];
            for &c in children.get(&cur).into_iter().flatten() {
                if let std::collections::hash_map::Entry::Vacant(e) = depths.entry(c) {
                    e.insert(d + 1);
                    queue.push_back(c);
                }
            }
        }
    }
    depths
}

/// Tidy tree layout of the spawn forest: leaves on consecutive x slots of
/// width `slot`, each parent centered over its children, y = depth * `h_layer`.
/// The result is centered around the origin.
pub fn tree_targets(graph: &AgentGraph, h_layer: f32, slot: f32) -> HashMap<NodeIndex, egui::Pos2> {
    let (roots, children) = spawn_forest(graph);
    let mut all: Vec<NodeIndex> = graph.graph.node_indices().collect();
    all.sort();
    let mut x_of: HashMap<NodeIndex, f32> = HashMap::new();
    let mut depth_of: HashMap<NodeIndex, usize> = HashMap::new();
    let mut next_slot = 0usize;

    fn place(
        node: NodeIndex,
        depth: usize,
        children: &HashMap<NodeIndex, Vec<NodeIndex>>,
        x_of: &mut HashMap<NodeIndex, f32>,
        depth_of: &mut HashMap<NodeIndex, usize>,
        next_slot: &mut usize,
        slot: f32,
    ) {
        depth_of.insert(node, depth);
        let mut child_xs = Vec::new();
        for &c in children.get(&node).into_iter().flatten() {
            if depth_of.contains_key(&c) {
                continue;
            }
            place(c, depth + 1, children, x_of, depth_of, next_slot, slot);
            child_xs.push(x_of[&c]);
        }
        let x = match (child_xs.first(), child_xs.last()) {
            (Some(a), Some(b)) => (a + b) / 2.0,
            _ => {
                let x = *next_slot as f32 * slot;
                *next_slot += 1;
                x
            }
        };
        x_of.insert(node, x);
    }

    for start in roots.into_iter().chain(all) {
        if !depth_of.contains_key(&start) {
            place(
                start,
                0,
                &children,
                &mut x_of,
                &mut depth_of,
                &mut next_slot,
                slot,
            );
        }
    }

    let max_depth = depth_of.values().copied().max().unwrap_or(0) as f32;
    let width = next_slot.saturating_sub(1) as f32 * slot;
    x_of.into_iter()
        .map(|(idx, x)| {
            let y = depth_of[&idx] as f32 * h_layer - max_depth * h_layer / 2.0;
            (idx, egui::pos2(x - width / 2.0, y))
        })
        .collect()
}

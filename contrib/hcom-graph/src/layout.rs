use crate::model::AgentGraph;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::IntoEdgeReferences;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NodePhysics {
    pub pos: egui::Pos2,
    pub vel: egui::Vec2,
    pub pinned: bool,
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
            k_rep: 10000.0,
            k_att: 0.04,
            l_0: 120.0,
            k_center: 0.02,
            damping: 0.85,
            max_speed: 400.0,
            epsilon_sleep: 0.5,
            max_step_duration: std::time::Duration::from_millis(2),
            is_sleeping: false,
        }
    }

    pub fn sync_nodes(&mut self, graph: &AgentGraph) {
        let current_indices: Vec<NodeIndex> = graph.graph.node_indices().collect();

        // Remove deleted nodes
        self.nodes.retain(|idx, _| graph.graph.contains_node(*idx));

        // Add new nodes with initial positioning
        let count = current_indices.len();
        for (i, &idx) in current_indices.iter().enumerate() {
            if !self.nodes.contains_key(&idx) {
                let node_data = &graph.graph[idx];
                let initial_pos = if let Some(ref parent_name) = node_data.parent_name {
                    if let Some(parent_idx) = graph.get_node_index(parent_name) {
                        if let Some(parent_phys) = self.nodes.get(&parent_idx) {
                            let angle = (i as f32) * 1.5;
                            parent_phys.pos + egui::vec2(60.0 * angle.cos(), 60.0 * angle.sin())
                        } else {
                            egui::pos2(0.0, 0.0)
                        }
                    } else {
                        egui::pos2(0.0, 0.0)
                    }
                } else {
                    let angle = (i as f32) * 2.0 * std::f32::consts::PI / (count.max(1) as f32);
                    let radius = 120.0 + (count as f32 * 10.0);
                    egui::pos2(radius * angle.cos(), radius * angle.sin())
                };

                self.nodes.insert(
                    idx,
                    NodePhysics {
                        pos: initial_pos,
                        vel: egui::Vec2::ZERO,
                        pinned: false,
                    },
                );
                self.is_sleeping = false;
            }
        }

        self.sorted_indices = self.nodes.keys().copied().collect();
        self.sorted_indices.sort();

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
    }

    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
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
        }
    }

    /// Performs time-sliced force-directed simulation with a strict frame budget clamp.
    /// Resumes exact cursors across frames for:
    /// 1. Coulomb repulsion (cursor_i, cursor_j)
    /// 2. Hooke spring attraction (edge_cursor)
    /// 3. Velocity/position integration (integrate_cursor)
    ///
    /// Center gravity is computed directly during integration without O(N) pre-pass or allocations.
    /// Guarantees bounded execution and forward progress on every single frame without stagnation or starvation.
    pub fn step(&mut self, graph: &AgentGraph, dt: f32) -> f32 {
        if self.is_sleeping {
            return 0.0;
        }

        let start_time = std::time::Instant::now();
        let clamped_dt = dt.clamp(0.001, 0.033);
        let sub_dt = clamped_dt * 0.5;

        let mut total_motion = 0.0f32;
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

        // Dedicate 70% of budget to repulsion/edges, reserving 30% for integration
        let repulsion_budget = self.max_step_duration.saturating_mul(7) / 10;

        let mut forces = HashMap::<NodeIndex, egui::Vec2>::new();

        // 1. Coulomb repulsion with exact pair cursor
        let total_pairs = n * (n - 1) / 2;
        let mut processed_pairs = 0usize;
        let mut cur_i = self.cursor_i;
        let mut cur_j = self.cursor_j;

        while processed_pairs < total_pairs && cur_i < n - 1 {
            if processed_pairs > 0
                && processed_pairs.is_multiple_of(32)
                && start_time.elapsed() >= repulsion_budget
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

        // 2. Hooke spring attraction with edge cursor
        if !budget_exceeded {
            let mut edge_count = 0usize;
            let start_edge = self.edge_cursor;

            for (i, edge_ref) in graph.graph.edge_references().enumerate() {
                if i < start_edge {
                    continue;
                }
                if edge_count > 0
                    && edge_count.is_multiple_of(32)
                    && start_time.elapsed() >= repulsion_budget
                {
                    budget_exceeded = true;
                    self.edge_cursor = i;
                    break;
                }

                let source = petgraph::visit::EdgeRef::source(&edge_ref);
                let target = petgraph::visit::EdgeRef::target(&edge_ref);

                if let (Some(phys_s), Some(phys_t)) =
                    (self.nodes.get(&source), self.nodes.get(&target))
                {
                    let d = phys_t.pos - phys_s.pos;
                    let dist = d.length().max(1.0);
                    let force_mag = self.k_att * (dist - self.l_0);
                    let f = (d / dist) * force_mag;

                    *forces.entry(source).or_default() += f;
                    *forces.entry(target).or_default() -= f;
                }
                edge_count += 1;
            }

            if !budget_exceeded {
                self.edge_cursor = 0;
            }
        }

        // 3. Integration with integrate cursor and deadline clamp
        let mut integrated_count = 0usize;
        let mut k = self.integrate_cursor;

        while integrated_count < n {
            if integrated_count > 0
                && integrated_count.is_multiple_of(64)
                && start_time.elapsed() >= self.max_step_duration
            {
                budget_exceeded = true;
                break;
            }

            let idx = self.sorted_indices[k];
            if let Some(phys) = self.nodes.get_mut(&idx) {
                if phys.pinned {
                    phys.vel = egui::Vec2::ZERO;
                } else {
                    let pair_f = forces.get(&idx).copied().unwrap_or(egui::Vec2::ZERO);
                    // Center gravity added directly per node without pre-allocation
                    let f = pair_f - self.k_center * phys.pos.to_vec2();
                    phys.vel = (phys.vel + f * sub_dt) * self.damping;

                    let speed = phys.vel.length();
                    if speed > self.max_speed {
                        phys.vel = (phys.vel / speed) * self.max_speed;
                    }

                    phys.pos += phys.vel * sub_dt;
                    total_motion += speed * sub_dt;
                }
            }

            integrated_count += 1;
            k = (k + 1) % n;
        }

        self.integrate_cursor = k;

        if !budget_exceeded && total_motion < self.epsilon_sleep {
            self.is_sleeping = true;
        }

        if budget_exceeded {
            total_motion.max(self.epsilon_sleep + 0.1)
        } else {
            total_motion
        }
    }
}

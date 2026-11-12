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
/// Canvas size in tree cells when it is unknown: the Hierarchy never wraps.
pub const NO_CANVAS: (usize, usize) = (usize::MAX, usize::MAX);
/// Fixed reference area (world units) that bounds the Hierarchy top layer.
/// The arrangement depends only on the graph, never on the window size.
const TREE_REFERENCE_SIZE: egui::Vec2 = egui::vec2(1600.0, 900.0);

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
pub struct Spring {
    pub a: NodeIndex,
    pub b: NodeIndex,
    /// Messages in both directions (total since load, not the time window).
    pub msgs: usize,
    /// Whether this spring connects two instances of the same catalog agent.
    pub is_cluster: bool,
}

/// Duration of the eased move to a planned arrangement after a Reset or a
/// mode switch.
pub const TRANSITION_SECS: f32 = 4.0;
/// Average speed (px/s) of the eased move to settled positions; smoothstep
/// peaks at 1.5x, under 6 px per frame at 60 fps.
const SETTLE_SPEED: f32 = 230.0;
/// Shortest and longest eased move to settled positions.
const SETTLE_MIN_SECS: f32 = 0.3;
const SETTLE_MAX_SECS: f32 = 2.0;
/// Step limits of a simulation that computes target positions: the
/// full-strength phase, and in total with the cooling tail (~750 steps).
const PLAN_CONVERGE_STEPS: u32 = 1500;
const PLAN_MAX_STEPS: u32 = 2500;
/// Step time of a planning simulation: the longest step `simulate` accepts,
/// since no frame shows its intermediate positions.
const PLAN_DT: f32 = 1.0 / 30.0;
/// Share of a frame a plan may use, in frame budgets: no node moves while
/// it runs, so it may take longer than a live simulation step.
const PLAN_BUDGET_FACTOR: u32 = 4;
/// A planning simulation runs at full strength without cooling, so it finds
/// the force equilibrium instead of freezing on the way there; it counts as
/// converged when no node moved `PLAN_SETTLED_PX` over `PLAN_WINDOW` steps.
const PLAN_WINDOW: u32 = 60;
const PLAN_SETTLED_PX: f32 = 1.0;
/// Stiffness of the spring between instances of the same catalog agent; it
/// dominates message springs and repulsion, so the instances sit together.
const CLUSTER_K: f32 = 300.0;

/// Eased move of the unpinned nodes from `from` to `to`.
#[derive(Debug, Clone)]
pub struct Transition {
    from: HashMap<NodeIndex, egui::Pos2>,
    to: HashMap<NodeIndex, egui::Pos2>,
    elapsed: f32,
    duration: f32,
    /// Move to an arrangement computed from scratch (Reset, mode switch); a
    /// structural change re-plans it from scratch. Otherwise it is a move to
    /// settled positions, and any wake cancels it.
    reset: bool,
}

/// Simulation of a copy of the layout, time-sliced across frames, whose
/// resting positions become the targets of an eased move.
#[derive(Clone)]
struct Plan {
    sim: Box<GraphLayout>,
    steps: u32,
}

#[derive(Clone)]
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
    /// Cooling factor (d3-force alpha): scales every force and decays each
    /// step; below `alpha_min` the layout sleeps after its next clean
    /// collision pass, even if the motion threshold was never met.
    pub alpha: f32,
    pub alpha_min: f32,
    /// Per-step decay; 1 - 0.001^(1/900) cools from 1 to `alpha_min` in 900
    /// steps (15 s at 60 fps); a calm graph sleeps earlier on the motion threshold.
    pub alpha_decay: f32,
    /// Alpha after a wake by pin or drag; a structural change reheats to 1.
    pub alpha_reheat: f32,
    /// Node set, spawn structure and communicating pairs at the last sync;
    /// only a change wakes the layout.
    wake_key: Option<u64>,

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
    /// Instances of each catalog agent with at least two of them, rebuilt
    /// with the springs; `bind_agent_groups` keeps them together.
    agent_groups: Vec<Vec<NodeIndex>>,
    /// Positions of user-pinned nodes by name; a pinned node starts there and
    /// the simulation does not move it, in every mode.
    pub pinned_positions: HashMap<String, egui::Pos2>,
    /// Running eased move to a planned arrangement; replaces the simulation
    /// while it lasts.
    pub transition: Option<Transition>,
    /// A node is being dragged: the simulation runs live. Otherwise an awake
    /// layout computes its resting positions first (`plan`) and moves the
    /// nodes there with a transition, so no node swings back and forth.
    pub dragging: bool,
    plan: Option<Plan>,
    /// Planning copy in its full-strength phase: positions at the start of
    /// the current convergence window and the complete steps since then
    /// (see `plan_step`).
    convergence: Option<(Vec<egui::Pos2>, u32)>,
    /// The last `simulate` call finished every pass within the frame budget.
    step_complete: bool,
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
            max_speed: 270.0,
            epsilon_sleep: 0.2,
            max_step_duration: std::time::Duration::from_millis(2),
            is_sleeping: false,
            calm_steps: 0,
            sleep_after_calm_steps: 20,
            alpha: 1.0,
            alpha_min: 0.001,
            alpha_decay: 1.0 - 0.001f32.powf(1.0 / 900.0),
            alpha_reheat: 0.3,
            wake_key: None,
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
            agent_groups: Vec::new(),
            pinned_positions: HashMap::new(),
            transition: None,
            dragging: false,
            plan: None,
            convergence: None,
            step_complete: false,
        }
    }

    /// Switches the mode, resets every pass cursor so no half-finished pass
    /// of the previous mode is mixed in, and wakes the simulation.
    pub fn set_mode(&mut self, mode: LayoutMode) {
        self.mode = mode;
        self.transition = None;
        self.plan = None;
        self.reset_cursors();
        self.tree_key = None;
        self.is_sleeping = false;
        self.calm_steps = 0;
        self.alpha = 1.0;
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
                let pinned = self.pinned_positions.contains_key(&graph.graph[idx].name);
                self.nodes.insert(
                    idx,
                    NodePhysics {
                        pos,
                        vel: egui::Vec2::ZERO,
                        pinned,
                    },
                );
            }
        }

        // Only a structural change wakes the layout; a periodic snapshot of
        // the same graph or a message between known partners does not.
        let key = wake_key(graph);
        let changed = self.wake_key != Some(key);
        if changed {
            self.wake_key = Some(key);
            self.wake_up();
            self.alpha = 1.0;
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
        // A structural change during a transition re-plans it from here.
        if changed && self.transition.is_some() {
            self.start_transition(graph);
        }
    }

    /// Arrangement of the current mode computed from scratch: pinned nodes
    /// stay, every other node starts at its initial position (independent of
    /// its current one) and the simulation runs without frame budget until
    /// it sleeps. In Cluster mode, unpinned nodes without any edge then fill
    /// rows below the others.
    pub fn planned_positions(&self, graph: &AgentGraph) -> HashMap<NodeIndex, egui::Pos2> {
        let mut sim = self.clone();
        sim.transition = None;
        sim.plan = None;
        sim.max_step_duration = std::time::Duration::from_secs(3600);
        sim.tree_key = None;
        sim.reset_cursors();
        let depths = spawn_depths(graph);
        let mut free: Vec<NodeIndex> = sim
            .nodes
            .iter()
            .filter(|(_, p)| !p.pinned)
            .map(|(i, _)| *i)
            .collect();
        free.sort_by_key(|i| (depths.get(i).copied().unwrap_or(0), *i));
        sim.nodes.retain(|_, p| p.pinned);
        if sim.mode == LayoutMode::Hierarchy {
            sim.refresh_tree_targets(graph);
        }
        for &idx in &free {
            let pos = sim.initial_position(graph, idx);
            sim.nodes.insert(
                idx,
                NodePhysics {
                    pos,
                    vel: egui::Vec2::ZERO,
                    pinned: false,
                },
            );
        }
        sim.sorted_indices = sim.nodes.keys().copied().collect();
        sim.sorted_indices.sort();
        sim.is_sleeping = false;
        sim.calm_steps = 0;
        sim.converge_fully();
        for steps in 0..PLAN_MAX_STEPS {
            sim.plan_step(graph, steps);
            if sim.is_sleeping {
                break;
            }
        }
        let mut out: HashMap<NodeIndex, egui::Pos2> =
            sim.nodes.iter().map(|(i, p)| (*i, p.pos)).collect();
        if sim.mode == LayoutMode::Cluster {
            sim.refresh_springs(graph);
            let linked: HashSet<NodeIndex> = sim.springs.iter().flat_map(|s| [s.a, s.b]).collect();
            let mut lone: Vec<NodeIndex> = free
                .iter()
                .copied()
                .filter(|i| !linked.contains(i))
                .collect();
            lone.sort();
            let rest = out
                .iter()
                .filter(|(i, _)| !lone.contains(i))
                .fold(egui::Rect::NOTHING, |b, (_, p)| {
                    b.union(egui::Rect::from_center_size(*p, egui::Vec2::ZERO))
                });
            if !lone.is_empty() && rest.is_finite() {
                let slot = self.tree_slot();
                let cols = ((rest.width() / slot).floor() as usize + 1)
                    .max(lone.len().isqrt())
                    .max(1);
                for (k, &idx) in lone.iter().enumerate() {
                    let (row, col) = (k / cols, k % cols);
                    let in_row = (lone.len() - row * cols).min(cols);
                    let x = rest.center().x + (col as f32 - (in_row - 1) as f32 / 2.0) * slot;
                    let y = rest.max.y + (row + 1) as f32 * slot;
                    out.insert(idx, egui::pos2(x, y));
                }
            }
        }
        out
    }

    /// Starts an eased move of the unpinned nodes to `planned_positions`;
    /// nothing in Manual mode.
    pub fn start_transition(&mut self, graph: &AgentGraph) {
        if self.mode == LayoutMode::Manual {
            return;
        }
        let to = self.planned_positions(graph);
        let from = self.nodes.iter().map(|(i, p)| (*i, p.pos)).collect();
        self.plan = None;
        self.transition = Some(Transition {
            from,
            to,
            elapsed: 0.0,
            duration: TRANSITION_SECS,
            reset: true,
        });
        self.is_sleeping = false;
    }

    /// Advances the simulation of the resting positions within the frame
    /// budget, from the current positions, velocities and alpha. Once it
    /// sleeps (or hits the step limit), the unpinned nodes start an eased
    /// move there that lasts in proportion to the longest distance. The
    /// visible nodes stand still meanwhile.
    fn advance_plan(&mut self, graph: &AgentGraph) -> f32 {
        let start = std::time::Instant::now();
        if self.plan.is_none() {
            let mut sim = self.clone();
            sim.transition = None;
            sim.dragging = false;
            sim.converge_fully();
            self.plan = Some(Plan {
                sim: Box::new(sim),
                steps: 0,
            });
        }
        let plan = self.plan.as_mut().expect("plan was just created");
        let mut this_frame = 0;
        while !plan.sim.is_sleeping && plan.steps < PLAN_MAX_STEPS {
            let left =
                (self.max_step_duration * PLAN_BUDGET_FACTOR).saturating_sub(start.elapsed());
            // At least one (time-sliced) step per frame, so an exhausted
            // budget still makes progress.
            if left.is_zero() && this_frame > 0 {
                return self.epsilon_sleep + 0.1;
            }
            plan.sim.max_step_duration = left;
            plan.sim.plan_step(graph, plan.steps);
            plan.steps += 1;
            this_frame += 1;
        }
        let plan = self.plan.take().expect("plan exists");
        // The targets are the equilibrium: a later wake reheats gently.
        self.alpha = self.alpha.min(self.alpha_min);
        let mut from = HashMap::new();
        let mut to = HashMap::new();
        let mut longest = 0.0f32;
        for (idx, phys) in &self.nodes {
            if let Some(target) = plan.sim.nodes.get(idx)
                && !phys.pinned
            {
                longest = longest.max(phys.pos.distance(target.pos));
                from.insert(*idx, phys.pos);
                to.insert(*idx, target.pos);
            }
        }
        self.transition = Some(Transition {
            from,
            to,
            elapsed: 0.0,
            duration: (longest / SETTLE_SPEED).clamp(SETTLE_MIN_SECS, SETTLE_MAX_SECS),
            reset: false,
        });
        self.epsilon_sleep + 0.1
    }

    /// Makes this planning copy run at full strength without cooling (see
    /// `plan_step`).
    fn converge_fully(&mut self) {
        self.alpha = 1.0;
        self.convergence = Some((Vec::new(), PLAN_WINDOW));
    }

    /// One planning step at `PLAN_DT`. The full-strength phase ends once no
    /// node moved `PLAN_SETTLED_PX` over the last `PLAN_WINDOW` complete
    /// steps (the force equilibrium; contact jitter cancels out over the
    /// window; a step cut by the frame budget does not count) or after
    /// `PLAN_CONVERGE_STEPS`. The cap makes a plan whose steps the budget
    /// keeps cutting (very large graphs) a bounded approximation, not the
    /// equilibrium. A cooling run from `alpha_reheat` then relieves lasting
    /// contact pressure and sleeps as the live layout does.
    fn plan_step(&mut self, graph: &AgentGraph, steps: u32) {
        if steps >= PLAN_CONVERGE_STEPS {
            self.end_convergence();
        }
        if let Some((start, complete)) = &self.convergence
            && *complete >= PLAN_WINDOW
        {
            let now: Vec<egui::Pos2> = self
                .sorted_indices
                .iter()
                .map(|i| self.nodes[i].pos)
                .collect();
            let settled = start.len() == now.len()
                && start
                    .iter()
                    .zip(&now)
                    .all(|(a, b)| a.distance(*b) < PLAN_SETTLED_PX);
            if settled {
                self.end_convergence();
            } else {
                self.convergence = Some((now, 0));
            }
        }
        self.simulate(graph, PLAN_DT);
        if self.step_complete
            && let Some((_, n)) = self.convergence.as_mut()
        {
            *n += 1;
        }
    }

    /// Ends the full-strength planning phase (see `plan_step`).
    fn end_convergence(&mut self) {
        if self.convergence.take().is_some() {
            self.alpha = self.alpha_reheat;
            self.calm_steps = 0;
        }
    }

    /// Advances the transition by `dt`: smoothstep from `from` to `to`, so
    /// the move accelerates and decelerates without a jump. At the end the
    /// nodes rest on their targets and the layout sleeps. Returns the
    /// largest displacement of this frame.
    fn advance_transition(&mut self, dt: f32) -> f32 {
        let Some(tr) = self.transition.as_mut() else {
            return 0.0;
        };
        tr.elapsed += dt.clamp(0.0, 0.1);
        let t = (tr.elapsed / tr.duration).min(1.0);
        let e = t * t * (3.0 - 2.0 * t);
        let mut motion = 0.0f32;
        for (idx, to) in &tr.to {
            if let (Some(from), Some(phys)) = (tr.from.get(idx), self.nodes.get_mut(idx))
                && !phys.pinned
            {
                let p = *from + (*to - *from) * e;
                motion = motion.max(p.distance(phys.pos));
                phys.pos = p;
                phys.vel = egui::Vec2::ZERO;
            }
        }
        if t >= 1.0 {
            self.transition = None;
            self.is_sleeping = true;
            self.calm_steps = 0;
            // A later wake reheats gently instead of re-running the cooling.
            self.alpha = self.alpha.min(self.alpha_min);
            return motion;
        }
        motion.max(self.epsilon_sleep + 0.1)
    }

    /// Position of a new node: its tree target in Hierarchy mode; otherwise
    /// next to its placed parent or first placed message partner; otherwise
    /// a deterministic spiral position (no fixed circle).
    fn initial_position(&self, graph: &AgentGraph, idx: NodeIndex) -> egui::Pos2 {
        if let Some(p) = self.pinned_positions.get(&graph.graph[idx].name) {
            return *p;
        }
        if self.mode == LayoutMode::Hierarchy
            && let Some(t) = self.tree_targets.get(&idx)
        {
            return *t;
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
                tree_targets(graph, self.h_layer, self.tree_slot(), self.tree_cells());
            self.tree_key = Some(key);
        }
    }

    /// Horizontal distance of tree slots in Hierarchy mode.
    fn tree_slot(&self) -> f32 {
        2.0 * self.collision_radius + 20.0
    }

    /// `TREE_REFERENCE_SIZE` in tree slots and layers: the area whose width
    /// bounds the Hierarchy top layer (see `tree_targets`).
    pub fn tree_cells(&self) -> (usize, usize) {
        (
            ((TREE_REFERENCE_SIZE.x / self.tree_slot()).floor() as usize).max(1),
            ((TREE_REFERENCE_SIZE.y / self.h_layer).floor() as usize).max(1),
        )
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
        let mut pairs: HashMap<(NodeIndex, NodeIndex), (usize, bool)> = HashMap::new();
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
            let entry = pairs.entry(pair).or_insert((0, false));
            entry.0 += msgs;
        }

        // Group nodes by their catalog agent name for cluster cohesion
        let mut agent_groups: HashMap<String, Vec<NodeIndex>> = HashMap::new();
        for idx in graph.graph.node_indices() {
            if let Some(node) = graph.graph.node_weight(idx)
                && let Some(ref agent) = node.agent_name
                && !agent.is_empty()
            {
                agent_groups.entry(agent.clone()).or_default().push(idx);
            }
        }

        // Add cluster springs between all pairs of nodes belonging to the same agent
        for group in agent_groups.values() {
            if group.len() >= 2 {
                for i in 0..group.len() {
                    for j in (i + 1)..group.len() {
                        let (a, b) = if group[i] < group[j] {
                            (group[i], group[j])
                        } else {
                            (group[j], group[i])
                        };
                        let entry = pairs.entry((a, b)).or_insert((0, true));
                        entry.1 = true;
                    }
                }
            }
        }

        let mut springs: Vec<Spring> = pairs
            .into_iter()
            .map(|((a, b), (msgs, is_cluster))| Spring {
                a,
                b,
                msgs,
                is_cluster,
            })
            .collect();
        springs.sort_by_key(|s| (s.a, s.b));
        self.springs = springs;
        let mut groups: Vec<Vec<NodeIndex>> = agent_groups
            .into_values()
            .filter(|g| g.len() >= 2)
            .map(|mut g| {
                g.sort();
                g
            })
            .collect();
        groups.sort();
        self.agent_groups = groups;
        self.springs_key = Some(key);
        self.edge_cursor = 0;
    }

    /// Rigid link of the instances of one catalog agent: every instance
    /// farther than ring contact from the group's anchor (its first pinned
    /// instance, otherwise its first instance) is pulled back to ring
    /// contact. Like a collision, an unpinned pair shares the correction, so
    /// the forces on every instance move the whole group as one body;
    /// collision then spreads the instances around the anchor.
    fn bind_agent_groups(&mut self) {
        let reach = 2.0 * (self.collision_radius + COLLISION_MARGIN);
        for group in &self.agent_groups {
            let Some(&anchor) = group
                .iter()
                .find(|i| self.nodes.get(i).is_some_and(|p| p.pinned))
                .or_else(|| group.iter().find(|i| self.nodes.contains_key(i)))
            else {
                continue;
            };
            for &idx in group {
                let (Some(a), Some(p)) = (self.nodes.get(&anchor), self.nodes.get(&idx)) else {
                    continue;
                };
                if idx == anchor || p.pinned {
                    continue;
                }
                let d = p.pos - a.pos;
                let excess = d.length() - reach;
                if excess <= 0.0 {
                    continue;
                }
                let dir = d.normalized();
                let anchor_share = if a.pinned { 0.0 } else { 0.5 };
                if let Some(a) = self.nodes.get_mut(&anchor) {
                    a.pos += dir * (excess * anchor_share);
                }
                if let Some(p) = self.nodes.get_mut(&idx) {
                    p.pos -= dir * (excess * (1.0 - anchor_share));
                }
            }
        }
    }

    /// Access to aggregated springs (for tests and inspection).
    pub fn springs(&self) -> &[Spring] {
        &self.springs
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

    /// Stiffness and rest length taking cluster cohesion into account:
    /// instances of one agent rest just beyond ring contact.
    pub fn spring_params_for(&self, s: &Spring) -> (f32, f32) {
        if s.is_cluster {
            let (k, _) = self.spring_params(s.msgs);
            return (
                k.max(CLUSTER_K),
                2.0 * (self.collision_radius + COLLISION_MARGIN),
            );
        }
        self.spring_params(s.msgs)
    }

    /// Drops cached tree targets and springs, e.g. after the graph object was
    /// replaced and node or edge indices may be reused in another order.
    pub fn invalidate_caches(&mut self) {
        self.tree_key = None;
        self.springs_key = None;
    }

    /// Wakes the simulation and reheats it to at least `alpha_reheat`. A
    /// computed plan and a move to settled positions are stale: the nodes stop
    /// where they are and the plan restarts from there.
    pub fn wake_up(&mut self) {
        self.plan = None;
        if self.transition.as_ref().is_some_and(|t| !t.reset) {
            self.transition = None;
        }
        self.is_sleeping = false;
        self.calm_steps = 0;
        self.alpha = self.alpha.max(self.alpha_reheat);
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
        // A node placed by hand leaves a running transition.
        if let Some(tr) = self.transition.as_mut() {
            tr.to.remove(&idx);
        }
        if let Some(phys) = self.nodes.get_mut(&idx) {
            phys.pos = pos;
            phys.vel = egui::Vec2::ZERO;
            phys.pinned = pinned;
            self.wake_up();
        }
    }

    /// Pins a node at its current position (`pin`) or releases it; returns
    /// the pinned position, `None` when unpinned or the node is unknown.
    pub fn set_pinned(&mut self, idx: NodeIndex, name: &str, pin: bool) -> Option<egui::Pos2> {
        let phys = self.nodes.get_mut(&idx)?;
        phys.pinned = pin;
        phys.vel = egui::Vec2::ZERO;
        let pos = phys.pos;
        self.wake_up();
        if pin {
            self.pinned_positions.insert(name.to_string(), pos);
            Some(pos)
        } else {
            self.pinned_positions.remove(name);
            None
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
    /// Manual mode does not simulate. While a node is dragged the simulation
    /// runs live (`simulate`); a Reset transition keeps running (the drag's
    /// wakes cancel any other). Otherwise an awake layout plans its resting
    /// positions and eases the nodes there.
    pub fn step(&mut self, graph: &AgentGraph, dt: f32) -> f32 {
        if self.mode == LayoutMode::Manual {
            return 0.0;
        }
        if self.dragging && self.transition.is_none() {
            self.plan = None;
            return self.simulate(graph, dt);
        }
        if self.transition.is_some() {
            return self.advance_transition(dt);
        }
        if self.is_sleeping {
            return 0.0;
        }
        self.advance_plan(graph)
    }

    /// One live simulation step. The layout sleeps when the last complete
    /// collision pass was clean and either motion stayed below the threshold
    /// or alpha cooled below `alpha_min`.
    fn simulate(&mut self, graph: &AgentGraph, dt: f32) -> f32 {
        if self.is_sleeping {
            return 0.0;
        }
        let start_time = std::time::Instant::now();
        let clamped_dt = dt.clamp(0.001, 0.033);
        // Integration time scale (1.5x the original dt/2): fast enough that a
        // node finishes its travel while alpha cools, slow enough that pushed
        // and anchored nodes do not visibly bounce; with `max_speed` 270 a node
        // moves at most ~3.4 px per frame at 60 fps (TASK-77.28).
        let sub_dt = clamped_dt * 0.75;

        let mut budget_exceeded = false;

        if self.sorted_indices.len() != self.nodes.len() {
            self.sorted_indices = self.nodes.keys().copied().collect();
            self.sorted_indices.sort();
        }

        let n = self.sorted_indices.len();
        self.step_complete = n < 2;
        if n < 2 {
            return 0.0;
        }

        // Cooled down: no forces and no integration, only collision. The pass
        // restarted when alpha crossed `alpha_min`, so a clean result covers
        // every pair after the last integration.
        if self.alpha < self.alpha_min {
            self.collide(start_time, n * (n - 1) / 2);
            if self.last_collision_pass_clean {
                self.is_sleeping = true;
                return 0.0;
            }
            return self.epsilon_sleep + 0.1;
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

        // Forces by position in `sorted_indices`; the pair loops avoid map lookups.
        let mut forces = vec![egui::Vec2::ZERO; n];
        let slot: HashMap<NodeIndex, usize> = self
            .sorted_indices
            .iter()
            .enumerate()
            .map(|(k, i)| (*i, k))
            .collect();

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

            let d = start_pos[cur_j] - start_pos[cur_i];
            let dist_sq = d.length_sq().max(1.0);
            let dist = dist_sq.sqrt();

            let force_mag = self.k_rep / dist_sq;
            let f = (d / dist) * force_mag;

            forces[cur_i] -= f;
            forces[cur_j] += f;

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
                // Hierarchy: the tree places spawn pairs; messages and clusters act on x only, weaker.
                if !(hierarchy && s.msgs == 0 && !s.is_cluster)
                    && let (Some(&a), Some(&b)) = (slot.get(&s.a), slot.get(&s.b))
                {
                    let (k, l) = self.spring_params_for(&s);
                    let d = start_pos[b] - start_pos[a];
                    let dist = d.length().max(1.0);
                    let mut f = (d / dist) * (k * (dist - l));
                    if hierarchy {
                        f = egui::vec2(f.x * 0.5, 0.0);
                    }
                    forces[a] += f;
                    forces[b] -= f;
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
                    let pair_f = forces[k];
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
                    phys.vel = (phys.vel + f * (self.alpha * sub_dt)) * self.damping;

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

        if !hierarchy {
            self.bind_agent_groups();
        }

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

        // A planning copy in its full-strength phase neither sleeps nor cools.
        let converging = self.convergence.is_some();
        if !converging
            && !budget_exceeded
            && total_motion < self.epsilon_sleep
            && self.last_collision_pass_clean
        {
            self.calm_steps += 1;
            if self.calm_steps >= self.sleep_after_calm_steps {
                self.is_sleeping = true;
            }
        } else {
            self.calm_steps = 0;
        }
        // Cooling counts steps, not time, so the layout stops even when the
        // frame budget cuts passes short.
        if !converging {
            self.alpha *= 1.0 - self.alpha_decay;
        }
        if self.alpha < self.alpha_min {
            // Start a fresh collision pass; the previous result predates the
            // last integration.
            self.collision_done = 0;
            self.collision_pass_clean = true;
            self.last_collision_pass_clean = false;
        }

        self.step_complete = !budget_exceeded;
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
        // Local copies: the pair loop avoids map lookups.
        let mut pos: Vec<egui::Pos2> = Vec::with_capacity(n);
        let mut pinned: Vec<bool> = Vec::with_capacity(n);
        for idx in &self.sorted_indices {
            let p = &self.nodes[idx];
            pos.push(p.pos);
            pinned.push(p.pinned);
        }
        let mut processed = 0usize;
        let mut complete = true;
        while processed < total_pairs {
            if processed > 0
                && processed.is_multiple_of(64)
                && start_time.elapsed() >= self.max_step_duration
            {
                complete = false;
                break;
            }
            let (ci, cj) = (self.collision_i, self.collision_j);
            let (pin_i, pin_j) = (pinned[ci], pinned[cj]);
            let d = pos[cj] - pos[ci];
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
                pos[ci] -= dir * move_i;
                pos[cj] += dir * move_j;
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
        for (idx, p) in self.sorted_indices.iter().zip(pos) {
            if let Some(phys) = self.nodes.get_mut(idx) {
                phys.pos = p;
            }
        }
        complete
    }
}

/// Order-independent key of the node set, the spawn structure and the agent
/// of each instance.
fn structure_key(graph: &AgentGraph) -> u64 {
    let mut sum = graph.graph.node_count() as u64;
    for (parent, child) in graph.spawn_edges.keys() {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (parent, child).hash(&mut h);
        sum = sum.wrapping_add(h.finish());
    }
    for idx in graph.graph.node_indices() {
        sum = sum.wrapping_add((idx.index() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        // The agent often arrives with a later snapshot than the node; it
        // changes the cluster springs.
        if let Some(agent) = &graph.graph[idx].agent_name {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (idx, agent).hash(&mut h);
            sum = sum.wrapping_add(h.finish());
        }
    }
    sum
}

/// Order-independent key of the structure plus the set of communicating
/// pairs; message counts are left out.
fn wake_key(graph: &AgentGraph) -> u64 {
    let pairs: HashSet<(NodeIndex, NodeIndex)> = graph
        .graph
        .edge_references()
        .filter(|e| matches!(e.weight(), EdgeKind::Message(_)) && e.source() != e.target())
        .map(|e| {
            let (s, t) = (e.source(), e.target());
            if s < t { (s, t) } else { (t, s) }
        })
        .collect();
    let mut sum = structure_key(graph);
    for pair in pairs {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        pair.hash(&mut h);
        sum = sum.wrapping_add(h.finish());
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
/// When the top layer needs more slots than the canvas width holds
/// (`canvas` = size at zoom 1 in slots and layers), roots without children
/// leave it and fill rows below the trees, one `h_layer` apart, each row
/// centered; the row length (at least the tree width) is the one whose block
/// fits the canvas at the largest zoom. The result is centered around the
/// origin.
pub fn tree_targets(
    graph: &AgentGraph,
    h_layer: f32,
    slot: f32,
    canvas: (usize, usize),
) -> HashMap<NodeIndex, egui::Pos2> {
    let (roots, children) = spawn_forest(graph);
    let is_lone = |n: &NodeIndex| children.get(n).is_none_or(|c| c.is_empty());
    let targets = tidy_forest(graph, &roots, &children, h_layer, slot, None);
    let lone: Vec<NodeIndex> = roots.iter().copied().filter(is_lone).collect();
    let (cols, rows) = canvas;
    if targets.trees_leaf_slots <= cols || lone.is_empty() {
        return targets.positions;
    }
    let lone_set: HashSet<NodeIndex> = lone.iter().copied().collect();
    let trees = tidy_forest(graph, &roots, &children, h_layer, slot, Some(&lone_set));
    let tree_rows = if trees.positions.is_empty() {
        0
    } else {
        trees.max_depth + 1
    };
    // Row capacity (at least the tree width) whose block fits the canvas at
    // the largest zoom; the smallest such capacity on a tie.
    let fit_scale = |cap: usize| {
        let total = tree_rows + lone.len().div_ceil(cap);
        (cols as f32 / cap as f32).min(rows as f32 / total as f32)
    };
    let min_cap = trees.trees_leaf_slots.max(1);
    let row_cap = (min_cap..=min_cap.max(lone.len()))
        .fold((min_cap, f32::MIN), |best, cap| {
            let s = fit_scale(cap);
            if s > best.1 { (cap, s) } else { best }
        })
        .0;
    let lone_rows = lone.len().div_ceil(row_cap);
    // Shift the tree block so trees and lone rows together are centered in y.
    let total_rows = tree_rows + lone_rows;
    let top = -((total_rows - 1) as f32) * h_layer / 2.0;
    let tree_top = -(trees.max_depth as f32) * h_layer / 2.0;
    let mut out: HashMap<NodeIndex, egui::Pos2> = trees
        .positions
        .into_iter()
        .map(|(idx, p)| (idx, egui::pos2(p.x, p.y - tree_top + top)))
        .collect();
    for (k, &idx) in lone.iter().enumerate() {
        let row = k / row_cap;
        let col = k % row_cap;
        let in_row = (lone.len() - row * row_cap).min(row_cap);
        let x = (col as f32 - (in_row - 1) as f32 / 2.0) * slot;
        let y = top + (tree_rows + row) as f32 * h_layer;
        out.insert(idx, egui::pos2(x, y));
    }
    out
}

struct TidyForest {
    positions: HashMap<NodeIndex, egui::Pos2>,
    /// Leaf slots used, i.e. the width of the layer in slots.
    trees_leaf_slots: usize,
    max_depth: usize,
}

/// Tidy layout of the forest started from `roots` and then from every
/// unreached node (cycles), skipping the `skip` roots; centered around the
/// origin.
fn tidy_forest(
    graph: &AgentGraph,
    roots: &[NodeIndex],
    children: &HashMap<NodeIndex, Vec<NodeIndex>>,
    h_layer: f32,
    slot: f32,
    skip: Option<&HashSet<NodeIndex>>,
) -> TidyForest {
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

    for &start in roots.iter().chain(&all) {
        if !depth_of.contains_key(&start) && !skip.is_some_and(|s| s.contains(&start)) {
            place(
                start,
                0,
                children,
                &mut x_of,
                &mut depth_of,
                &mut next_slot,
                slot,
            );
        }
    }

    let max_depth = depth_of.values().copied().max().unwrap_or(0);
    let width = next_slot.saturating_sub(1) as f32 * slot;
    let positions = x_of
        .into_iter()
        .map(|(idx, x)| {
            let y = depth_of[&idx] as f32 * h_layer - max_depth as f32 * h_layer / 2.0;
            (idx, egui::pos2(x - width / 2.0, y))
        })
        .collect();
    TidyForest {
        positions,
        trees_leaf_slots: next_slot,
        max_depth,
    }
}

use crate::config::ResolvedTheme;
use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus, EdgeKind, WindowedCount};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

/// Zoom range of the canvas (also used to sanitize persisted zoom).
pub const ZOOM_MIN: f32 = 0.15;
pub const ZOOM_MAX: f32 = 4.0;

/// Canvas colors also drawn by the legend (detail panel).
pub const SPAWN_EDGE_COLOR: Color32 = Color32::from_rgb(110, 120, 140);
pub const MESSAGE_EDGE_COLOR: Color32 = Color32::from_rgb(165, 105, 189);
pub const MESSAGE_EDGE_IDLE_COLOR: Color32 =
    Color32::from_rgba_unmultiplied_const(165, 105, 189, 50);
pub const ROAMING_OUTLINE_COLOR: Color32 = Color32::from_rgb(200, 205, 220);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemePalette {
    pub resolved: ResolvedTheme,
    pub canvas_bg: Color32,
    pub grid_dot: Color32,
    pub node_fill: Color32,
    pub node_stroke: Color32,
    pub node_text_primary: Color32,
    pub node_text_secondary: Color32,
    // Edges
    pub spawn_edge: Color32,
    pub spawn_edge_hover: Color32,
    pub message_edge: Color32,
    pub message_edge_hover: Color32,
    pub message_edge_idle: Color32,
    /// Main arrow from the side that started the conversation of a pair.
    pub initiator_edge: Color32,
    pub edge_selected: Color32,
    pub roaming_outline: Color32,
    // Badges & labels
    pub badge_fill: Color32,
    pub badge_stroke: Color32,
    pub badge_text: Color32,
    pub message_label_text: Color32,
    pub message_label_outline: Color32,
    pub selected_ring: Color32,
    // Node pins & context fills
    pub pin_fill: Color32,
    pub pin_stroke: Color32,
    pub context_fill_low: Color32,
    pub context_fill_mid: Color32,
    pub context_fill_high: Color32,
    // Sliders
    pub slider_track: Color32,
    pub slider_fill: Color32,
    pub slider_knob: Color32,
    pub slider_stroke: Color32,
}

impl ThemePalette {
    pub fn for_theme(resolved: ResolvedTheme) -> Self {
        match resolved {
            ResolvedTheme::Dark => Self {
                resolved,
                canvas_bg: Color32::from_rgb(18, 19, 24),
                grid_dot: Color32::from_rgba_premultiplied(40, 42, 54, 50),
                node_fill: Color32::from_rgb(28, 30, 38),
                node_stroke: Color32::from_rgb(40, 42, 54),
                node_text_primary: Color32::from_rgb(236, 240, 241),
                node_text_secondary: Color32::from_rgb(160, 165, 180),
                spawn_edge: Color32::from_rgb(110, 120, 140),
                spawn_edge_hover: Color32::from_rgb(170, 185, 210),
                message_edge: Color32::from_rgb(165, 105, 189),
                message_edge_hover: Color32::from_rgb(210, 150, 235),
                message_edge_idle: Color32::from_rgba_unmultiplied(165, 105, 189, 50),
                initiator_edge: Color32::from_rgba_unmultiplied(26, 188, 156, 150),
                edge_selected: Color32::from_rgb(241, 196, 15),
                roaming_outline: Color32::from_rgb(200, 205, 220),
                badge_fill: Color32::from_rgb(35, 38, 50),
                badge_stroke: Color32::from_rgb(70, 75, 95),
                badge_text: Color32::from_rgb(220, 220, 230),
                message_label_text: Color32::from_rgb(210, 150, 235),
                message_label_outline: Color32::from_rgb(18, 19, 24),
                selected_ring: Color32::from_rgb(241, 196, 15),
                pin_fill: Color32::from_rgb(236, 240, 241),
                pin_stroke: Color32::from_rgb(18, 19, 24),
                context_fill_low: Color32::from_rgb(46, 204, 113),
                context_fill_mid: Color32::from_rgb(243, 156, 18),
                context_fill_high: Color32::from_rgb(231, 76, 60),
                slider_track: Color32::from_rgb(55, 55, 60),
                slider_fill: Color32::from_rgb(70, 70, 80),
                slider_knob: Color32::WHITE,
                slider_stroke: Color32::from_rgb(100, 100, 100),
            },
            ResolvedTheme::Light => Self {
                resolved,
                canvas_bg: Color32::from_rgb(245, 247, 250),
                grid_dot: Color32::from_rgba_premultiplied(180, 188, 205, 60),
                node_fill: Color32::from_rgb(255, 255, 255),
                node_stroke: Color32::from_rgb(210, 215, 225),
                node_text_primary: Color32::from_rgb(24, 28, 36),
                node_text_secondary: Color32::from_rgb(85, 92, 105),
                spawn_edge: Color32::from_rgb(100, 110, 130),
                spawn_edge_hover: Color32::from_rgb(70, 85, 115),
                message_edge: Color32::from_rgb(142, 68, 173),
                message_edge_hover: Color32::from_rgb(125, 45, 155),
                message_edge_idle: Color32::from_rgba_unmultiplied(142, 68, 173, 60),
                initiator_edge: Color32::from_rgba_unmultiplied(13, 148, 136, 150),
                edge_selected: Color32::from_rgb(217, 119, 6),
                roaming_outline: Color32::from_rgb(85, 95, 115),
                badge_fill: Color32::from_rgb(235, 238, 245),
                badge_stroke: Color32::from_rgb(180, 188, 205),
                badge_text: Color32::from_rgb(30, 35, 45),
                message_label_text: Color32::from_rgb(125, 45, 155),
                message_label_outline: Color32::from_rgb(245, 247, 250),
                selected_ring: Color32::from_rgb(217, 119, 6),
                pin_fill: Color32::from_rgb(30, 35, 45),
                pin_stroke: Color32::from_rgb(245, 247, 250),
                context_fill_low: Color32::from_rgb(21, 128, 61),
                context_fill_mid: Color32::from_rgb(180, 83, 9),
                context_fill_high: Color32::from_rgb(185, 28, 28),
                slider_track: Color32::from_rgb(210, 215, 225),
                slider_fill: Color32::from_rgb(160, 175, 195),
                slider_knob: Color32::WHITE,
                slider_stroke: Color32::from_rgb(140, 145, 160),
            },
        }
    }

    pub fn status_color(&self, status: &AgentStatus) -> Color32 {
        match self.resolved {
            ResolvedTheme::Dark => match status {
                AgentStatus::Active => Color32::from_rgb(0x2E, 0xCC, 0x71),
                AgentStatus::Listening => Color32::from_rgb(0x34, 0x98, 0xDB),
                AgentStatus::Blocked => Color32::from_rgb(0xE7, 0x4C, 0x3C),
                AgentStatus::Stopped => Color32::from_rgb(0x7F, 0x8C, 0x8D),
            },
            ResolvedTheme::Light => match status {
                AgentStatus::Active => Color32::from_rgb(21, 128, 61),
                AgentStatus::Listening => Color32::from_rgb(29, 78, 216),
                AgentStatus::Blocked => Color32::from_rgb(185, 28, 28),
                AgentStatus::Stopped => Color32::from_rgb(75, 85, 99),
            },
        }
    }

    pub fn cli_color(&self, tool: Option<&str>) -> Color32 {
        let t = tool.map(|s| s.to_ascii_lowercase());
        match self.resolved {
            ResolvedTheme::Dark => match t.as_deref() {
                Some("claude") => Color32::from_rgb(0xD9, 0x77, 0x06),
                Some("codex") => Color32::from_rgb(0x10, 0xA3, 0x7F),
                Some("agy" | "antigravity" | "gemini") => Color32::from_rgb(0x42, 0x85, 0xF4),
                Some("hermes") => Color32::from_rgb(0xE8, 0x4A, 0x8A),
                Some("kimi") => Color32::from_rgb(0x8B, 0x5C, 0xF6),
                Some("grok") => Color32::from_rgb(0xE5, 0xE7, 0xEB),
                Some("opencode" | "kilo") => Color32::from_rgb(0x06, 0xB6, 0xD4),
                Some("pi" | "omp") => Color32::from_rgb(0xEA, 0xB3, 0x08),
                Some("cursor" | "copilot" | "qoder") => Color32::from_rgb(0x84, 0xCC, 0x16),
                _ => Color32::from_rgb(0x64, 0x74, 0x8B),
            },
            ResolvedTheme::Light => match t.as_deref() {
                Some("claude") => Color32::from_rgb(180, 83, 9),
                Some("codex") => Color32::from_rgb(4, 120, 87),
                Some("agy" | "antigravity" | "gemini") => Color32::from_rgb(29, 78, 216),
                Some("hermes") => Color32::from_rgb(190, 24, 93),
                Some("kimi") => Color32::from_rgb(109, 40, 217),
                Some("grok") => Color32::from_rgb(55, 65, 81),
                Some("opencode" | "kilo") => Color32::from_rgb(14, 116, 144),
                Some("pi" | "omp") => Color32::from_rgb(161, 98, 7),
                Some("cursor" | "copilot" | "qoder") => Color32::from_rgb(77, 124, 15),
                _ => Color32::from_rgb(71, 85, 105),
            },
        }
    }

    pub fn context_fill_color(&self, percent: f32) -> Color32 {
        if percent > 85.0 {
            self.context_fill_high
        } else if percent >= 70.0 {
            self.context_fill_mid
        } else {
            self.context_fill_low
        }
    }
}

/// The node and edge encodings visible in the current graph (see `legend`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Legend {
    /// Status border ring colors, in `AgentStatus` order.
    pub statuses: Vec<AgentStatus>,
    /// CLI ring colors with the tool names drawn in that color.
    pub cli: Vec<(Color32, Vec<String>)>,
    /// Dashed outer outline of roaming agents.
    pub roaming: bool,
    /// Dashed spawn edges.
    pub spawn: bool,
    /// Solid message edges with messages in the window.
    pub message: bool,
    /// Dimmed message edges without messages in the window.
    pub message_idle: bool,
}

/// Legend of what the canvas draws: nodes in `layout` and `visible`, and
/// edges whose both endpoints are such nodes (the canvas visibility rule).
/// `edge_counts` are the windowed counts as in `CanvasView::edge_counts`.
pub fn legend(
    graph: &AgentGraph,
    layout: &GraphLayout,
    visible: &std::collections::HashSet<NodeIndex>,
    edge_counts: Option<&std::collections::HashMap<EdgeIndex, WindowedCount>>,
    palette: &ThemePalette,
) -> Legend {
    let shown = |idx: &NodeIndex| layout.nodes.contains_key(idx) && visible.contains(idx);
    let mut legend = Legend::default();
    let mut tools: Vec<String> = Vec::new();
    for idx in graph.graph.node_indices().filter(shown) {
        let node = &graph.graph[idx];
        if !legend.statuses.contains(&node.status) {
            legend.statuses.push(node.status);
        }
        let tool = node
            .tool
            .as_deref()
            .map(str::to_ascii_lowercase)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "unknown".into());
        if !tools.contains(&tool) {
            tools.push(tool);
        }
        legend.roaming |= graph.node_style(&node.name).is_roaming;
    }
    let order = [
        AgentStatus::Active,
        AgentStatus::Listening,
        AgentStatus::Blocked,
        AgentStatus::Stopped,
    ];
    legend
        .statuses
        .sort_by_key(|s| order.iter().position(|o| o == s));
    tools.sort();
    for tool in tools {
        let color = palette.cli_color(Some(&tool));
        match legend.cli.iter_mut().find(|(c, _)| *c == color) {
            Some((_, names)) => names.push(tool),
            None => legend.cli.push((color, vec![tool])),
        }
    }
    for e in graph.graph.edge_references() {
        if !shown(&e.source()) || !shown(&e.target()) {
            continue;
        }
        match e.weight() {
            EdgeKind::Spawn(_) => legend.spawn = true,
            EdgeKind::Message(msg) => {
                let count = match edge_counts {
                    Some(map) => map.get(&e.id()).map_or(0, |c| c.count),
                    None => msg.count,
                };
                if count > 0 {
                    legend.message = true;
                } else {
                    legend.message_idle = true;
                }
            }
        }
    }
    legend
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    NodeClicked(NodeIndex),
    NodeDoubleClicked(NodeIndex),
    EdgeClicked(EdgeIndex),
    NodeSecondaryClicked(NodeIndex, Pos2),
    EdgeSecondaryClicked(EdgeIndex, Pos2),
    SelectionCleared,
}

pub struct CanvasView {
    pub pan: Vec2,
    pub zoom: f32,
    pub dragged_node: Option<NodeIndex>,
    pub pending_drag_node: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,
    pub hovered_edge: Option<EdgeIndex>,
    pub selected_node: Option<NodeIndex>,
    pub selected_edge: Option<EdgeIndex>,
    pub mouse_down_pos: Option<Pos2>,
    pub is_dragging: bool,
    /// Windowed message counts per edge; `None` = totals (no time window).
    pub edge_counts: Option<std::collections::HashMap<EdgeIndex, WindowedCount>>,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self::new()
    }
}

pub fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-4 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}

pub fn distance_to_quadratic_bezier(p: Pos2, p0: Pos2, p1: Pos2, p2: Pos2) -> f32 {
    let mut min_dist = f32::MAX;
    let steps = 16;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let it = 1.0 - t;
        let curr = Pos2::new(
            it * it * p0.x + 2.0 * it * t * p1.x + t * t * p2.x,
            it * it * p0.y + 2.0 * it * t * p1.y + t * t * p2.y,
        );
        let dist = distance_to_segment(p, prev, curr);
        if dist < min_dist {
            min_dist = dist;
        }
        prev = curr;
    }
    min_dist
}

impl CanvasView {
    pub fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            dragged_node: None,
            pending_drag_node: None,
            hovered_node: None,
            hovered_edge: None,
            selected_node: None,
            selected_edge: None,
            mouse_down_pos: None,
            is_dragging: false,
            edge_counts: None,
        }
    }

    pub fn world_to_screen(&self, world_pos: Pos2, rect: Rect) -> Pos2 {
        rect.center() + self.pan + world_pos.to_vec2() * self.zoom
    }

    pub fn screen_to_world(&self, screen_pos: Pos2, rect: Rect) -> Pos2 {
        let offset = screen_pos - rect.center() - self.pan;
        Pos2::new(offset.x / self.zoom, offset.y / self.zoom)
    }

    pub fn status_color(status: &AgentStatus) -> Color32 {
        ThemePalette::for_theme(ResolvedTheme::Dark).status_color(status)
    }

    /// Accent ring color of a node by its CLI tool.
    pub fn cli_color(tool: Option<&str>) -> Color32 {
        ThemePalette::for_theme(ResolvedTheme::Dark).cli_color(tool)
    }

    /// Context fill color: green below 70 %, amber 70–85 %, red above 85 %.
    pub fn context_fill_color(percent: f32) -> Color32 {
        ThemePalette::for_theme(ResolvedTheme::Dark).context_fill_color(percent)
    }

    /// Distance from a node center where edges start and end: outside the
    /// CLI (+3.5), roaming (+7.5) and selection (+11) rings, so arrow heads
    /// are never covered by node decorations drawn after the edges.
    fn edge_gap(&self) -> f32 {
        (26.0 + 13.0) * self.zoom
    }

    /// Sets zoom and pan so every item fits `rect` (see `fit_view`).
    /// Returns false (view unchanged) when there is nothing to fit.
    pub fn fit_to(&mut self, items: &[(Pos2, f32)], rect: Rect, max_zoom: f32) -> bool {
        match fit_view(items, rect, max_zoom) {
            Some((zoom, pan)) => {
                self.zoom = zoom;
                self.pan = pan;
                true
            }
            None => false,
        }
    }

    pub fn hit_test_edge(
        &self,
        screen_pos: Pos2,
        graph: &AgentGraph,
        layout: &GraphLayout,
        rect: Rect,
        hit_threshold: f32,
    ) -> Option<EdgeIndex> {
        self.hit_test_edge_filtered(screen_pos, graph, layout, rect, hit_threshold, None)
    }

    pub fn hit_test_edge_filtered(
        &self,
        screen_pos: Pos2,
        graph: &AgentGraph,
        layout: &GraphLayout,
        rect: Rect,
        hit_threshold: f32,
        visible_nodes: Option<&std::collections::HashSet<NodeIndex>>,
    ) -> Option<EdgeIndex> {
        for edge_ref in graph.graph.edge_references() {
            let source_idx = edge_ref.source();
            let target_idx = edge_ref.target();

            if let Some(v) = visible_nodes
                && (!v.contains(&source_idx) || !v.contains(&target_idx))
            {
                continue;
            }

            let Some(source_phys) = layout.nodes.get(&source_idx) else {
                continue;
            };
            let Some(target_phys) = layout.nodes.get(&target_idx) else {
                continue;
            };

            let p1 = self.world_to_screen(source_phys.pos, rect);
            let p2 = self.world_to_screen(target_phys.pos, rect);

            let d = p2 - p1;
            let len = d.length();
            if len <= 2.0 * self.edge_gap() {
                continue;
            }

            let dir = d / len;
            let normal = Vec2::new(-dir.y, dir.x);
            let is_reciprocal = graph.graph.contains_edge(target_idx, source_idx);
            let curve_offset = if is_reciprocal { 18.0 * self.zoom } else { 0.0 };

            let edge_gap = self.edge_gap();
            let start_pt = p1 + dir * edge_gap;
            let end_pt = p2 - dir * edge_gap;

            let dist = if curve_offset.abs() > 0.1 {
                let mid = p1 + (p2 - p1) * 0.5;
                let ctrl_pt = mid + normal * curve_offset;
                distance_to_quadratic_bezier(screen_pos, start_pt, ctrl_pt, end_pt)
            } else {
                distance_to_segment(screen_pos, start_pt, end_pt)
            };

            if dist <= hit_threshold {
                return Some(edge_ref.id());
            }
        }
        None
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        graph: &AgentGraph,
        layout: &mut GraphLayout,
        visible_nodes: Option<&std::collections::HashSet<NodeIndex>>,
        palette: &ThemePalette,
    ) -> (egui::Response, Option<CanvasAction>) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let mut triggered_action = None;

        // Clear selection if selected element became filtered out
        if let Some(sel) = self.selected_node
            && let Some(v) = visible_nodes
            && !v.contains(&sel)
        {
            self.selected_node = None;
        }
        if let Some(sel_e) = self.selected_edge
            && let Some(v) = visible_nodes
        {
            if let Some((src, tgt)) = graph.graph.edge_endpoints(sel_e) {
                if !v.contains(&src) || !v.contains(&tgt) {
                    self.selected_edge = None;
                }
            } else {
                self.selected_edge = None;
            }
        }

        // 1. Zoom interaction (anchored at mouse cursor)
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        let zoom_factor = if scroll_delta != 0.0 {
            (scroll_delta * 0.002).exp()
        } else {
            ui.input(|i| i.zoom_delta())
        };

        if zoom_factor != 1.0
            && let Some(hover_pos) = response.hover_pos()
        {
            let world_under_mouse = self.screen_to_world(hover_pos, rect);
            let new_zoom = (self.zoom * zoom_factor).clamp(ZOOM_MIN, ZOOM_MAX);
            self.pan = hover_pos - rect.center() - world_under_mouse.to_vec2() * new_zoom;
            self.zoom = new_zoom;
        }

        // 2. Hit testing & Click vs. Drag handling
        let hit_radius_world = 26.0;

        // Interaction position while pressed or dragged, hover position otherwise
        // (interact_pointer_pos alone is None on a plain hover).
        if let Some(mouse_pos) = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos())
        {
            let world_mouse = self.screen_to_world(mouse_pos, rect);
            self.hovered_node = layout
                .hit_test(world_mouse, hit_radius_world)
                .filter(|idx| visible_nodes.is_none_or(|v| v.contains(idx)));
            if self.hovered_node.is_none() {
                self.hovered_edge = self.hit_test_edge_filtered(
                    mouse_pos,
                    graph,
                    layout,
                    rect,
                    10.0,
                    visible_nodes,
                );
            } else {
                self.hovered_edge = None;
            }

            // Track mouse down pos to distinguish small click from drag
            if (response.drag_started_by(egui::PointerButton::Primary)
                || (ui.input(|i| i.pointer.primary_pressed()) && response.contains_pointer()))
                && self.mouse_down_pos.is_none()
            {
                self.mouse_down_pos = Some(mouse_pos);
                self.is_dragging = false;
                self.pending_drag_node = self.hovered_node;
            }

            if ui.input(|i| i.pointer.primary_down())
                && let Some(down) = self.mouse_down_pos
                && (mouse_pos - down).length() > 3.0
            {
                self.is_dragging = true;
                if self.dragged_node.is_none()
                    && let Some(cand) = self.pending_drag_node
                {
                    self.dragged_node = Some(cand);
                }
            }
        } else {
            self.hovered_node = None;
            self.hovered_edge = None;
        }

        // Handle active node dragging
        if let Some(dragged) = self.dragged_node {
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(mouse_pos) = response.interact_pointer_pos()
            {
                let world_mouse = self.screen_to_world(mouse_pos, rect);
                layout.set_node_pos(dragged, world_mouse, true);
            }

            if response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
                // A user-pinned node stays pinned; any other node is released.
                let keep_pinned = graph
                    .graph
                    .node_weight(dragged)
                    .is_some_and(|n| layout.pinned_positions.contains_key(&n.name));
                if let Some(phys) = layout.nodes.get(&dragged) {
                    layout.set_node_pos(dragged, phys.pos, keep_pinned);
                }
                self.dragged_node = None;
                self.pending_drag_node = None;
                self.mouse_down_pos = None;
            }
        } else if response.dragged() {
            // Pan when dragging empty canvas (no pending node candidate) or using middle/secondary buttons
            if self.pending_drag_node.is_none()
                || response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary)
            {
                self.pan += response.drag_delta();
            }
        }

        if response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
            self.pending_drag_node = None;
            self.mouse_down_pos = None;
        }

        // Handle Clicks (when not dragging)
        if response.double_clicked() && !self.is_dragging {
            if let Some(node_idx) = self.hovered_node {
                self.selected_node = Some(node_idx);
                self.selected_edge = None;
                triggered_action = Some(CanvasAction::NodeDoubleClicked(node_idx));
            }
        } else if response.clicked() && !self.is_dragging {
            if let Some(node_idx) = self.hovered_node {
                self.selected_node = Some(node_idx);
                self.selected_edge = None;
                triggered_action = Some(CanvasAction::NodeClicked(node_idx));
            } else if let Some(edge_idx) = self.hovered_edge {
                self.selected_edge = Some(edge_idx);
                self.selected_node = None;
                triggered_action = Some(CanvasAction::EdgeClicked(edge_idx));
            } else {
                self.selected_node = None;
                self.selected_edge = None;
                triggered_action = Some(CanvasAction::SelectionCleared);
            }
        }

        // Secondary Click (Right click) for context menus
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            if let Some(node_idx) = self.hovered_node {
                self.selected_node = Some(node_idx);
                self.selected_edge = None;
                triggered_action = Some(CanvasAction::NodeSecondaryClicked(node_idx, pos));
            } else if let Some(edge_idx) = self.hovered_edge {
                self.selected_edge = Some(edge_idx);
                self.selected_node = None;
                triggered_action = Some(CanvasAction::EdgeSecondaryClicked(edge_idx, pos));
            } else {
                self.selected_node = None;
                self.selected_edge = None;
                triggered_action = Some(CanvasAction::SelectionCleared);
            }
        }

        // 3. Render Background Grid
        let bg_color = palette.canvas_bg;
        painter.rect_filled(rect, 0.0, bg_color);

        let grid_size = 50.0 * self.zoom;
        if grid_size > 15.0 {
            let grid_color = palette.grid_dot;
            let start_x = (rect.center().x + self.pan.x) % grid_size + rect.min.x;
            let start_y = (rect.center().y + self.pan.y) % grid_size + rect.min.y;

            let mut x = start_x;
            while x < rect.max.x {
                painter.line_segment(
                    [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                    Stroke::new(1.0, grid_color),
                );
                x += grid_size;
            }
            let mut y = start_y;
            while y < rect.max.y {
                painter.line_segment(
                    [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                    Stroke::new(1.0, grid_color),
                );
                y += grid_size;
            }
        }

        // 4a. Initiator arrows below the message edges: straight between the
        // two curved message edges of a reciprocal pair, curved beside the
        // straight edge of a one-way pair.
        for (a, b, initiators) in graph.iter_initiators() {
            let (Some(ia), Some(ib)) = (graph.get_node_index(a), graph.get_node_index(b)) else {
                continue;
            };
            if let Some(v) = visible_nodes
                && (!v.contains(&ia) || !v.contains(&ib))
            {
                continue;
            }
            let (Some(pa), Some(pb)) = (layout.nodes.get(&ia), layout.nodes.get(&ib)) else {
                continue;
            };
            let p1 = self.world_to_screen(pa.pos, rect);
            let p2 = self.world_to_screen(pb.pos, rect);
            let d = p2 - p1;
            let len = d.length();
            let gap = self.edge_gap();
            if len <= 2.0 * gap {
                continue;
            }
            let dir = d / len;
            let normal = Vec2::new(-dir.y, dir.x);
            let start_pt = p1 + dir * gap;
            let end_pt = p2 - dir * gap;
            let reciprocal = graph.graph.contains_edge(ia, ib) && graph.graph.contains_edge(ib, ia);
            let ctrl = start_pt
                + (end_pt - start_pt) * 0.5
                + if reciprocal {
                    Vec2::ZERO
                } else {
                    normal * (18.0 * self.zoom)
                };
            let color = palette.initiator_edge;
            let stroke = Stroke::new(4.0 * self.zoom.clamp(0.6, 2.2), color);
            let head = 12.0 * self.zoom;
            let (head_at_a, head_at_b) = initiator_heads(a, initiators);
            let tip_a = if head_at_a {
                start_pt + (ctrl - start_pt).normalized() * head
            } else {
                start_pt
            };
            let tip_b = if head_at_b {
                end_pt - (end_pt - ctrl).normalized() * head
            } else {
                end_pt
            };
            // One polyline: overlapping segments would darken the translucent stroke.
            let points = (0..=16)
                .map(|step| {
                    let t = step as f32 / 16.0;
                    let it = 1.0 - t;
                    Pos2::new(
                        it * it * tip_a.x + 2.0 * it * t * ctrl.x + t * t * tip_b.x,
                        it * it * tip_a.y + 2.0 * it * t * ctrl.y + t * t * tip_b.y,
                    )
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
            if head_at_a {
                draw_arrow_head(
                    &painter,
                    start_pt,
                    (start_pt - ctrl).normalized(),
                    head,
                    color,
                );
            }
            if head_at_b {
                draw_arrow_head(&painter, end_pt, (end_pt - ctrl).normalized(), head, color);
            }
        }

        // 4. Render Edges
        let node_radius = 26.0 * self.zoom;
        for edge_ref in graph.graph.edge_references() {
            let edge_id = edge_ref.id();
            let source_idx = edge_ref.source();
            let target_idx = edge_ref.target();

            if let Some(v) = visible_nodes
                && (!v.contains(&source_idx) || !v.contains(&target_idx))
            {
                continue;
            }

            let Some(source_phys) = layout.nodes.get(&source_idx) else {
                continue;
            };
            let Some(target_phys) = layout.nodes.get(&target_idx) else {
                continue;
            };

            let p1 = self.world_to_screen(source_phys.pos, rect);
            let p2 = self.world_to_screen(target_phys.pos, rect);

            let d = p2 - p1;
            let len = d.length();
            if len <= 2.0 * self.edge_gap() {
                continue;
            }

            let dir = d / len;
            let normal = Vec2::new(-dir.y, dir.x);

            let is_reciprocal = graph.graph.contains_edge(target_idx, source_idx);
            let curve_offset = if is_reciprocal { 18.0 * self.zoom } else { 0.0 };

            let edge_gap = self.edge_gap();
            let start_pt = p1 + dir * edge_gap;
            let end_pt = p2 - dir * edge_gap;

            let is_selected = self.selected_edge == Some(edge_id);
            let is_hovered = self.hovered_edge == Some(edge_id);

            match edge_ref.weight() {
                EdgeKind::Spawn(_) => {
                    let color = if is_selected {
                        palette.edge_selected
                    } else if is_hovered {
                        palette.spawn_edge_hover
                    } else {
                        palette.spawn_edge
                    };
                    let stroke = Stroke::new(
                        (if is_selected { 2.5 } else { 1.5 }) * self.zoom.clamp(0.5, 2.0),
                        color,
                    );
                    // Dashed and muted to stay distinct from solid message edges;
                    // the arrow points from parent (source) to child (target).
                    let arrow = 8.0 * self.zoom;
                    painter.extend(egui::Shape::dashed_line(
                        &[start_pt, end_pt - dir * arrow],
                        stroke,
                        6.0 * self.zoom,
                        4.0 * self.zoom,
                    ));
                    draw_arrow_head(&painter, end_pt, dir, arrow, color);
                }
                EdgeKind::Message(msg) => {
                    let wc = match &self.edge_counts {
                        Some(map) => map.get(&edge_id).copied().unwrap_or_default(),
                        None => WindowedCount { count: msg.count },
                    };
                    // Edges without messages in the window are dimmed, not hidden,
                    // and stay clickable (hit test is unchanged).
                    let inactive = wc.count == 0;
                    let base_thickness = if inactive {
                        1.0 * self.zoom.clamp(0.6, 2.2)
                    } else {
                        (1.8 + (wc.count as f32).log2().min(5.0)) * self.zoom.clamp(0.6, 2.2)
                    };
                    let thickness = if is_selected {
                        base_thickness + 2.0 * self.zoom
                    } else if is_hovered {
                        base_thickness + 1.0 * self.zoom
                    } else {
                        base_thickness
                    };

                    let color = if is_selected {
                        palette.edge_selected
                    } else if is_hovered {
                        palette.message_edge_hover
                    } else if inactive {
                        palette.message_edge_idle
                    } else {
                        palette.message_edge
                    };
                    let focus = [self.selected_node, self.hovered_node]
                        .iter()
                        .flatten()
                        .any(|n| *n == source_idx || *n == target_idx);
                    let badge = edge_badge_label(wc).filter(|_| show_edge_badge(wc.count, focus));
                    let stroke = Stroke::new(thickness, color);

                    if curve_offset.abs() > 0.1 {
                        let mid = p1 + (p2 - p1) * 0.5;
                        let ctrl_pt = mid + normal * curve_offset;
                        draw_quadratic_bezier(&painter, start_pt, ctrl_pt, end_pt, stroke);
                        let mid_dir = (end_pt - ctrl_pt).normalized();
                        draw_arrow_head(&painter, end_pt, mid_dir, 10.0 * self.zoom, color);

                        // Pill badge with message count
                        if let Some(ref text) = badge {
                            draw_badge(&painter, ctrl_pt, text, self.zoom, palette);
                        }
                    } else {
                        painter.line_segment([start_pt, end_pt], stroke);
                        draw_arrow_head(&painter, end_pt, dir, 10.0 * self.zoom, color);

                        // Pill badge with message count
                        let mid_pt = start_pt + (end_pt - start_pt) * 0.5;
                        if let Some(ref text) = badge {
                            draw_badge(&painter, mid_pt, text, self.zoom, palette);
                        }
                    }
                }
            }
        }

        // 5. Render Nodes
        // Messages in the window per node (in + out), the same counts as the
        // badges, and the nodes whose label is always drawn.
        let mut node_msgs: std::collections::HashMap<NodeIndex, usize> =
            std::collections::HashMap::new();
        for e in graph.graph.edge_references() {
            if let EdgeKind::Message(msg) = e.weight() {
                let n = match &self.edge_counts {
                    Some(map) => map.get(&e.id()).map_or(0, |c| c.count),
                    None => msg.count,
                };
                *node_msgs.entry(e.source()).or_default() += n;
                *node_msgs.entry(e.target()).or_default() += n;
            }
        }
        let mut forced_labels: std::collections::HashSet<NodeIndex> =
            [self.hovered_node, self.selected_node]
                .into_iter()
                .flatten()
                .collect();
        if let Some(sel) = self.selected_node {
            forced_labels.extend(graph.graph.neighbors_undirected(sel));
        }
        let mut labels = Vec::new();
        let mut off_view = Vec::new();
        for (idx, phys) in &layout.nodes {
            if let Some(v) = visible_nodes
                && !v.contains(idx)
            {
                continue;
            }
            let Some(node_data) = graph.graph.node_weight(*idx) else {
                continue;
            };
            let center = self.world_to_screen(phys.pos, rect);

            let is_hovered = self.hovered_node == Some(*idx);
            let is_dragged = self.dragged_node == Some(*idx);
            let is_selected = self.selected_node == Some(*idx);

            let status_color = palette.status_color(&node_data.status);
            let node_fill = palette.node_fill;

            let border_width = if is_dragged || is_hovered || is_selected {
                3.5 * self.zoom
            } else {
                2.2 * self.zoom
            };

            let style = graph.node_style(&node_data.name);

            // Selection ring (outside the CLI and roaming rings)
            if is_selected {
                painter.circle_stroke(
                    center,
                    node_radius + 11.0 * self.zoom,
                    Stroke::new(2.5 * self.zoom, palette.selected_ring),
                );
            }

            // Outer glow if hovered
            if is_hovered && !is_selected {
                painter.circle_filled(
                    center,
                    node_radius + 4.0 * self.zoom,
                    Color32::from_rgba_premultiplied(
                        status_color.r(),
                        status_color.g(),
                        status_color.b(),
                        40,
                    ),
                );
            }

            // Node body & border
            painter.circle_filled(center, node_radius, node_fill);

            // Context window fill rising from the bottom
            if let Some(pct) = style.context_percent {
                let fill_points = context_fill_polygon(center, node_radius, pct);
                if fill_points.len() >= 3 {
                    let c = palette.context_fill_color(pct);
                    painter.add(egui::Shape::convex_polygon(
                        fill_points,
                        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 110),
                        Stroke::NONE,
                    ));
                }
            }

            painter.circle_stroke(center, node_radius, Stroke::new(border_width, status_color));

            // CLI accent ring
            painter.circle_stroke(
                center,
                node_radius + 3.5 * self.zoom,
                Stroke::new(
                    2.5 * self.zoom,
                    palette.cli_color(node_data.tool.as_deref()),
                ),
            );

            // Roaming agents: dashed outer outline
            if style.is_roaming {
                let ring = circle_points(center, node_radius + 7.5 * self.zoom, 48);
                painter.extend(egui::Shape::dashed_line(
                    &ring,
                    Stroke::new(1.5 * self.zoom, palette.roaming_outline),
                    5.0 * self.zoom,
                    3.5 * self.zoom,
                ));
            }

            // User pin: a small light dot on the upper right of the node
            if layout.pinned_positions.contains_key(&node_data.name) {
                painter.circle(
                    center + Vec2::new(0.72, -0.72) * node_radius,
                    4.0 * self.zoom,
                    palette.pin_fill,
                    Stroke::new(1.5 * self.zoom, palette.pin_stroke),
                );
            }

            // Agent name text, drawn after all nodes (5b)
            let galley = painter.layout_no_wrap(
                node_data.name.clone(),
                FontId::proportional(label_font_size(self.zoom)),
                palette.node_text_primary,
            );
            let label = label_rect(center, galley.size(), self.zoom);
            labels.push((
                (
                    label,
                    node_msgs.get(idx).copied().unwrap_or(0),
                    forced_labels.contains(idx),
                ),
                galley,
            ));

            if let Some(marker) = edge_marker(rect, center) {
                off_view.push((marker, status_color));
            }
        }

        // 5b. Node labels on a canvas-colored background; a label that would
        // overlap a label of a busier node is hidden (see `visible_labels`).
        let items: Vec<(Rect, usize, bool)> = labels.iter().map(|(item, _)| *item).collect();
        for ((item, galley), shown) in labels.into_iter().zip(visible_labels(&items)) {
            if shown {
                painter.rect_filled(item.0, 3.0 * self.zoom, bg_color);
                painter.galley(
                    item.0.min + LABEL_PAD * self.zoom,
                    galley,
                    palette.node_text_primary,
                );
            }
        }

        // 6. Markers on the canvas edge for visible nodes outside the view
        for ((tip, dir), color) in off_view {
            let normal = Vec2::new(-dir.y, dir.x);
            let base = tip - dir * 10.0;
            painter.add(egui::Shape::convex_polygon(
                vec![tip, base + normal * 6.0, base - normal * 6.0],
                color,
                Stroke::new(1.0, palette.canvas_bg),
            ));
        }

        (response, triggered_action)
    }
}

/// Node radius plus the outermost decoration (selection ring), in world units.
const NODE_EXTENT: f32 = 26.0 + 11.0;

/// Font size of a node label at `zoom` (as drawn by `CanvasView::show`).
fn label_font_size(zoom: f32) -> f32 {
    (12.0 * zoom).clamp(8.0, 18.0)
}

/// Zoom and pan that fit all `items` (world position, label width at a
/// 12 px font) with their decorations and labels inside `rect` minus a 5 %
/// margin per side: the largest zoom in `ZOOM_MIN..=max_zoom` that fits
/// (`ZOOM_MIN` when none does), the content centered. `None` without items.
pub fn fit_view(items: &[(Pos2, f32)], rect: Rect, max_zoom: f32) -> Option<(f32, Vec2)> {
    if items.is_empty() {
        return None;
    }
    let avail = rect.size() * 0.9;
    // Screen bounding box of the content relative to rect.center() + pan.
    let bbox = |z: f32| {
        let font = label_font_size(z);
        let mut b = Rect::NOTHING;
        for &(pos, label_w12) in items {
            let c = (pos.to_vec2() * z).to_pos2();
            b = b.union(Rect::from_center_size(
                c,
                Vec2::splat(NODE_EXTENT * z * 2.0),
            ));
            // Label below the node; its height approximated as 1.2 × font.
            b = b.union(label_rect(
                c,
                Vec2::new(label_w12 * font / 12.0, font * 1.2),
                z,
            ));
        }
        b
    };
    let fits = |z: f32| {
        let s = bbox(z).size();
        s.x <= avail.x && s.y <= avail.y
    };
    let max_zoom = max_zoom.max(ZOOM_MIN);
    let zoom = if fits(max_zoom) {
        max_zoom
    } else if !fits(ZOOM_MIN) {
        ZOOM_MIN
    } else {
        let (mut lo, mut hi) = (ZOOM_MIN, max_zoom);
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    };
    Some((zoom, -bbox(zoom).center().to_vec2()))
}

/// Off-view marker of a node at `screen_pos`: `None` while the node center is
/// inside `rect`, otherwise the point where the ray from the rect center to
/// the node meets `rect` shrunk by 8 px, and the unit direction to the node.
pub fn edge_marker(rect: Rect, screen_pos: Pos2) -> Option<(Pos2, Vec2)> {
    if rect.contains(screen_pos) {
        return None;
    }
    let inner = rect.shrink(8.0);
    let c = inner.center();
    let d = screen_pos - c;
    let half = inner.size() / 2.0;
    let t = [(d.x, half.x), (d.y, half.y)]
        .iter()
        .filter(|(dv, _)| dv.abs() > 1e-6)
        .map(|(dv, h)| h / dv.abs())
        .fold(f32::INFINITY, f32::min);
    if !t.is_finite() {
        return None;
    }
    Some((c + d * t.min(1.0), d.normalized()))
}

/// Badge text of a message edge: `None` when the window holds no message.
pub fn edge_badge_label(wc: WindowedCount) -> Option<String> {
    match wc.count {
        0 => None,
        1 => Some("1 msg".to_string()),
        n => Some(format!("{n} msgs")),
    }
}

/// Whether a message edge with `count` messages in the window shows its
/// badge: from 2 messages on, or always on an edge of the selected or
/// hovered node (`focus_edge`).
pub fn show_edge_badge(count: usize, focus_edge: bool) -> bool {
    count >= 2 || focus_edge
}

/// Screen radius of a node with all its rings at `zoom`.
pub fn node_extent(zoom: f32) -> f32 {
    NODE_EXTENT * zoom
}

/// Top-left corner of a hover card of `size` for a node at `node` (screen)
/// with `radius`: the first of right, left, below and above the node (8 px
/// gap, centered on the node and shifted along the free axis to stay in
/// `canvas`) that lies inside `canvas` without overlapping the node; none
/// fits: the right position clamped into `canvas`.
pub fn tooltip_placement(node: Pos2, radius: f32, size: Vec2, canvas: Rect) -> Pos2 {
    let gap = 8.0;
    let clamp_x = |x: f32| x.clamp(canvas.min.x, (canvas.max.x - size.x).max(canvas.min.x));
    let clamp_y = |y: f32| y.clamp(canvas.min.y, (canvas.max.y - size.y).max(canvas.min.y));
    let mid_x = clamp_x(node.x - size.x / 2.0);
    let mid_y = clamp_y(node.y - size.y / 2.0);
    let right = Pos2::new(node.x + radius + gap, mid_y);
    let candidates = [
        right,
        Pos2::new(node.x - radius - gap - size.x, mid_y),
        Pos2::new(mid_x, node.y + radius + gap),
        Pos2::new(mid_x, node.y - radius - gap - size.y),
    ];
    let node_rect = Rect::from_center_size(node, Vec2::splat(radius * 2.0));
    candidates
        .into_iter()
        .find(|p| {
            let r = Rect::from_min_size(*p, size);
            canvas.contains_rect(r) && !r.intersects(node_rect)
        })
        .unwrap_or(Pos2::new(clamp_x(right.x), right.y))
}

/// Gap between the outermost node decoration and the label, in world units.
const LABEL_GAP: f32 = 2.0;
/// Label background padding (x, y), in world units.
const LABEL_PAD: Vec2 = Vec2::new(3.0, 1.0);

/// Screen rect of a node label (background included) for a text of
/// `text_size` at `zoom`: centered below the node's outermost decoration
/// (the selection ring), so the label never covers the node's rings.
pub fn label_rect(center: Pos2, text_size: Vec2, zoom: f32) -> Rect {
    let size = text_size + LABEL_PAD * 2.0 * zoom;
    let top = center.y + (NODE_EXTENT + LABEL_GAP) * zoom;
    Rect::from_min_size(Pos2::new(center.x - size.x / 2.0, top), size)
}

/// Which labels are drawn, per item `(rect, messages in window, forced)`.
/// Forced labels (hovered, selected, neighbours of the selected node) are
/// always drawn; the others in descending message count (ties in input
/// order), each only when it does not overlap an already drawn label.
pub fn visible_labels(items: &[(Rect, usize, bool)]) -> Vec<bool> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (!items[i].2, std::cmp::Reverse(items[i].1)));
    let mut shown = vec![false; items.len()];
    let mut placed: Vec<Rect> = Vec::new();
    for i in order {
        let (rect, _, forced) = items[i];
        if forced || !placed.iter().any(|p| p.intersects(rect)) {
            shown[i] = true;
            placed.push(rect);
        }
    }
    shown
}

/// Closed polyline around a circle (first point repeated at the end).
pub(crate) fn circle_points(center: Pos2, radius: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / steps as f32;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// Convex polygon of the circle segment filled from the bottom up to
/// `percent` (0–100) of the diameter. Empty for 0 % or less.
pub fn context_fill_polygon(center: Pos2, radius: f32, percent: f32) -> Vec<Pos2> {
    let p = (percent / 100.0).clamp(0.0, 1.0);
    if p <= 0.0 {
        return Vec::new();
    }
    // Screen y grows downwards: the segment holds the arc angles with
    // sin(a) >= 1 - 2p, i.e. a in [start, PI - start].
    let start = (1.0 - 2.0 * p).asin();
    let end = std::f32::consts::PI - start;
    let steps = 32;
    (0..=steps)
        .map(|i| {
            let a = start + (end - start) * i as f32 / steps as f32;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// Arrow heads of the initiator arrow of pair `a`–`b`: `(at a, at b)`. The
/// arrow points away from each side that started a segment.
pub fn initiator_heads(a: &str, initiators: &[(String, usize)]) -> (bool, bool) {
    let by_a = initiators.iter().any(|(n, _)| n == a);
    let by_b = initiators.iter().any(|(n, _)| n != a);
    (by_b, by_a)
}

/// Pair initiators as shown in the edge detail and tooltip, e.g.
/// `alice (3), bob (1)`; `None` without an initiator.
pub fn format_initiators(initiators: &[(String, usize)]) -> Option<String> {
    (!initiators.is_empty()).then(|| {
        initiators
            .iter()
            .map(|(n, c)| format!("{n} ({c})"))
            .collect::<Vec<_>>()
            .join(", ")
    })
}

fn draw_arrow_head(painter: &egui::Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let normal = Vec2::new(-dir.y, dir.x);
    let p1 = tip - dir * size + normal * (size * 0.5);
    let p2 = tip - dir * size - normal * (size * 0.5);
    painter.add(egui::Shape::convex_polygon(
        vec![tip, p1, p2],
        color,
        Stroke::NONE,
    ));
}

fn draw_quadratic_bezier(painter: &egui::Painter, p0: Pos2, p1: Pos2, p2: Pos2, stroke: Stroke) {
    let steps = 16;
    let mut prev = p0;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let it = 1.0 - t;
        let pt = Pos2::new(
            it * it * p0.x + 2.0 * it * t * p1.x + t * t * p2.x,
            it * it * p0.y + 2.0 * it * t * p1.y + t * t * p2.y,
        );
        painter.line_segment([prev, pt], stroke);
        prev = pt;
    }
}

fn draw_badge(
    painter: &egui::Painter,
    center: Pos2,
    text: &str,
    zoom: f32,
    palette: &ThemePalette,
) {
    let font_size = (10.0 * zoom).clamp(7.0, 14.0);
    let galley = painter.layout_no_wrap(
        text.to_string(),
        FontId::proportional(font_size),
        palette.badge_text,
    );
    let badge_rect =
        Rect::from_center_size(center, galley.size() + Vec2::new(8.0 * zoom, 4.0 * zoom));

    painter.rect_filled(badge_rect, 4.0 * zoom, palette.badge_fill);
    painter.rect_stroke(
        badge_rect,
        4.0 * zoom,
        Stroke::new(1.0, palette.badge_stroke),
        egui::StrokeKind::Outside,
    );
    painter.galley(
        badge_rect.min + Vec2::new(4.0 * zoom, 2.0 * zoom),
        galley,
        palette.badge_text,
    );
}

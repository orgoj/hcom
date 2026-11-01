use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus, EdgeKind};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    NodeClicked(NodeIndex),
    EdgeClicked(EdgeIndex),
    NodeSecondaryClicked(NodeIndex, Pos2),
    EdgeSecondaryClicked(EdgeIndex, Pos2),
    SelectionCleared,
}

pub struct CanvasView {
    pub pan: Vec2,
    pub zoom: f32,
    pub dragged_node: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,
    pub hovered_edge: Option<EdgeIndex>,
    pub selected_node: Option<NodeIndex>,
    pub selected_edge: Option<EdgeIndex>,
    pub mouse_down_pos: Option<Pos2>,
    pub is_dragging: bool,
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
            hovered_node: None,
            hovered_edge: None,
            selected_node: None,
            selected_edge: None,
            mouse_down_pos: None,
            is_dragging: false,
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
        match status {
            AgentStatus::Active => Color32::from_rgb(46, 204, 113), // Green
            AgentStatus::Listening => Color32::from_rgb(52, 152, 219), // Blue
            AgentStatus::Blocked => Color32::from_rgb(231, 76, 60), // Red/Orange
            AgentStatus::Stopped => Color32::from_rgb(127, 140, 141), // Gray
        }
    }

    /// Accent ring color of a node by its CLI tool.
    pub fn cli_color(tool: Option<&str>) -> Color32 {
        match tool.map(|t| t.to_ascii_lowercase()).as_deref() {
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
        }
    }

    /// Context fill color: green below 70 %, amber 70–85 %, red above 85 %.
    pub fn context_fill_color(percent: f32) -> Color32 {
        if percent > 85.0 {
            Color32::from_rgb(231, 76, 60)
        } else if percent >= 70.0 {
            Color32::from_rgb(243, 156, 18)
        } else {
            Color32::from_rgb(46, 204, 113)
        }
    }

    /// Distance from a node center where edges start and end: outside the
    /// CLI (+3.5), roaming (+7.5) and selection (+11) rings, so arrow heads
    /// are never covered by node decorations drawn after the edges.
    fn edge_gap(&self) -> f32 {
        (26.0 + 13.0) * self.zoom
    }

    pub fn reset_view(&mut self) {
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
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
            let new_zoom = (self.zoom * zoom_factor).clamp(0.15, 4.0);
            self.pan = hover_pos - rect.center() - world_under_mouse.to_vec2() * new_zoom;
            self.zoom = new_zoom;
        }

        // 2. Hit testing & Click vs. Drag handling
        let hit_radius_world = 26.0;

        if let Some(mouse_pos) = response.interact_pointer_pos() {
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
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.mouse_down_pos = Some(mouse_pos);
                self.is_dragging = false;

                if let Some(hit) = self.hovered_node {
                    self.dragged_node = Some(hit);
                    layout.set_node_pos(hit, world_mouse, true);
                }
            }

            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(down) = self.mouse_down_pos
                && (mouse_pos - down).length() > 3.0
            {
                self.is_dragging = true;
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
                if let Some(phys) = layout.nodes.get(&dragged) {
                    layout.set_node_pos(dragged, phys.pos, false);
                }
                self.dragged_node = None;
            }
        } else if response.dragged() {
            // Pan when dragging empty canvas or using middle/secondary buttons
            self.pan += response.drag_delta();
        }

        // Handle Clicks (when not dragging)
        if response.clicked() && !self.is_dragging {
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
            }
        }

        // 3. Render Background Grid
        let bg_color = Color32::from_rgb(18, 19, 24);
        painter.rect_filled(rect, 0.0, bg_color);

        let grid_size = 50.0 * self.zoom;
        if grid_size > 15.0 {
            let grid_color = Color32::from_rgba_premultiplied(40, 42, 54, 50);
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
                        Color32::from_rgb(241, 196, 15)
                    } else if is_hovered {
                        Color32::from_rgb(170, 185, 210)
                    } else {
                        Color32::from_rgb(110, 120, 140)
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
                    let base_thickness =
                        (1.8 + (msg.count as f32).log2().min(5.0)) * self.zoom.clamp(0.6, 2.2);
                    let thickness = if is_selected {
                        base_thickness + 2.0 * self.zoom
                    } else if is_hovered {
                        base_thickness + 1.0 * self.zoom
                    } else {
                        base_thickness
                    };

                    let color = if is_selected {
                        Color32::from_rgb(241, 196, 15)
                    } else if is_hovered {
                        Color32::from_rgb(210, 150, 235)
                    } else {
                        Color32::from_rgb(165, 105, 189)
                    };
                    let stroke = Stroke::new(thickness, color);

                    if curve_offset.abs() > 0.1 {
                        let mid = p1 + (p2 - p1) * 0.5;
                        let ctrl_pt = mid + normal * curve_offset;
                        draw_quadratic_bezier(&painter, start_pt, ctrl_pt, end_pt, stroke);
                        let mid_dir = (end_pt - ctrl_pt).normalized();
                        draw_arrow_head(&painter, end_pt, mid_dir, 10.0 * self.zoom, color);

                        // Pill badge with message count
                        draw_badge(&painter, ctrl_pt, &format!("{} msgs", msg.count), self.zoom);
                    } else {
                        painter.line_segment([start_pt, end_pt], stroke);
                        draw_arrow_head(&painter, end_pt, dir, 10.0 * self.zoom, color);

                        // Pill badge with message count
                        let mid_pt = start_pt + (end_pt - start_pt) * 0.5;
                        draw_badge(&painter, mid_pt, &format!("{} msgs", msg.count), self.zoom);
                    }
                }
            }
        }

        // 5. Render Nodes
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

            let status_color = Self::status_color(&node_data.status);
            let node_fill = Color32::from_rgb(28, 30, 38);

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
                    Stroke::new(2.5 * self.zoom, Color32::from_rgb(241, 196, 15)),
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
                    let c = Self::context_fill_color(pct);
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
                Stroke::new(2.5 * self.zoom, Self::cli_color(node_data.tool.as_deref())),
            );

            // Roaming agents: dashed outer outline
            if style.is_roaming {
                let ring = circle_points(center, node_radius + 7.5 * self.zoom, 48);
                painter.extend(egui::Shape::dashed_line(
                    &ring,
                    Stroke::new(1.5 * self.zoom, Color32::from_rgb(200, 205, 220)),
                    5.0 * self.zoom,
                    3.5 * self.zoom,
                ));
            }

            // Agent name text
            let font_size = (12.0 * self.zoom).clamp(8.0, 18.0);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                &node_data.name,
                FontId::proportional(font_size),
                Color32::WHITE,
            );
        }

        (response, triggered_action)
    }
}

/// Closed polyline around a circle (first point repeated at the end).
fn circle_points(center: Pos2, radius: f32, steps: usize) -> Vec<Pos2> {
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

fn draw_badge(painter: &egui::Painter, center: Pos2, text: &str, zoom: f32) {
    let font_size = (10.0 * zoom).clamp(7.0, 14.0);
    let galley = painter.layout_no_wrap(
        text.to_string(),
        FontId::proportional(font_size),
        Color32::from_rgb(220, 220, 230),
    );
    let badge_rect =
        Rect::from_center_size(center, galley.size() + Vec2::new(8.0 * zoom, 4.0 * zoom));

    painter.rect_filled(badge_rect, 4.0 * zoom, Color32::from_rgb(35, 38, 50));
    painter.rect_stroke(
        badge_rect,
        4.0 * zoom,
        Stroke::new(1.0, Color32::from_rgb(70, 75, 95)),
        egui::StrokeKind::Outside,
    );
    painter.galley(
        badge_rect.min + Vec2::new(4.0 * zoom, 2.0 * zoom),
        galley,
        Color32::from_rgb(220, 220, 230),
    );
}

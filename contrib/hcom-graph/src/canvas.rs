use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus, EdgeKind};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

pub struct CanvasView {
    pub pan: Vec2,
    pub zoom: f32,
    pub dragged_node: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasView {
    pub fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            dragged_node: None,
            hovered_node: None,
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

    pub fn reset_view(&mut self) {
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        graph: &AgentGraph,
        layout: &mut GraphLayout,
    ) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

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

        // 2. Hit testing and Drag & Drop
        let node_radius = 26.0 * self.zoom;
        let hit_radius_world = 26.0;

        if let Some(mouse_pos) = response.interact_pointer_pos() {
            let world_mouse = self.screen_to_world(mouse_pos, rect);
            self.hovered_node = layout.hit_test(world_mouse, hit_radius_world);

            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(hit) = self.hovered_node
            {
                self.dragged_node = Some(hit);
                layout.set_node_pos(hit, world_mouse, true);
            }
        } else {
            self.hovered_node = None;
        }

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
        for edge_ref in graph.graph.edge_references() {
            let source_idx = edge_ref.source();
            let target_idx = edge_ref.target();

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
            if len <= node_radius {
                continue;
            }

            let dir = d / len;
            let normal = Vec2::new(-dir.y, dir.x);

            // Check if reciprocal edge exists between these two nodes
            let is_reciprocal = graph.graph.contains_edge(target_idx, source_idx);
            let curve_offset = if is_reciprocal { 18.0 * self.zoom } else { 0.0 };

            let start_pt = p1 + dir * node_radius;
            let end_pt = p2 - dir * node_radius;

            match edge_ref.weight() {
                EdgeKind::Spawn(_) => {
                    let stroke = Stroke::new(
                        1.5 * self.zoom.clamp(0.5, 2.0),
                        Color32::from_rgb(110, 120, 140),
                    );
                    painter.line_segment([start_pt, end_pt], stroke);
                    draw_arrow_head(&painter, end_pt, dir, 8.0 * self.zoom, stroke.color);
                }
                EdgeKind::Message(msg) => {
                    let thickness =
                        (1.8 + (msg.count as f32).log2().min(5.0)) * self.zoom.clamp(0.6, 2.2);
                    let color = Color32::from_rgb(165, 105, 189);
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
            let Some(node_data) = graph.graph.node_weight(*idx) else {
                continue;
            };
            let center = self.world_to_screen(phys.pos, rect);

            let is_hovered = self.hovered_node == Some(*idx);
            let is_dragged = self.dragged_node == Some(*idx);

            let status_color = Self::status_color(&node_data.status);
            let node_fill = Color32::from_rgb(28, 30, 38);

            let border_width = if is_dragged || is_hovered {
                3.5 * self.zoom
            } else {
                2.2 * self.zoom
            };

            // Outer glow if hovered
            if is_hovered {
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
            painter.circle_stroke(center, node_radius, Stroke::new(border_width, status_color));

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

        response
    }
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

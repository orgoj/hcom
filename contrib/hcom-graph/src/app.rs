use crate::canvas::CanvasView;
use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus};
use crate::sync::BackgroundSync;
use egui::{Color32, RichText};

pub struct HcomGraphApp {
    pub graph: AgentGraph,
    pub layout: GraphLayout,
    pub canvas: CanvasView,
    pub sync: Option<BackgroundSync>,
}

impl HcomGraphApp {
    pub fn new(cc: &eframe::CreationContext<'_>, sync: Option<BackgroundSync>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let mut graph = AgentGraph::new();
        if let Some(ref s) = sync
            && let Some(initial) = s.current_graph()
        {
            graph = initial;
        }

        let mut layout = GraphLayout::new();
        layout.sync_nodes(&graph);

        Self {
            graph,
            layout,
            canvas: CanvasView::new(),
            sync,
        }
    }
}

impl eframe::App for HcomGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. Ingest non-blocking updates from BackgroundSync
        if let Some(ref sync) = self.sync
            && let Some(updated) = sync.try_get_latest()
        {
            self.graph = updated;
            self.layout.sync_nodes(&self.graph);
            self.layout.wake_up();
            ctx.request_repaint();
        }

        // 2. Physics simulation step
        let dt = ctx.input(|i| i.stable_dt);
        let motion = self.layout.step(&self.graph, dt);
        if motion > 0.0 || self.canvas.dragged_node.is_some() {
            ctx.request_repaint();
        }

        // 3. Top Status Panel
        egui::Panel::top("top_status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("hcom graph").strong());
                ui.separator();

                let mut active_count = 0;
                let mut listening_count = 0;
                let mut blocked_count = 0;
                let mut stopped_count = 0;

                for node in self.graph.graph.node_weights() {
                    match node.status {
                        AgentStatus::Active => active_count += 1,
                        AgentStatus::Listening => listening_count += 1,
                        AgentStatus::Blocked => blocked_count += 1,
                        AgentStatus::Stopped => stopped_count += 1,
                    }
                }

                let total = self.graph.graph.node_count();
                ui.label(format!("Agents: {}", total));

                ui.colored_label(
                    Color32::from_rgb(46, 204, 113),
                    format!("● {} active", active_count),
                );
                ui.colored_label(
                    Color32::from_rgb(52, 152, 219),
                    format!("● {} listening", listening_count),
                );
                ui.colored_label(
                    Color32::from_rgb(231, 76, 60),
                    format!("● {} blocked", blocked_count),
                );
                ui.colored_label(
                    Color32::from_rgb(127, 140, 141),
                    format!("● {} stopped", stopped_count),
                );

                ui.separator();
                let edge_count = self.graph.graph.edge_count();
                ui.label(format!("Edges: {}", edge_count));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset View").clicked() {
                        self.canvas.reset_view();
                    }
                    ui.label(format!("Zoom: {:.0}%", self.canvas.zoom * 100.0));
                });
            });
        });

        // 4. Central 2D Canvas Panel
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.show(ui, &self.graph, &mut self.layout);
        });
    }
}

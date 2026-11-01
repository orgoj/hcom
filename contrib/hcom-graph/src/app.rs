use crate::action::{ActionRequest, ActionResult, ActionWorkerHandle, SystemActionRunner};
use crate::canvas::{CanvasAction, CanvasView};
use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus, EdgeKind};
use crate::sync::BackgroundSync;
use egui::{Color32, RichText};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};

#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuTarget {
    Node(NodeIndex),
    Edge(EdgeIndex),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub target: ContextMenuTarget,
    pub screen_pos: egui::Pos2,
}

pub struct HcomGraphApp {
    pub graph: AgentGraph,
    pub layout: GraphLayout,
    pub canvas: CanvasView,
    pub sync: Option<BackgroundSync>,
    pub action_worker: Option<ActionWorkerHandle>,
    pub pending_action: Option<String>,
    pub action_error: Option<String>,
    pub confirm_kill_target: Option<String>,
    pub inject_input: String,
    pub loaded_transcript: Option<(String, String)>,
    pub context_menu: Option<ContextMenuState>,
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

        let action_worker = Some(ActionWorkerHandle::new(SystemActionRunner::new()));

        Self {
            graph,
            layout,
            canvas: CanvasView::new(),
            sync,
            action_worker,
            pending_action: None,
            action_error: None,
            confirm_kill_target: None,
            inject_input: String::new(),
            loaded_transcript: None,
            context_menu: None,
        }
    }

    pub fn dispatch_action(&mut self, req: ActionRequest) {
        if let Some(ref worker) = self.action_worker {
            match &req {
                ActionRequest::Attach { agent } => {
                    self.pending_action = Some(format!("Attaching to {}...", agent));
                }
                ActionRequest::Inject { agent, .. } => {
                    self.pending_action = Some(format!("Injecting to {}...", agent));
                }
                ActionRequest::Transcript { agent, .. } => {
                    self.pending_action = Some(format!("Loading transcript for {}...", agent));
                }
                ActionRequest::Kill { agent } => {
                    self.pending_action = Some(format!("Killing {}...", agent));
                }
                ActionRequest::Shutdown => {}
            }
            if let Err(e) = worker.try_send(req) {
                self.pending_action = None;
                self.action_error = Some(e);
            }
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

        // 2. Poll background action results
        if let Some(ref worker) = self.action_worker {
            while let Some(res) = worker.try_recv() {
                self.pending_action = None;
                match res {
                    ActionResult::AttachFinished { agent, result } => {
                        if let Err(e) = result {
                            self.action_error =
                                Some(format!("Attach to '{}' failed: {}", agent, e));
                        }
                    }
                    ActionResult::InjectFinished { agent, result } => match result {
                        Ok(()) => {
                            self.inject_input.clear();
                        }
                        Err(e) => {
                            self.action_error =
                                Some(format!("Inject to '{}' failed: {}", agent, e));
                        }
                    },
                    ActionResult::TranscriptLoaded { agent, content } => match content {
                        Ok(text) => {
                            self.loaded_transcript = Some((agent, text));
                        }
                        Err(e) => {
                            self.action_error =
                                Some(format!("Transcript for '{}' failed: {}", agent, e));
                        }
                    },
                    ActionResult::KillFinished { agent, result } => {
                        if let Err(e) = result {
                            self.action_error = Some(format!("Kill '{}' failed: {}", agent, e));
                        }
                    }
                }
                ctx.request_repaint();
            }
        }

        // 3. Physics simulation step
        let dt = ctx.input(|i| i.stable_dt);
        let motion = self.layout.step(&self.graph, dt);
        if motion > 0.0 || self.canvas.dragged_node.is_some() || self.pending_action.is_some() {
            ctx.request_repaint();
        }

        // 4. Top Status Panel
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

                if let Some(ref pending) = self.pending_action {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(241, 196, 15), format!("⏳ {}", pending));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset View").clicked() {
                        self.canvas.reset_view();
                    }
                    ui.label(format!("Zoom: {:.0}%", self.canvas.zoom * 100.0));
                });
            });
        });

        // 5. Right Side Panel for Agent Details & Edge Message History
        let has_selection =
            self.canvas.selected_node.is_some() || self.canvas.selected_edge.is_some();
        if has_selection {
            egui::Panel::right("detail_side_panel").show(ui, |ui| {
                ui.set_min_width(320.0);
                ui.add_space(4.0);

                // Error banner
                let mut dismiss_err = false;
                if let Some(ref err) = self.action_error {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(231, 76, 60), format!("⚠ {}", err));
                        if ui.small_button("✖").clicked() {
                            dismiss_err = true;
                        }
                    });
                    ui.separator();
                }
                if dismiss_err {
                    self.action_error = None;
                }

                // Render Selected Node Details (AC #1, #3, #4)
                if let Some(node_idx) = self.canvas.selected_node {
                    if let Some(node) = self.graph.graph.node_weight(node_idx) {
                        let agent_name = node.name.clone();
                        let status_col = CanvasView::status_color(&node.status);

                        ui.horizontal(|ui| {
                            ui.heading(RichText::new(&node.name).strong());
                            ui.colored_label(status_col, format!("{:?}", node.status));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("✖").clicked() {
                                        self.canvas.selected_node = None;
                                    }
                                },
                            );
                        });
                        ui.separator();

                        // Metadata rows
                        egui::Grid::new("agent_meta_grid")
                            .num_columns(2)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Status Detail:").weak());
                                ui.label(
                                    node.status_context.as_deref().unwrap_or("none").to_string(),
                                );
                                ui.end_row();

                                ui.label(RichText::new("Status Age:").weak());
                                ui.label(format!("{}s", node.status_age_seconds));
                                ui.end_row();

                                ui.label(RichText::new("Unread Count:").weak());
                                ui.label(format!("{}", node.unread_count));
                                ui.end_row();

                                ui.label(RichText::new("CLI Tool:").weak());
                                ui.label(node.tool.as_deref().unwrap_or("-").to_string());
                                ui.end_row();

                                ui.label(RichText::new("Directory:").weak());
                                ui.label(node.directory.as_deref().unwrap_or("-").to_string());
                                ui.end_row();

                                ui.label(RichText::new("Parent:").weak());
                                ui.label(node.parent_name.as_deref().unwrap_or("none").to_string());
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.separator();

                        // Quick Action Buttons (AC #4)
                        ui.label(RichText::new("Quick Actions:").strong());
                        ui.horizontal(|ui| {
                            if ui.button("Attach (Focus)").clicked() {
                                self.dispatch_action(ActionRequest::Attach {
                                    agent: agent_name.clone(),
                                });
                            }
                            if ui.button("Transcript").clicked() {
                                self.dispatch_action(ActionRequest::Transcript {
                                    agent: agent_name.clone(),
                                    last_lines: 30,
                                });
                            }
                            if ui
                                .button(RichText::new("Kill").color(Color32::from_rgb(231, 76, 60)))
                                .clicked()
                            {
                                self.confirm_kill_target = Some(agent_name.clone());
                            }
                        });

                        // Confirmation Modal/Bar for Kill
                        let mut confirm_action = None;
                        let mut cancel_confirm = false;
                        if let Some(ref target) = self.confirm_kill_target {
                            ui.add_space(6.0);
                            ui.group(|ui| {
                                ui.colored_label(
                                    Color32::from_rgb(231, 76, 60),
                                    format!("Confirm kill agent '{}'?", target),
                                );
                                ui.horizontal(|ui| {
                                    if ui.button("Confirm Kill").clicked() {
                                        confirm_action = Some(target.clone());
                                    }
                                    if ui.button("Cancel").clicked() {
                                        cancel_confirm = true;
                                    }
                                });
                            });
                        }
                        if let Some(target) = confirm_action {
                            if let Some(agent_node) = self.graph.get_agent(&target) {
                                if agent_node.status == AgentStatus::Stopped {
                                    self.action_error =
                                        Some(format!("Agent '{}' is already stopped", target));
                                } else {
                                    self.dispatch_action(ActionRequest::Kill { agent: target });
                                }
                            } else {
                                self.action_error =
                                    Some(format!("Agent '{}' does not exist", target));
                            }
                            self.confirm_kill_target = None;
                        } else if cancel_confirm {
                            self.confirm_kill_target = None;
                        }

                        // Inject field (AC #4)
                        ui.add_space(6.0);
                        ui.label(RichText::new("Inject Prompt/Command:").strong());
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.inject_input);
                            if ui.button("Inject").clicked() && !self.inject_input.trim().is_empty()
                            {
                                self.dispatch_action(ActionRequest::Inject {
                                    agent: agent_name.clone(),
                                    text: self.inject_input.clone(),
                                });
                            }
                        });

                        // Loaded Transcript View
                        if let Some((ref tr_agent, ref text)) = self.loaded_transcript
                            && tr_agent == &agent_name
                        {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.label(RichText::new("Recent Transcript:").strong());
                            egui::ScrollArea::vertical()
                                .max_height(250.0)
                                .show(ui, |ui| {
                                    ui.monospace(text);
                                });
                        }
                    }
                } else if let Some(edge_idx) = self.canvas.selected_edge {
                    // Render Selected Edge Message History (AC #2)
                    if let Some(edge_weight) = self.graph.graph.edge_weight(edge_idx) {
                        match edge_weight {
                            EdgeKind::Message(msg) => {
                                let from = msg.from.clone();
                                let to = msg.to.clone();

                                ui.horizontal(|ui| {
                                    ui.heading(
                                        RichText::new(format!("{} ⇄ {}", from, to)).strong(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("✖").clicked() {
                                                self.canvas.selected_edge = None;
                                            }
                                        },
                                    );
                                });
                                ui.separator();

                                ui.label(format!("Total Messages (directed): {}", msg.count));
                                ui.label(format!(
                                    "Last Active: {}",
                                    msg.last_timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                                ));
                                if let Some(ref intent) = msg.last_intent {
                                    ui.label(format!("Last Intent: {}", intent));
                                }
                                ui.label(format!("Last Preview: {}", msg.last_preview));
                                ui.separator();

                                ui.label(RichText::new("Conversation History (last 50):").strong());
                                let history = self.graph.get_conversation_history(&from, &to);
                                if history.is_empty() {
                                    ui.label("No recent stored messages.");
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height(400.0)
                                        .show(ui, |ui| {
                                            for m in &history {
                                                ui.group(|ui| {
                                                    ui.horizontal(|ui| {
                                                        let from_color = if m.from == from {
                                                            Color32::from_rgb(52, 152, 219)
                                                        } else {
                                                            Color32::from_rgb(46, 204, 113)
                                                        };
                                                        ui.colored_label(
                                                            from_color,
                                                            RichText::new(&m.from).strong(),
                                                        );
                                                        ui.label("→");
                                                        ui.label(&m.to);
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                ui.weak(
                                                                    m.ts.format("%H:%M:%S")
                                                                        .to_string(),
                                                                );
                                                            },
                                                        );
                                                    });
                                                    if let Some(ref intent) = m.intent {
                                                        ui.weak(format!("intent: {}", intent));
                                                    }
                                                    ui.monospace(&m.text);
                                                });
                                                ui.add_space(2.0);
                                            }
                                        });
                                }
                            }
                            EdgeKind::Spawn(spawn) => {
                                ui.horizontal(|ui| {
                                    ui.heading(RichText::new("Spawn Relation").strong());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("✖").clicked() {
                                                self.canvas.selected_edge = None;
                                            }
                                        },
                                    );
                                });
                                ui.separator();
                                ui.label(format!("Parent: {}", spawn.parent));
                                ui.label(format!("Child: {}", spawn.child));
                            }
                        }
                    }
                }
            });
        }

        // 6. Central 2D Canvas Panel
        egui::CentralPanel::default().show(ui, |ui| {
            let (_response, action) = self.canvas.show(ui, &self.graph, &mut self.layout);

            if let Some(act) = action {
                match act {
                    CanvasAction::NodeClicked(idx) => {
                        // AC #1: Clicking node triggers attach/focus in terminal
                        if let Some(node) = self.graph.graph.node_weight(idx) {
                            let name = node.name.clone();
                            self.dispatch_action(ActionRequest::Attach { agent: name });
                        }
                    }
                    CanvasAction::EdgeClicked(_) => {
                        // Handled by CanvasView selection; side panel automatically shows message history (AC #2)
                    }
                    CanvasAction::NodeSecondaryClicked(idx, pos) => {
                        // AC #5: Right click on node opens context menu
                        self.context_menu = Some(ContextMenuState {
                            target: ContextMenuTarget::Node(idx),
                            screen_pos: pos,
                        });
                    }
                    CanvasAction::EdgeSecondaryClicked(idx, pos) => {
                        // AC #5: Right click on edge opens context menu
                        self.context_menu = Some(ContextMenuState {
                            target: ContextMenuTarget::Edge(idx),
                            screen_pos: pos,
                        });
                    }
                    CanvasAction::SelectionCleared => {
                        self.context_menu = None;
                    }
                }
            }
        });

        // 7. Context Menu Popup (AC #4, #5)
        if let Some(ref menu) = self.context_menu.clone() {
            let mut close_menu = false;
            #[allow(deprecated)]
            let area = egui::Area::new(egui::Id::new("canvas_context_menu"))
                .fixed_pos(menu.screen_pos)
                .order(egui::Order::Foreground);

            area.show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    match menu.target {
                        ContextMenuTarget::Node(node_idx) => {
                            if let Some(node) = self.graph.graph.node_weight(node_idx) {
                                let agent_name = node.name.clone();
                                ui.label(RichText::new(&agent_name).strong());
                                ui.separator();

                                if ui.button("Attach (Focus)").clicked() {
                                    self.dispatch_action(ActionRequest::Attach {
                                        agent: agent_name.clone(),
                                    });
                                    close_menu = true;
                                }
                                if ui.button("View Transcript").clicked() {
                                    self.dispatch_action(ActionRequest::Transcript {
                                        agent: agent_name.clone(),
                                        last_lines: 30,
                                    });
                                    close_menu = true;
                                }
                                if ui.button("Inject...").clicked() {
                                    // Select node so inject field is available in side panel
                                    self.canvas.selected_node = Some(node_idx);
                                    close_menu = true;
                                }
                                if ui
                                    .button(
                                        RichText::new("Kill Agent")
                                            .color(Color32::from_rgb(231, 76, 60)),
                                    )
                                    .clicked()
                                {
                                    self.confirm_kill_target = Some(agent_name);
                                    close_menu = true;
                                }
                            }
                        }
                        ContextMenuTarget::Edge(edge_idx) => {
                            ui.label(RichText::new("Edge Actions").strong());
                            ui.separator();
                            if ui.button("View Message Flow").clicked() {
                                self.canvas.selected_edge = Some(edge_idx);
                                self.canvas.selected_node = None;
                                close_menu = true;
                            }
                        }
                    }
                });
            });

            if close_menu
                || ui.input(|i| {
                    i.pointer.primary_clicked()
                        && !ui.rect_contains_pointer(egui::Rect::from_center_size(
                            menu.screen_pos,
                            [150.0, 150.0].into(),
                        ))
                })
            {
                self.context_menu = None;
            }
        }
    }
}

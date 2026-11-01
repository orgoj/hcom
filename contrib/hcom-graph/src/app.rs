use crate::action::{ActionRequest, ActionResult, ActionWorkerHandle, SystemActionRunner};
use crate::canvas::{CanvasAction, CanvasView};
use crate::config::{
    GraphConfig, GraphState, PlaceholderContext, load_config, load_state, replace_placeholders_str,
    replace_placeholders_vec, save_state, screen_bar_buttons,
};
use crate::layout::GraphLayout;
use crate::model::{AgentGraph, AgentStatus, EdgeKind};
use crate::sync::BackgroundSync;
use crate::table_data::{DEFAULT_COLUMNS, REPORT_FIELDS, TableRow, filter_row, sort_rows};
use egui::{Color32, RichText};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanelTab {
    Summary,
    Report,
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
    pub confirm_restart_target: Option<(String, Vec<String>)>,
    pub confirm_kill_group: Option<Vec<String>>,
    pub custom_cmd_confirm: Option<(String, Vec<String>, Option<String>)>,
    pub inject_input: String,
    pub loaded_transcript: Option<(String, String)>,
    pub context_menu: Option<ContextMenuState>,

    // Config & state
    pub hcom_dir: PathBuf,
    pub config: GraphConfig,
    pub state: GraphState,
    pub selection_generation: u64,
    pub search_query: String,

    // Real table data & groups
    pub table_rows: Vec<TableRow>,
    pub catalog_groups: Vec<String>,
    pub tool_groups: Vec<String>,
    pub last_table_refresh: Instant,
    pub last_stats_refresh: Instant,
    pub table_reload_in_flight: bool,
    pub stats_reload_in_flight: bool,
    pub cached_event_stats: std::collections::HashMap<String, crate::table_data::EventStatsEntry>,

    // Async panel contents (agent, generation, content)
    pub screen_content: Option<(String, u64, String)>,
    pub transcript_content: Option<(String, u64, String)>,
    pub events_content: Option<(String, u64, String)>,
    pub command_logs: Vec<String>,

    // Screen tab inject bar
    pub screen_inject_text: String,
    pub screen_inject_enter: bool,

    // UI tab state
    pub side_panel_tab: SidePanelTab,

    // Dialogs
    pub selected_row_id: Option<String>,
    pub show_start_dialog: bool,
    pub start_agent_name: String,
    pub start_instance_name: String,
    pub start_cli: String,
    pub start_initial_cli: String,
    pub start_dir: String,
    pub start_initial_dir: String,
    pub start_model: String,
    pub start_initial_model: String,
    pub start_reasoning: String,
    pub start_initial_reasoning: String,
    pub start_resume: bool,
    pub start_prompt: String,

    pub show_send_dialog: bool,
    pub send_target: String,
    pub send_text: String,

    pub inject_input_has_focus: bool,
}

pub fn accept_generation_result(current_generation: u64, result_generation: u64) -> bool {
    current_generation == result_generation
}

impl HcomGraphApp {
    pub fn new(cc: &eframe::CreationContext<'_>, sync: Option<BackgroundSync>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let hcom_dir = std::env::var("HCOM_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".hcom"))
                    .unwrap_or_else(|_| PathBuf::from(".hcom"))
            });

        let config = load_config(&hcom_dir);
        let state = load_state(&hcom_dir);

        let mut graph = AgentGraph::new();
        if let Some(ref s) = sync
            && let Some(initial) = s.current_graph()
        {
            graph = initial;
        }

        let mut layout = GraphLayout::new();
        layout.sync_nodes(&graph);

        let action_worker = Some(ActionWorkerHandle::new(SystemActionRunner::new()));

        let mut app = Self {
            graph,
            layout,
            canvas: CanvasView::new(),
            sync,
            action_worker,
            pending_action: None,
            action_error: None,
            confirm_kill_target: None,
            confirm_restart_target: None,
            confirm_kill_group: None,
            custom_cmd_confirm: None,
            inject_input: String::new(),
            loaded_transcript: None,
            context_menu: None,
            hcom_dir,
            config,
            state,
            selection_generation: 0,
            search_query: String::new(),
            table_rows: Vec::new(),
            catalog_groups: Vec::new(),
            tool_groups: Vec::new(),
            last_table_refresh: Instant::now() - Duration::from_secs(100), // trigger immediate load
            last_stats_refresh: Instant::now() - Duration::from_secs(100),
            table_reload_in_flight: true,
            stats_reload_in_flight: true,
            cached_event_stats: std::collections::HashMap::new(),
            screen_content: None,
            transcript_content: None,
            events_content: None,
            command_logs: Vec::new(),
            screen_inject_text: String::new(),
            screen_inject_enter: true,
            selected_row_id: None,
            side_panel_tab: SidePanelTab::Summary,
            show_start_dialog: false,
            start_agent_name: String::new(),
            start_instance_name: String::new(),
            start_cli: "claude".into(),
            start_initial_cli: String::new(),
            start_dir: String::new(),
            start_initial_dir: String::new(),
            start_model: String::new(),
            start_initial_model: String::new(),
            start_reasoning: String::new(),
            start_initial_reasoning: String::new(),
            start_resume: false,
            start_prompt: String::new(),
            show_send_dialog: false,
            send_target: String::new(),
            send_text: String::new(),
            inject_input_has_focus: false,
        };

        // Dispatch initial table data reload and stats reload
        app.dispatch_action(ActionRequest::ReloadTableData);
        app.dispatch_action(ActionRequest::ReloadStats);
        app
    }

    pub fn new_for_test(config: GraphConfig, state: GraphState, hcom_dir: PathBuf) -> Self {
        Self {
            graph: AgentGraph::new(),
            layout: GraphLayout::new(),
            canvas: CanvasView::new(),
            sync: None,
            action_worker: None,
            pending_action: None,
            action_error: None,
            confirm_kill_target: None,
            confirm_restart_target: None,
            confirm_kill_group: None,
            custom_cmd_confirm: None,
            context_menu: None,
            inject_input: String::new(),
            loaded_transcript: None,
            hcom_dir,
            config,
            state,
            selection_generation: 0,
            search_query: String::new(),
            table_rows: Vec::new(),
            catalog_groups: Vec::new(),
            tool_groups: Vec::new(),
            last_table_refresh: Instant::now(),
            last_stats_refresh: Instant::now(),
            table_reload_in_flight: false,
            stats_reload_in_flight: false,
            cached_event_stats: std::collections::HashMap::new(),
            screen_content: None,
            transcript_content: None,
            events_content: None,
            command_logs: Vec::new(),
            screen_inject_text: String::new(),
            screen_inject_enter: true,
            selected_row_id: None,
            side_panel_tab: SidePanelTab::Summary,
            show_start_dialog: false,
            start_agent_name: String::new(),
            start_instance_name: String::new(),
            start_cli: "claude".into(),
            start_initial_cli: String::new(),
            start_dir: String::new(),
            start_initial_dir: String::new(),
            start_model: String::new(),
            start_initial_model: String::new(),
            start_reasoning: String::new(),
            start_initial_reasoning: String::new(),
            start_resume: false,
            start_prompt: String::new(),
            show_send_dialog: false,
            send_target: String::new(),
            send_text: String::new(),
            inject_input_has_focus: false,
        }
    }

    pub fn save_state(&self) {
        if let Err(e) = save_state(&self.hcom_dir, &self.state) {
            eprintln!("Warning: failed to save graph state: {}", e);
        }
    }

    pub fn selected_agent_name(&self) -> Option<String> {
        self.selected_row().map(|r| {
            if !r.instance.is_empty() {
                r.instance
            } else {
                r.agent
            }
        })
    }

    pub fn selected_row(&self) -> Option<TableRow> {
        let sel_id = self.selected_row_id.as_ref()?;
        let all_rows = self.get_all_rows();
        all_rows.into_iter().find(|r| {
            let id = if !r.instance.is_empty() {
                r.instance.as_str()
            } else {
                r.agent.as_str()
            };
            id == sel_id
        })
    }

    pub fn on_selection_changed(&mut self) {
        let row_opt = self.selected_row();
        let is_running = row_opt
            .as_ref()
            .map(|r| r.is_running && !r.instance.is_empty())
            .unwrap_or(false);
        if !is_running {
            self.screen_content = None;
            self.transcript_content = None;
            self.events_content = None;
            return;
        }
        let agent = row_opt.unwrap().instance;
        self.selection_generation += 1;
        let gen_id = self.selection_generation;
        match self.state.bottom_tab.as_str() {
            "Screen" => {
                self.dispatch_action(ActionRequest::Screen {
                    agent,
                    generation: gen_id,
                });
            }
            "Transcript" => {
                self.dispatch_action(ActionRequest::Transcript {
                    agent,
                    generation: gen_id,
                    last_lines: 100,
                });
            }
            "Events" => {
                self.dispatch_action(ActionRequest::Events {
                    agent,
                    generation: gen_id,
                    last_n: 50,
                });
            }
            _ => {}
        }
    }

    pub fn select_bottom_tab(&mut self, tab_name: &str) {
        self.state.bottom_tab = tab_name.to_string();
        self.save_state();
        self.on_selection_changed();
    }

    pub fn cycle_sort(&mut self, col: &str) {
        if self.state.sort_col.as_deref() == Some(col) {
            if !self.state.sort_reverse {
                self.state.sort_reverse = true;
            } else {
                self.state.sort_col = None;
                self.state.sort_reverse = false;
            }
        } else {
            self.state.sort_col = Some(col.to_string());
            self.state.sort_reverse = false;
        }
        self.save_state();
    }

    pub fn resolve_attach_cmd(&self, row: Option<&TableRow>, agent: &str) -> Option<Vec<String>> {
        let term = row.map(|r| r.where_term.as_str()).unwrap_or("");
        let key = if term.starts_with("herdr") {
            "herdr"
        } else {
            term
        };
        if let Some(template) = self
            .config
            .attach
            .get(key)
            .or_else(|| self.config.attach.get(term))
        {
            let dir = row.map(|r| r.directory.as_str()).unwrap_or("");
            let instance = row.map(|r| r.instance.as_str()).unwrap_or(agent);
            let catalog_agent = row.map(|r| r.agent.as_str()).unwrap_or(agent);
            let (pane_id, herdr_ws, herdr_tab, terminal_effective) =
                if let Some(lc) = row.and_then(|r| r.launch_context.as_ref()) {
                    (
                        lc.pane_id.as_deref().unwrap_or(""),
                        lc.herdr_workspace.as_deref().unwrap_or(""),
                        lc.herdr_tab.as_deref().unwrap_or(""),
                        lc.terminal_preset_effective.as_deref().unwrap_or(term),
                    )
                } else {
                    ("", "", "", term)
                };
            let ctx = PlaceholderContext {
                dir,
                instance,
                agent: catalog_agent,
                terminal: terminal_effective,
                pane_id,
                herdr_workspace: herdr_ws,
                herdr_tab,
            };
            replace_placeholders_vec(template, &ctx).ok()
        } else {
            None
        }
    }

    pub fn dispatch_action(&mut self, req: ActionRequest) {
        if let Some(ref worker) = self.action_worker {
            match &req {
                ActionRequest::Attach { agent, .. } => {
                    self.pending_action = Some(format!("Attaching to {}...", agent));
                }
                ActionRequest::Inject { agent, .. } => {
                    self.pending_action = Some(format!("Injecting to {}...", agent));
                }
                ActionRequest::Transcript { agent, .. } => {
                    self.pending_action = Some(format!("Loading transcript for {}...", agent));
                }
                ActionRequest::Screen { agent, .. } => {
                    self.pending_action = Some(format!("Loading screen for {}...", agent));
                }
                ActionRequest::Events { agent, .. } => {
                    self.pending_action = Some(format!("Loading events for {}...", agent));
                }
                ActionRequest::Kill { agent } => {
                    self.pending_action = Some(format!("Killing {}...", agent));
                }
                ActionRequest::KillGroup { agents } => {
                    self.pending_action = Some(format!("Killing {} agents...", agents.len()));
                }
                ActionRequest::RunCommand { cmd, .. } => {
                    self.pending_action = Some(format!("Running {}...", cmd.join(" ")));
                }
                ActionRequest::SpawnDetached { cmd, .. } => {
                    self.pending_action = Some(format!("Spawning {}...", cmd.join(" ")));
                }
                ActionRequest::ReloadTableData => {}
                ActionRequest::ReloadStats => {}
                ActionRequest::Shutdown => {}
            }
            if let Err(e) = worker.try_send(req.clone()) {
                self.pending_action = None;
                self.action_error = Some(e);
                match req {
                    ActionRequest::ReloadTableData => self.table_reload_in_flight = false,
                    ActionRequest::ReloadStats => self.stats_reload_in_flight = false,
                    _ => {}
                }
            }
        } else {
            match req {
                ActionRequest::ReloadTableData => self.table_reload_in_flight = false,
                ActionRequest::ReloadStats => self.stats_reload_in_flight = false,
                _ => {}
            }
        }
    }

    fn render_table(&mut self, ui: &mut egui::Ui, rows: &[TableRow]) {
        let columns = if !self.state.columns.is_empty() {
            self.state.columns.clone()
        } else {
            DEFAULT_COLUMNS.iter().map(|s| s.to_string()).collect()
        };

        egui::ScrollArea::both()
            .id_salt("table_scroll_area")
            .show(ui, |ui| {
                egui::Grid::new("agent_table_grid")
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        // Header row
                        for col in &columns {
                            let sort_indicator =
                                if self.state.sort_col.as_deref() == Some(col.as_str()) {
                                    if self.state.sort_reverse {
                                        " ▼"
                                    } else {
                                        " ▲"
                                    }
                                } else {
                                    ""
                                };
                            if ui
                                .button(
                                    RichText::new(format!("{}{}", col, sort_indicator)).strong(),
                                )
                                .clicked()
                            {
                                self.cycle_sort(col);
                            }
                        }
                        ui.end_row();

                        // Rows
                        let selected_id = self.selected_row_id.clone();
                        for row in rows {
                            let target_name = if !row.instance.is_empty() {
                                row.instance.clone()
                            } else {
                                row.agent.clone()
                            };
                            let is_selected = selected_id.as_deref() == Some(&target_name);
                            let mut row_clicked = false;
                            for col in &columns {
                                let val = row.get_cell_value(col);
                                let resp = ui.selectable_label(is_selected, &val);
                                if resp.clicked() {
                                    row_clicked = true;
                                }
                            }
                            ui.end_row();

                            if row_clicked {
                                self.selected_row_id = Some(target_name.clone());
                                if !row.instance.is_empty() {
                                    if let Some(idx) = self.graph.get_node_index(&row.instance) {
                                        self.canvas.selected_node = Some(idx);
                                        self.canvas.selected_edge = None;
                                    } else {
                                        self.canvas.selected_node = None;
                                    }
                                } else {
                                    self.canvas.selected_node = None;
                                }
                                self.on_selection_changed();
                            }
                        }
                    });
            });
    }

    fn render_report_view(&self, ui: &mut egui::Ui, row: &TableRow) {
        let configured = self.config.report.iter().map(|s| s.as_str());
        let fields: Vec<&str> = if !self.config.report.is_empty() {
            configured.collect()
        } else {
            REPORT_FIELDS.to_vec()
        };

        egui::ScrollArea::vertical()
            .id_salt("report_scroll_view")
            .show(ui, |ui| {
                egui::Grid::new("report_view_grid")
                    .num_columns(2)
                    .striped(true)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for key in fields {
                            let val = row.get_cell_value(key);
                            ui.label(RichText::new(key).strong());
                            ui.horizontal(|ui| {
                                ui.label(&val);
                                if !val.is_empty() && ui.small_button("📋").clicked() {
                                    ui.ctx().copy_text(val);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    }

    fn render_screen_tab(&mut self, ui: &mut egui::Ui) {
        let agent = self.selected_agent_name();
        ui.vertical(|ui| {
            // Screen output area
            egui::ScrollArea::both()
                .id_salt("screen_terminal_scroll")
                .max_height(ui.available_height() - 40.0)
                .show(ui, |ui| {
                    if let Some(ref selected) = agent {
                        if let Some((ref tr_agent, _, ref text)) = self.screen_content {
                            if tr_agent == selected {
                                ui.monospace(text);
                            } else {
                                ui.weak(format!("Loading screen for {}...", selected));
                            }
                        } else {
                            ui.weak(format!("Loading screen for {}...", selected));
                        }
                    } else {
                        ui.weak("No running agent selected");
                    }
                });

            ui.separator();

            // Screen bar buttons
            ui.horizontal(|ui| {
                let current_agent = agent.clone();

                // Inject input
                ui.label("Inject:");
                let inject_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.screen_inject_text)
                        .hint_text("Type input..."),
                );
                self.inject_input_has_focus = inject_resp.has_focus();

                ui.checkbox(&mut self.screen_inject_enter, "Enter");
                if ui.button("Inject").clicked()
                    && let Some(ref a) = current_agent
                {
                    self.dispatch_action(ActionRequest::Inject {
                        agent: a.clone(),
                        text: self.screen_inject_text.clone(),
                        enter: self.screen_inject_enter,
                    });
                }

                ui.separator();

                // Screen bar items
                let buttons = screen_bar_buttons(&self.config.screen_bar);
                for btn in buttons {
                    if btn.is_inject_dialog {
                        continue;
                    }
                    if ui.button(&btn.label).clicked()
                        && let Some(ref a) = current_agent
                    {
                        self.dispatch_action(ActionRequest::Inject {
                            agent: a.clone(),
                            text: btn.keys,
                            enter: btn.is_enter_flag,
                        });
                    }
                }
            });
        });
    }

    fn render_bottom_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.state.bottom_tab == "Screen", "Screen (F5)")
                .clicked()
            {
                self.select_bottom_tab("Screen");
            }
            if ui
                .selectable_label(self.state.bottom_tab == "Transcript", "Transcript (F6)")
                .clicked()
            {
                self.select_bottom_tab("Transcript");
            }
            if ui
                .selectable_label(self.state.bottom_tab == "Events", "Events (F7)")
                .clicked()
            {
                self.select_bottom_tab("Events");
            }
            if ui
                .selectable_label(self.state.bottom_tab == "Log", "Log (F8)")
                .clicked()
            {
                self.select_bottom_tab("Log");
            }
            if ui
                .selectable_label(self.state.bottom_tab == "Report", "Report (F9)")
                .clicked()
            {
                self.select_bottom_tab("Report");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("🔄 Refresh").clicked() {
                    self.dispatch_action(ActionRequest::ReloadTableData);
                    self.on_selection_changed();
                }
            });
        });

        ui.separator();

        match self.state.bottom_tab.as_str() {
            "Screen" => {
                self.render_screen_tab(ui);
            }
            "Transcript" => {
                let agent = self.selected_agent_name();
                egui::ScrollArea::both()
                    .id_salt("transcript_scroll_tab")
                    .show(ui, |ui| {
                        if let Some(ref selected) = agent {
                            if let Some((ref tr_agent, _, ref text)) = self.transcript_content {
                                if tr_agent == selected {
                                    ui.monospace(text);
                                } else {
                                    ui.weak(format!("Loading transcript for {}...", selected));
                                }
                            } else {
                                ui.weak(format!("Loading transcript for {}...", selected));
                            }
                        } else {
                            ui.weak("No agent selected");
                        }
                    });
            }
            "Events" => {
                let agent = self.selected_agent_name();
                egui::ScrollArea::both()
                    .id_salt("events_scroll_tab")
                    .show(ui, |ui| {
                        if let Some(ref selected) = agent {
                            if let Some((ref ev_agent, _, ref text)) = self.events_content {
                                if ev_agent == selected {
                                    ui.monospace(text);
                                } else {
                                    ui.weak(format!("Loading events for {}...", selected));
                                }
                            } else {
                                ui.weak(format!("Loading events for {}...", selected));
                            }
                        } else {
                            ui.weak("No agent selected");
                        }
                    });
            }
            "Log" => {
                ui.horizontal(|ui| {
                    if ui.button("Clear Log").clicked() {
                        self.command_logs.clear();
                    }
                });
                egui::ScrollArea::both()
                    .id_salt("command_logs_scroll")
                    .show(ui, |ui| {
                        if self.command_logs.is_empty() {
                            ui.weak("No command log entries.");
                        } else {
                            for line in &self.command_logs {
                                if line.starts_with("[ERROR]") {
                                    ui.colored_label(Color32::from_rgb(231, 76, 60), line);
                                } else if line.starts_with("[OK]") {
                                    ui.colored_label(Color32::from_rgb(46, 204, 113), line);
                                } else {
                                    ui.monospace(line);
                                }
                            }
                        }
                    });
            }
            "Report" => {
                if let Some(row) = self.selected_row() {
                    self.render_report_view(ui, &row);
                } else {
                    ui.weak("No agent selected");
                }
            }
            _ => {
                ui.weak("Unknown tab");
            }
        }
    }

    fn get_all_rows(&self) -> Vec<TableRow> {
        if !self.table_rows.is_empty() {
            self.table_rows.clone()
        } else {
            self.graph
                .graph
                .node_weights()
                .map(TableRow::from_agent_node)
                .collect()
        }
    }
}

impl eframe::App for HcomGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Ensure bottom_tab is never Table
        if self.state.bottom_tab == "Table" {
            self.state.bottom_tab = "Screen".to_string();
        }

        // Periodic table data reload
        if !self.table_reload_in_flight
            && self.last_table_refresh.elapsed() >= Duration::from_secs(self.config.refresh_seconds)
        {
            self.last_table_refresh = Instant::now();
            self.table_reload_in_flight = true;
            self.dispatch_action(ActionRequest::ReloadTableData);
        }

        // Periodic stats reload
        if !self.stats_reload_in_flight
            && self.last_stats_refresh.elapsed() >= Duration::from_secs(self.config.stats_seconds)
        {
            self.last_stats_refresh = Instant::now();
            self.stats_reload_in_flight = true;
            self.dispatch_action(ActionRequest::ReloadStats);
        }

        // Keyboard shortcuts
        let (clear_search, switch_tab) = ctx.input(|i| {
            let alt_x = i.modifiers.alt && i.key_pressed(egui::Key::X);
            let tab = if i.key_pressed(egui::Key::F4) {
                Some("Table")
            } else if i.key_pressed(egui::Key::F5) {
                Some("Screen")
            } else if i.key_pressed(egui::Key::F6) {
                Some("Transcript")
            } else if i.key_pressed(egui::Key::F7) {
                Some("Events")
            } else if i.key_pressed(egui::Key::F8) {
                Some("Log")
            } else if i.key_pressed(egui::Key::F9) {
                Some("Report")
            } else {
                None
            };
            (alt_x, tab)
        });

        if clear_search && !self.inject_input_has_focus {
            self.search_query.clear();
        }
        if let Some(tab) = switch_tab {
            if tab == "Table" {
                self.state.view_mode = "table_only".to_string();
                self.save_state();
            } else {
                self.select_bottom_tab(tab);
            }
        }

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
        let mut queued_actions = Vec::new();
        if let Some(ref worker) = self.action_worker {
            let mut results = Vec::new();
            while let Some(res) = worker.try_recv() {
                results.push(res);
            }
            for res in results {
                self.pending_action = None;
                match res {
                    ActionResult::AttachFinished { agent, result } => {
                        if let Err(e) = result {
                            let msg = format!("Attach to '{}' failed: {}", agent, e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        } else {
                            self.command_logs
                                .push(format!("[OK] Attached to {}", agent));
                        }
                    }
                    ActionResult::InjectFinished { agent, result } => match result {
                        Ok(()) => {
                            self.inject_input.clear();
                            self.screen_inject_text.clear();
                            self.command_logs
                                .push(format!("[OK] Injected to {}", agent));
                            queued_actions.push(ActionRequest::Screen {
                                agent,
                                generation: self.selection_generation,
                            });
                        }
                        Err(e) => {
                            let msg = format!("Inject to '{}' failed: {}", agent, e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        }
                    },
                    ActionResult::TranscriptLoaded {
                        agent,
                        generation,
                        content,
                    } => {
                        if accept_generation_result(self.selection_generation, generation) {
                            match content {
                                Ok(text) => {
                                    self.loaded_transcript = Some((agent.clone(), text.clone()));
                                    self.transcript_content = Some((agent, generation, text));
                                }
                                Err(e) => {
                                    let msg = format!("Transcript for '{}' failed: {}", agent, e);
                                    self.action_error = Some(msg.clone());
                                    self.transcript_content =
                                        Some((agent, generation, format!("Error: {}", e)));
                                }
                            }
                        }
                    }
                    ActionResult::ScreenLoaded {
                        agent,
                        generation,
                        content,
                    } => {
                        if accept_generation_result(self.selection_generation, generation) {
                            match content {
                                Ok(text) => {
                                    self.screen_content = Some((agent, generation, text));
                                }
                                Err(e) => {
                                    let msg = format!("Screen for '{}' failed: {}", agent, e);
                                    self.action_error = Some(msg.clone());
                                    self.screen_content =
                                        Some((agent, generation, format!("Error: {}", e)));
                                }
                            }
                        }
                    }
                    ActionResult::EventsLoaded {
                        agent,
                        generation,
                        content,
                    } => {
                        if accept_generation_result(self.selection_generation, generation) {
                            match content {
                                Ok(text) => {
                                    self.events_content = Some((agent, generation, text));
                                }
                                Err(e) => {
                                    let msg = format!("Events for '{}' failed: {}", agent, e);
                                    self.action_error = Some(msg.clone());
                                    self.events_content =
                                        Some((agent, generation, format!("Error: {}", e)));
                                }
                            }
                        }
                    }
                    ActionResult::KillFinished { agent, result } => {
                        if let Err(e) = result {
                            let msg = format!("Kill '{}' failed: {}", agent, e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        } else {
                            self.command_logs
                                .push(format!("[OK] Killed agent {}", agent));
                            queued_actions.push(ActionRequest::ReloadTableData);
                        }
                    }
                    ActionResult::KillGroupFinished { count, result } => match result {
                        Ok(()) => {
                            self.command_logs
                                .push(format!("[OK] Killed {} agents", count));
                            queued_actions.push(ActionRequest::ReloadTableData);
                        }
                        Err(e) => {
                            let msg = format!("Kill group failed: {}", e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        }
                    },
                    ActionResult::CommandFinished { result } => match result {
                        Ok(out) => {
                            self.command_logs
                                .push(format!("[OK] Command output:\n{}", out.trim()));
                        }
                        Err(e) => {
                            let msg = format!("Command failed: {}", e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        }
                    },
                    ActionResult::SpawnDetachedFinished { cmd, result } => match result {
                        Ok(()) => {
                            self.command_logs
                                .push(format!("[OK] Spawned detached: {}", cmd.join(" ")));
                        }
                        Err(e) => {
                            let msg = format!("Spawn '{}' failed: {}", cmd.join(" "), e);
                            self.action_error = Some(msg.clone());
                            self.command_logs.push(format!("[ERROR] {}", msg));
                        }
                    },
                    ActionResult::TableDataLoaded {
                        mut rows,
                        catalog_groups,
                        tool_groups,
                    } => {
                        self.table_reload_in_flight = false;
                        crate::table_data::apply_event_stats(&mut rows, &self.cached_event_stats);
                        self.table_rows = rows;
                        self.catalog_groups = catalog_groups;
                        self.tool_groups = tool_groups;
                        self.on_selection_changed();
                    }
                    ActionResult::TableDataFailed { error } => {
                        self.table_reload_in_flight = false;
                        self.command_logs.push(format!("[ERROR] {}", error));
                    }
                    ActionResult::StatsLoaded { stats } => {
                        self.stats_reload_in_flight = false;
                        self.cached_event_stats = stats;
                        crate::table_data::apply_event_stats(
                            &mut self.table_rows,
                            &self.cached_event_stats,
                        );
                    }
                    ActionResult::StatsFailed { error } => {
                        self.stats_reload_in_flight = false;
                        self.command_logs.push(format!("[ERROR] {}", error));
                    }
                }
                ctx.request_repaint();
            }
        }

        for action in queued_actions {
            self.dispatch_action(action);
        }

        // 3. Physics simulation step
        let dt = ctx.input(|i| i.stable_dt);
        let motion = self.layout.step(&self.graph, dt);
        if motion > 0.0 || self.canvas.dragged_node.is_some() || self.pending_action.is_some() {
            ctx.request_repaint();
        }

        // Collect all table rows and apply filters
        let all_rows = self.get_all_rows();
        let total_count = all_rows.len();
        let mut filtered_rows: Vec<TableRow> = all_rows
            .iter()
            .filter(|&r| {
                filter_row(
                    r,
                    &self.search_query,
                    self.state.running_only,
                    self.state.blocked_only,
                    self.state.hide_disabled,
                    self.state.hide_adhoc,
                )
            })
            .cloned()
            .collect();
        sort_rows(
            &mut filtered_rows,
            self.state.sort_col.as_deref().unwrap_or("instance"),
            self.state.sort_reverse,
            self.state.empty_last,
        );
        let shown_count = filtered_rows.len();

        let visible_node_indices: std::collections::HashSet<NodeIndex> = filtered_rows
            .iter()
            .filter_map(|r| {
                let name = if !r.instance.is_empty() {
                    &r.instance
                } else {
                    &r.agent
                };
                self.graph.get_node_index(name)
            })
            .collect();

        // 4. Top Status & Toolbar Panel
        egui::Panel::top("top_status_bar").show(ui, |ui| {
            // Row 1: Header + Counts
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

                ui.label(format!("Agents: {}", self.graph.graph.node_count()));
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
                ui.label(format!("Edges: {}", self.graph.graph.edge_count()));

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

            ui.separator();

            // Row 2: View modes + Search Filter + Checkboxes + Count
            ui.horizontal(|ui| {
                // View modes
                if ui
                    .selectable_label(self.state.view_mode == "graph_only", "Graph")
                    .clicked()
                {
                    self.state.view_mode = "graph_only".to_string();
                    self.save_state();
                }
                if ui
                    .selectable_label(self.state.view_mode == "table_only", "Table (F4)")
                    .clicked()
                {
                    self.state.view_mode = "table_only".to_string();
                    self.save_state();
                }
                if ui
                    .selectable_label(self.state.view_mode == "split_horizontal", "Split H")
                    .clicked()
                {
                    self.state.view_mode = "split_horizontal".to_string();
                    self.save_state();
                }
                if ui
                    .selectable_label(
                        self.state.view_mode == "split" || self.state.view_mode == "split_vertical",
                        "Split V",
                    )
                    .clicked()
                {
                    self.state.view_mode = "split_vertical".to_string();
                    self.save_state();
                }

                ui.separator();

                // Search query
                ui.label("Filter:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search agent, dir, tool... (/ to focus)"),
                );
                if ui.small_button("✖").clicked() {
                    self.search_query.clear();
                }

                // Filter checkboxes
                let mut filter_changed = false;
                if ui
                    .checkbox(&mut self.state.running_only, "Running only")
                    .changed()
                {
                    filter_changed = true;
                }
                if ui
                    .checkbox(&mut self.state.blocked_only, "Blocked only")
                    .changed()
                {
                    filter_changed = true;
                }
                if ui
                    .checkbox(&mut self.state.hide_disabled, "Hide disabled")
                    .changed()
                {
                    filter_changed = true;
                }
                if ui
                    .checkbox(&mut self.state.hide_adhoc, "Hide ad-hoc")
                    .changed()
                {
                    filter_changed = true;
                }

                if filter_changed {
                    self.save_state();
                }

                ui.separator();
                ui.label(format!(
                    "Zobrazeno: {} / Celkem: {}",
                    shown_count, total_count
                ));
            });

            // Row 3: Action Toolbar (Start, Send, Open With, Commands, Attach, Restart, Kill, Kill Group)
            let mut toolbar_action_to_dispatch = None;
            ui.horizontal_wrapped(|ui| {
                let selected_row_opt = self.selected_row();
                let selected_agent_opt = self.selected_agent_name();
                let has_selected_dir = selected_row_opt
                    .as_ref()
                    .map(|r| !r.directory.is_empty())
                    .unwrap_or(false);

                // Start button
                if ui.button("Start...").clicked() {
                    if let Some(r) = selected_row_opt.as_ref() {
                        self.start_agent_name = r.agent.clone();
                        self.start_instance_name = if !r.agent.is_empty() {
                            format!("{}__", r.agent)
                        } else {
                            String::new()
                        };
                        self.start_model = r.model.clone();
                        self.start_initial_model = r.model.clone();
                        self.start_reasoning = r.reasoning.clone();
                        self.start_initial_reasoning = r.reasoning.clone();
                        self.start_dir = r.directory.clone();
                        self.start_initial_dir = r.directory.clone();
                        self.start_cli = if !r.cli.is_empty() {
                            r.cli.clone()
                        } else {
                            "claude".into()
                        };
                        self.start_initial_cli = self.start_cli.clone();
                    } else {
                        self.start_agent_name.clear();
                        self.start_instance_name.clear();
                        self.start_model.clear();
                        self.start_initial_model.clear();
                        self.start_reasoning.clear();
                        self.start_initial_reasoning.clear();
                        self.start_dir.clear();
                        self.start_initial_dir.clear();
                        self.start_cli = "claude".into();
                        self.start_initial_cli.clear();
                    }
                    self.start_resume = false;
                    self.start_prompt.clear();
                    self.show_start_dialog = true;
                }

                // Send button
                let is_running = selected_row_opt
                    .as_ref()
                    .map(|r| r.is_running && !r.instance.is_empty())
                    .unwrap_or(false);
                let can_restart = selected_row_opt
                    .as_ref()
                    .map(|r| r.is_running || !r.agent.is_empty())
                    .unwrap_or(false);

                if ui
                    .add_enabled(is_running, egui::Button::new("Send..."))
                    .clicked()
                {
                    if let Some(ref r) = selected_row_opt {
                        self.send_target = if !r.instance.is_empty() {
                            r.instance.clone()
                        } else {
                            r.agent.clone()
                        };
                    }
                    self.show_send_dialog = true;
                }

                ui.separator();

                // Open with buttons
                for item in &self.config.open_with {
                    let dir = selected_row_opt
                        .as_ref()
                        .map(|r| r.directory.as_str())
                        .unwrap_or("");
                    let instance = selected_row_opt
                        .as_ref()
                        .map(|r| r.instance.as_str())
                        .unwrap_or("");
                    let agent = selected_row_opt
                        .as_ref()
                        .map(|r| r.agent.as_str())
                        .unwrap_or("");
                    let where_term = selected_row_opt
                        .as_ref()
                        .map(|r| r.where_term.as_str())
                        .unwrap_or("");

                    let (pane_id, herdr_ws, herdr_tab, terminal_effective) = if let Some(lc) =
                        selected_row_opt
                            .as_ref()
                            .and_then(|r| r.launch_context.as_ref())
                    {
                        (
                            lc.pane_id.as_deref().unwrap_or(""),
                            lc.herdr_workspace.as_deref().unwrap_or(""),
                            lc.herdr_tab.as_deref().unwrap_or(""),
                            lc.terminal_preset_effective
                                .as_deref()
                                .unwrap_or(where_term),
                        )
                    } else {
                        ("", "", "", where_term)
                    };

                    let ctx = PlaceholderContext {
                        dir,
                        instance,
                        agent,
                        terminal: terminal_effective,
                        pane_id,
                        herdr_workspace: herdr_ws,
                        herdr_tab,
                    };

                    // Check if_exists
                    if let Some(ref if_path) = item.if_exists {
                        if !has_selected_dir {
                            continue;
                        }
                        if let Ok(resolved) = replace_placeholders_str(if_path, &ctx) {
                            if !Path::new(&resolved).exists() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    let label = if let Some(ref k) = item.key {
                        format!("{} ({})", item.label, k)
                    } else {
                        item.label.clone()
                    };

                    let btn = ui.add_enabled(has_selected_dir, egui::Button::new(&label));
                    if btn.clicked() {
                        match replace_placeholders_vec(&item.cmd, &ctx) {
                            Ok(cmd) => {
                                let cwd = item
                                    .cwd
                                    .as_ref()
                                    .and_then(|c| replace_placeholders_str(c, &ctx).ok());
                                toolbar_action_to_dispatch =
                                    Some(ActionRequest::SpawnDetached { cmd, cwd });
                            }
                            Err(e) => {
                                self.action_error = Some(e);
                            }
                        }
                    }
                }

                // Custom commands
                for cmd_item in &self.config.commands {
                    if ui.button(&cmd_item.label).clicked() {
                        if cmd_item.confirm.unwrap_or(false) {
                            self.custom_cmd_confirm = Some((
                                cmd_item.label.clone(),
                                cmd_item.cmd.clone(),
                                cmd_item.cwd.clone(),
                            ));
                        } else {
                            toolbar_action_to_dispatch = Some(ActionRequest::RunCommand {
                                cmd: cmd_item.cmd.clone(),
                                cwd: cmd_item.cwd.clone(),
                            });
                        }
                    }
                }

                ui.separator();

                // Attach button
                if let Some(ref a) = selected_agent_opt {
                    let custom_cmd = self.resolve_attach_cmd(selected_row_opt.as_ref(), a);
                    if ui
                        .add_enabled(is_running, egui::Button::new("Attach"))
                        .clicked()
                    {
                        toolbar_action_to_dispatch = Some(ActionRequest::Attach {
                            agent: a.clone(),
                            custom_cmd,
                        });
                    }
                    if ui
                        .add_enabled(can_restart, egui::Button::new("Restart"))
                        .clicked()
                    {
                        let target_instance = selected_row_opt
                            .as_ref()
                            .map(|r| r.instance.clone())
                            .unwrap_or_else(|| a.clone());
                        let catalog_agent = selected_row_opt
                            .as_ref()
                            .map(|r| r.agent.clone())
                            .unwrap_or_default();
                        let cmd = if !catalog_agent.is_empty() {
                            vec![
                                "hcom".into(),
                                "agent".into(),
                                catalog_agent,
                                "--as".into(),
                                target_instance.clone(),
                                "--restart".into(),
                            ]
                        } else {
                            vec!["hcom".into(), "r".into(), target_instance.clone()]
                        };
                        self.confirm_restart_target = Some((target_instance, cmd));
                    }
                    if ui
                        .add_enabled(
                            is_running,
                            egui::Button::new(
                                RichText::new("Kill").color(Color32::from_rgb(231, 76, 60)),
                            ),
                        )
                        .clicked()
                    {
                        self.confirm_kill_target = Some(a.clone());
                    }
                }

                // Kill Group button
                ui.menu_button("Kill group ▾", |ui| {
                    // 1. Filtered (N)
                    let running_filtered: Vec<String> = filtered_rows
                        .iter()
                        .filter(|r| r.is_running && !r.instance.is_empty())
                        .map(|r| r.instance.clone())
                        .collect();
                    let count_filtered = running_filtered.len();
                    let btn_filtered = ui.add_enabled(
                        count_filtered > 0,
                        egui::Button::new(format!("Filtered ({})", count_filtered)),
                    );
                    if btn_filtered.clicked() {
                        self.confirm_kill_group = Some(running_filtered);
                        ui.close();
                    }

                    ui.separator();

                    // 2. Catalog groups (@group)
                    if !self.catalog_groups.is_empty() {
                        ui.label(RichText::new("Catalog Groups:").weak());
                        for group in &self.catalog_groups {
                            let group_targets: Vec<String> = all_rows
                                .iter()
                                .filter(|r| {
                                    r.is_running
                                        && !r.instance.is_empty()
                                        && r.groups.iter().any(|g| g == group)
                                })
                                .map(|r| r.instance.clone())
                                .collect();
                            let count = group_targets.len();
                            let btn = ui.add_enabled(
                                count > 0,
                                egui::Button::new(format!("@{} ({})", group, count)),
                            );
                            if btn.clicked() {
                                self.confirm_kill_group = Some(group_targets);
                                ui.close();
                            }
                        }
                        ui.separator();
                    }

                    // 3. Tool groups (tool:cli)
                    if !self.tool_groups.is_empty() {
                        ui.label(RichText::new("CLI Groups:").weak());
                        for tool in &self.tool_groups {
                            let tool_targets: Vec<String> = all_rows
                                .iter()
                                .filter(|r| {
                                    r.is_running && !r.instance.is_empty() && r.cli == *tool
                                })
                                .map(|r| r.instance.clone())
                                .collect();
                            let count = tool_targets.len();
                            let btn = ui.add_enabled(
                                count > 0,
                                egui::Button::new(format!("tool:{} ({})", tool, count)),
                            );
                            if btn.clicked() {
                                self.confirm_kill_group = Some(tool_targets);
                                ui.close();
                            }
                        }
                    }
                });
            });

            if let Some(action) = toolbar_action_to_dispatch {
                self.dispatch_action(action);
            }
        });

        // 5. Confirmation Bars / Modals
        if let Some(ref target) = self.confirm_kill_target.clone() {
            egui::Window::new("Confirm Kill Agent")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.colored_label(
                        Color32::from_rgb(231, 76, 60),
                        format!("Are you sure you want to kill agent '{}'?", target),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Confirm Kill").clicked() {
                            if let Some(node) = self.graph.get_agent(target) {
                                if node.status == AgentStatus::Stopped {
                                    self.action_error =
                                        Some(format!("Agent '{}' is already stopped", target));
                                } else {
                                    self.dispatch_action(ActionRequest::Kill {
                                        agent: target.clone(),
                                    });
                                }
                            } else {
                                self.dispatch_action(ActionRequest::Kill {
                                    agent: target.clone(),
                                });
                            }
                            self.confirm_kill_target = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_kill_target = None;
                        }
                    });
                });
        }

        if let Some((ref target, ref cmd)) = self.confirm_restart_target.clone() {
            egui::Window::new("Confirm Restart Agent")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "Restart agent '{}' via '{}'?",
                        target,
                        cmd.join(" ")
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Confirm Restart").clicked() {
                            self.dispatch_action(ActionRequest::RunCommand {
                                cmd: cmd.clone(),
                                cwd: None,
                            });
                            self.confirm_restart_target = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_restart_target = None;
                        }
                    });
                });
        }

        if let Some(ref targets) = self.confirm_kill_group.clone() {
            egui::Window::new("Confirm Kill Group")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.colored_label(
                        Color32::from_rgb(231, 76, 60),
                        format!(
                            "The following {} agents will be killed:\n{}",
                            targets.len(),
                            targets.join(", ")
                        ),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Confirm Kill Group").clicked() {
                            self.dispatch_action(ActionRequest::KillGroup {
                                agents: targets.clone(),
                            });
                            self.confirm_kill_group = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_kill_group = None;
                        }
                    });
                });
        }

        if let Some((ref label, ref cmd, ref cwd)) = self.custom_cmd_confirm.clone() {
            egui::Window::new(format!("Confirm: {}", label))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Run command:\n{}", cmd.join(" ")));
                    ui.horizontal(|ui| {
                        if ui.button("Execute").clicked() {
                            self.dispatch_action(ActionRequest::RunCommand {
                                cmd: cmd.clone(),
                                cwd: cwd.clone(),
                            });
                            self.custom_cmd_confirm = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.custom_cmd_confirm = None;
                        }
                    });
                });
        }

        // Start Dialog
        if self.show_start_dialog {
            egui::Window::new("Start Agent")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Catalog Agent:");
                    if ui
                        .text_edit_singleline(&mut self.start_agent_name)
                        .changed()
                        && !self.start_agent_name.is_empty()
                    {
                        self.start_instance_name = format!("{}__", self.start_agent_name);
                    }
                    ui.label("Instance Name (optional, default auto):");
                    ui.text_edit_singleline(&mut self.start_instance_name);

                    ui.label("CLI Tool:");
                    ui.text_edit_singleline(&mut self.start_cli);

                    ui.label("Model (optional):");
                    ui.text_edit_singleline(&mut self.start_model);

                    ui.label("Reasoning (optional):");
                    ui.text_edit_singleline(&mut self.start_reasoning);

                    ui.label("Working Directory (optional):");
                    ui.text_edit_singleline(&mut self.start_dir);

                    ui.checkbox(&mut self.start_resume, "Resume session (--resume)");

                    ui.label("Prompt (optional):");
                    ui.text_edit_multiline(&mut self.start_prompt);

                    ui.horizontal(|ui| {
                        if ui.button("Launch").clicked() {
                            let mut cmd = vec!["hcom".into()];
                            if !self.start_agent_name.is_empty() {
                                cmd.push("agent".into());
                                cmd.push(self.start_agent_name.clone());
                            } else if !self.start_cli.is_empty() {
                                cmd.push("1".into());
                                cmd.push(self.start_cli.clone());
                            } else {
                                cmd.push("1".into());
                                cmd.push("claude".into());
                            }

                            if self.start_resume {
                                cmd.push("--resume".into());
                            }

                            let auto_prefix = format!("{}__", self.start_agent_name);
                            let inst_trimmed = self.start_instance_name.trim();
                            if !inst_trimmed.is_empty() && inst_trimmed != auto_prefix {
                                cmd.push("--as".into());
                                cmd.push(inst_trimmed.to_string());
                            }

                            if !self.start_dir.trim().is_empty()
                                && (self.start_dir != self.start_initial_dir
                                    || self.start_agent_name.is_empty())
                            {
                                cmd.push("--dir".into());
                                cmd.push(self.start_dir.trim().to_string());
                            }

                            if !self.start_agent_name.is_empty() {
                                if !self.start_cli.trim().is_empty()
                                    && self.start_cli != self.start_initial_cli
                                {
                                    cmd.push("--cli".into());
                                    cmd.push(self.start_cli.trim().to_string());
                                }

                                if !self.start_model.trim().is_empty()
                                    && self.start_model != self.start_initial_model
                                {
                                    cmd.push("--model".into());
                                    cmd.push(self.start_model.trim().to_string());
                                }

                                if !self.start_reasoning.trim().is_empty()
                                    && self.start_reasoning != self.start_initial_reasoning
                                {
                                    cmd.push("--reasoning".into());
                                    cmd.push(self.start_reasoning.trim().to_string());
                                }
                            }

                            if !self.start_prompt.trim().is_empty() {
                                cmd.push("--hcom-prompt".into());
                                cmd.push(self.start_prompt.trim().to_string());
                            }

                            let launch_dir = if !self.start_dir.trim().is_empty() {
                                Some(self.start_dir.clone())
                            } else {
                                None
                            };

                            self.dispatch_action(ActionRequest::RunCommand {
                                cmd,
                                cwd: launch_dir,
                            });
                            self.show_start_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_start_dialog = false;
                        }
                    });
                });
        }

        // Send Dialog
        if self.show_send_dialog {
            egui::Window::new("Send Message")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Target Agent / Instance:");
                    ui.text_edit_singleline(&mut self.send_target);
                    ui.label("Message:");
                    ui.text_edit_multiline(&mut self.send_text);
                    ui.horizontal(|ui| {
                        if ui.button("Send").clicked() && !self.send_target.trim().is_empty() {
                            let cmd = vec![
                                "hcom".into(),
                                "send".into(),
                                "-b".into(),
                                format!("@{}", self.send_target.trim().trim_start_matches('@')),
                                "--".into(),
                                self.send_text.clone(),
                            ];
                            self.dispatch_action(ActionRequest::RunCommand { cmd, cwd: None });
                            self.send_text.clear();
                            self.show_send_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_send_dialog = false;
                        }
                    });
                });
        }

        // 6. Right Side Panel for Agent Details & Report
        let has_selection =
            self.canvas.selected_node.is_some() || self.canvas.selected_edge.is_some();
        if has_selection {
            egui::Panel::right("detail_side_panel").show(ui, |ui| {
                ui.set_min_width(340.0);
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

                // Tab selector: Summary vs Report
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Summary, "Summary");
                    ui.selectable_value(&mut self.side_panel_tab, SidePanelTab::Report, "Report");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖").clicked() {
                            self.canvas.selected_node = None;
                            self.canvas.selected_edge = None;
                        }
                    });
                });
                ui.separator();

                match self.side_panel_tab {
                    SidePanelTab::Summary => {
                        // Render Selected Node Details
                        if let Some(node_idx) = self.canvas.selected_node {
                            if let Some(node) = self.graph.graph.node_weight(node_idx) {
                                let agent_name = node.name.clone();
                                let status_col = CanvasView::status_color(&node.status);

                                ui.horizontal(|ui| {
                                    ui.heading(RichText::new(&node.name).strong());
                                    ui.colored_label(status_col, format!("{:?}", node.status));
                                });
                                ui.separator();

                                egui::Grid::new("agent_meta_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Status Detail:").weak());
                                        ui.label(
                                            node.status_context
                                                .as_deref()
                                                .unwrap_or("none")
                                                .to_string(),
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
                                        ui.label(
                                            node.directory.as_deref().unwrap_or("-").to_string(),
                                        );
                                        ui.end_row();

                                        ui.label(RichText::new("Parent:").weak());
                                        ui.label(
                                            node.parent_name
                                                .as_deref()
                                                .unwrap_or("none")
                                                .to_string(),
                                        );
                                        ui.end_row();
                                    });

                                ui.add_space(8.0);
                                ui.separator();

                                // Quick Actions
                                ui.label(RichText::new("Quick Actions:").strong());
                                ui.horizontal(|ui| {
                                    let custom_cmd = self.resolve_attach_cmd(
                                        self.selected_row().as_ref(),
                                        &agent_name,
                                    );
                                    if ui.button("Attach (Focus)").clicked() {
                                        self.dispatch_action(ActionRequest::Attach {
                                            agent: agent_name.clone(),
                                            custom_cmd,
                                        });
                                    }
                                    if ui.button("Transcript").clicked() {
                                        self.dispatch_action(ActionRequest::Transcript {
                                            agent: agent_name.clone(),
                                            generation: self.selection_generation,
                                            last_lines: 30,
                                        });
                                    }
                                    if ui
                                        .button(
                                            RichText::new("Kill")
                                                .color(Color32::from_rgb(231, 76, 60)),
                                        )
                                        .clicked()
                                    {
                                        self.confirm_kill_target = Some(agent_name.clone());
                                    }
                                });

                                // Inject field
                                ui.add_space(6.0);
                                ui.label(RichText::new("Inject Prompt/Command:").strong());
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut self.inject_input);
                                    if ui.button("Inject").clicked()
                                        && !self.inject_input.trim().is_empty()
                                    {
                                        self.dispatch_action(ActionRequest::Inject {
                                            agent: agent_name.clone(),
                                            text: self.inject_input.clone(),
                                            enter: true,
                                        });
                                    }
                                });

                                // Loaded Transcript View
                                if let Some((ref tr_agent, _, ref text)) = self.transcript_content
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
                            // Render Selected Edge Message History
                            if let Some(edge_weight) = self.graph.graph.edge_weight(edge_idx) {
                                match edge_weight {
                                    EdgeKind::Message(msg) => {
                                        let from = msg.from.clone();
                                        let to = msg.to.clone();

                                        ui.horizontal(|ui| {
                                            ui.heading(
                                                RichText::new(format!("{} ⇄ {}", from, to))
                                                    .strong(),
                                            );
                                        });
                                        ui.separator();

                                        ui.label(format!(
                                            "Total Messages (directed): {}",
                                            msg.count
                                        ));
                                        ui.label(format!(
                                            "Last Active: {}",
                                            msg.last_timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                                        ));
                                        if let Some(ref intent) = msg.last_intent {
                                            ui.label(format!("Last Intent: {}", intent));
                                        }
                                        ui.label(format!("Last Preview: {}", msg.last_preview));
                                        ui.separator();

                                        ui.label(
                                            RichText::new("Conversation History (last 50):")
                                                .strong(),
                                        );
                                        let history =
                                            self.graph.get_conversation_history(&from, &to);
                                        if history.is_empty() {
                                            ui.label("No recent stored messages.");
                                        } else {
                                            egui::ScrollArea::vertical().max_height(400.0).show(
                                                ui,
                                                |ui| {
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
                                                                ui.weak(format!(
                                                                    "intent: {}",
                                                                    intent
                                                                ));
                                                            }
                                                            ui.monospace(&m.text);
                                                        });
                                                        ui.add_space(2.0);
                                                    }
                                                },
                                            );
                                        }
                                    }
                                    EdgeKind::Spawn(spawn) => {
                                        ui.heading(RichText::new("Spawn Relation").strong());
                                        ui.separator();
                                        ui.label(format!("Parent: {}", spawn.parent));
                                        ui.label(format!("Child: {}", spawn.child));
                                    }
                                }
                            }
                        }
                    }
                    SidePanelTab::Report => {
                        if let Some(row) = self.selected_row() {
                            self.render_report_view(ui, &row);
                        } else {
                            ui.weak("No agent selected");
                        }
                    }
                }
            });
        }

        // 7. Bottom Inspection Panel (always at bottom, holding Screen/Transcript/Events/Log/Report of selected agent)
        let bottom_h = self.state.sash_pos.unwrap_or(240.0).clamp(100.0, 800.0);
        let bottom_resp = egui::Panel::bottom("inspection_bottom_panel")
            .resizable(true)
            .default_size(bottom_h)
            .min_size(100.0)
            .show(ui, |ui| {
                self.render_bottom_tabs(ui);
            });
        let cur_h = bottom_resp.response.rect.height();
        if (cur_h - bottom_h).abs() > 2.0 && cur_h >= 80.0 {
            self.state.sash_pos = Some(cur_h);
            self.save_state();
        }

        // 8. Main Central Panel: View modes (Graph, Table, Split H, Split V)
        egui::CentralPanel::default().show(ui, |ui| match self.state.view_mode.as_str() {
            "graph_only" => {
                let (_response, action) = self.canvas.show(
                    ui,
                    &self.graph,
                    &mut self.layout,
                    Some(&visible_node_indices),
                );
                self.handle_canvas_action(action);
            }
            "table_only" => {
                self.render_table(ui, &filtered_rows);
            }
            "split_horizontal" => {
                let total_w = ui.available_width();
                let left_w = (total_w * 0.5).max(150.0);
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let (_response, action) = self.canvas.show(
                                ui,
                                &self.graph,
                                &mut self.layout,
                                Some(&visible_node_indices),
                            );
                            self.handle_canvas_action(action);
                        },
                    );
                    ui.separator();
                    self.render_table(ui, &filtered_rows);
                });
            }
            _ => {
                // "split" or "split_vertical"
                let total_h = ui.available_height();
                let top_h = (total_h * 0.5).max(100.0);
                ui.vertical(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), top_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let (_response, action) = self.canvas.show(
                                ui,
                                &self.graph,
                                &mut self.layout,
                                Some(&visible_node_indices),
                            );
                            self.handle_canvas_action(action);
                        },
                    );
                    ui.separator();
                    self.render_table(ui, &filtered_rows);
                });
            }
        });

        // 8. Context Menu Popup
        if let Some(ref menu) = self.context_menu.clone() {
            let mut close_menu = false;
            #[allow(deprecated)]
            let area = egui::Area::new(egui::Id::new("canvas_context_menu"))
                .fixed_pos(menu.screen_pos)
                .order(egui::Order::Foreground);

            area.show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| match menu.target {
                    ContextMenuTarget::Node(node_idx) => {
                        if let Some(node) = self.graph.graph.node_weight(node_idx) {
                            let agent_name = node.name.clone();
                            ui.label(RichText::new(&agent_name).strong());
                            ui.separator();

                            let custom_cmd =
                                self.resolve_attach_cmd(self.selected_row().as_ref(), &agent_name);
                            if ui.button("Attach (Focus)").clicked() {
                                self.dispatch_action(ActionRequest::Attach {
                                    agent: agent_name.clone(),
                                    custom_cmd,
                                });
                                close_menu = true;
                            }
                            if ui.button("View Transcript").clicked() {
                                self.dispatch_action(ActionRequest::Transcript {
                                    agent: agent_name.clone(),
                                    generation: self.selection_generation,
                                    last_lines: 30,
                                });
                                close_menu = true;
                            }
                            if ui.button("Inject...").clicked() {
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

        // S3: Periodic repaint so timer and refresh tick even when idle with no input
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl HcomGraphApp {
    fn handle_canvas_action(&mut self, action: Option<CanvasAction>) {
        if let Some(act) = action {
            match act {
                CanvasAction::NodeClicked(idx) => {
                    if let Some(node) = self.graph.graph.node_weight(idx) {
                        let name = node.name.clone();
                        self.selected_row_id = Some(name.clone());
                        let custom_cmd =
                            self.resolve_attach_cmd(self.selected_row().as_ref(), &name);
                        self.dispatch_action(ActionRequest::Attach {
                            agent: name,
                            custom_cmd,
                        });
                        self.on_selection_changed();
                    }
                }
                CanvasAction::EdgeClicked(_) => {}
                CanvasAction::NodeSecondaryClicked(idx, pos) => {
                    self.context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Node(idx),
                        screen_pos: pos,
                    });
                }
                CanvasAction::EdgeSecondaryClicked(idx, pos) => {
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
    }
}

use crate::action::{ActionRequest, ActionResult, ActionWorkerHandle, SystemActionRunner};
use crate::canvas::{CanvasAction, CanvasView};
use crate::config::{
    DETAIL_TABS, GraphConfig, GraphState, PlaceholderContext, SIDE_PANEL_MIN_WIDTH, SPLIT_FRAC_MAX,
    SPLIT_FRAC_MIN, load_config, load_state, replace_placeholders_str, replace_placeholders_vec,
    save_state, screen_bar_buttons,
};
use crate::layout::{GraphLayout, LayoutMode};
use crate::model::{AgentGraph, AgentNode, AgentStatus, EdgeKind, MessageWindow};
use crate::sync::BackgroundSync;
use crate::table_data::{
    DEFAULT_COLUMNS, REPORT_FIELDS, TableRow, filter_row, sort_rows, tooltip_lines,
};
use egui::{Color32, RichText};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Minimum canvas height kept when the agent table panel is resized or restored.
pub const CANVAS_MIN_HEIGHT: f32 = 150.0;
/// Minimum canvas width kept next to the detail panel.
pub const CANVAS_MIN_WIDTH: f32 = 200.0;
pub const TABLE_PANEL_MIN_HEIGHT: f32 = 80.0;
/// Window fraction of the table height and the detail panel width by default.
pub const DEFAULT_SPLIT_FRAC: f32 = 0.3;
const SPLIT_HANDLE_WIDTH: f32 = 6.0;

/// Panel size in px for a window fraction, at least `min` and leaving `keep`
/// px of `avail` for the canvas; `min` wins in a tiny or minimized window.
pub fn split_px(frac: f32, window: f32, min: f32, avail: f32, keep: f32) -> f32 {
    let max = (avail - keep).max(min);
    (frac * window).max(min).min(max)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitChange {
    /// The handle was dragged to this pointer coordinate along the split axis.
    Moved(f32),
    /// The handle was double-clicked.
    Reset,
}

/// A drag strip on the movable edge of `panel` (the top edge for `BottomUp`,
/// the left edge for `RightToLeft`), in a foreground area so it wins over
/// the panels and the canvas.
fn split_handle(
    ui: &egui::Ui,
    id: &str,
    panel: egui::Rect,
    side: egui::Direction,
) -> Option<SplitChange> {
    let half = SPLIT_HANDLE_WIDTH / 2.0;
    let (rect, cursor) = if side == egui::Direction::BottomUp {
        (
            egui::Rect::from_x_y_ranges(
                panel.x_range(),
                (panel.top() - half)..=(panel.top() + half),
            ),
            egui::CursorIcon::ResizeVertical,
        )
    } else {
        (
            egui::Rect::from_x_y_ranges(
                (panel.left() - half)..=(panel.left() + half),
                panel.y_range(),
            ),
            egui::CursorIcon::ResizeHorizontal,
        )
    };
    egui::Area::new(ui.scope_id().with(id))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .show(ui.ctx(), |ui| {
            let (rect, resp) = ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(cursor);
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_rgb(100, 150, 200));
            }
            if resp.double_clicked() {
                Some(SplitChange::Reset)
            } else if resp.dragged() {
                resp.interact_pointer_pos().map(|p| {
                    SplitChange::Moved(if side == egui::Direction::BottomUp {
                        p.y
                    } else {
                        p.x
                    })
                })
            } else {
                None
            }
        })
        .inner
}

/// How a legend swatch is drawn: a ring as on the node, or an edge line.
enum Swatch {
    Ring(Color32, f32),
    DashedRing(Color32),
    Line(Color32, bool),
}

/// One legend row: a small swatch drawn like the canvas, then `text`.
fn legend_row(ui: &mut egui::Ui, swatch: Swatch, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 14.0), egui::Sense::hover());
        let painter = ui.painter();
        let c = rect.center();
        match swatch {
            Swatch::Ring(color, width) => {
                painter.circle_stroke(c, 5.5, egui::Stroke::new(width, color));
            }
            Swatch::DashedRing(color) => {
                painter.extend(egui::Shape::dashed_line(
                    &crate::canvas::circle_points(c, 6.0, 24),
                    egui::Stroke::new(1.5, color),
                    3.0,
                    2.0,
                ));
            }
            Swatch::Line(color, dashed) => {
                let ends = [rect.left_center(), rect.right_center()];
                let stroke = egui::Stroke::new(2.0, color);
                if dashed {
                    painter.extend(egui::Shape::dashed_line(&ends, stroke, 6.0, 4.0));
                } else {
                    painter.line_segment(ends, stroke);
                }
            }
        }
        ui.label(text);
    });
}

/// Legend tab: the encodings of `legend`, sections without entries omitted.
fn render_legend(ui: &mut egui::Ui, legend: &crate::canvas::Legend) {
    use crate::canvas::{
        MESSAGE_EDGE_COLOR, MESSAGE_EDGE_IDLE_COLOR, ROAMING_OUTLINE_COLOR, SPAWN_EDGE_COLOR,
    };
    if legend.statuses.is_empty() {
        ui.weak("Nothing visible in the graph");
        return;
    }
    ui.strong("Status (node border)");
    for status in &legend.statuses {
        let name = match status {
            AgentStatus::Active => "active",
            AgentStatus::Listening => "listening",
            AgentStatus::Blocked => "blocked",
            AgentStatus::Stopped => "stopped",
        };
        legend_row(
            ui,
            Swatch::Ring(CanvasView::status_color(status), 2.2),
            name,
        );
    }
    ui.add_space(6.0);
    ui.strong("CLI (outer ring)");
    for (color, names) in &legend.cli {
        legend_row(ui, Swatch::Ring(*color, 2.5), &names.join(", "));
    }
    if legend.roaming {
        ui.add_space(6.0);
        ui.strong("Outline");
        legend_row(
            ui,
            Swatch::DashedRing(ROAMING_OUTLINE_COLOR),
            "roaming agent",
        );
    }
    if legend.spawn || legend.message || legend.message_idle {
        ui.add_space(6.0);
        ui.strong("Edges");
        if legend.spawn {
            legend_row(
                ui,
                Swatch::Line(SPAWN_EDGE_COLOR, true),
                "spawn (parent → child)",
            );
        }
        if legend.message {
            legend_row(ui, Swatch::Line(MESSAGE_EDGE_COLOR, false), "messages");
        }
        if legend.message_idle {
            legend_row(
                ui,
                Swatch::Line(MESSAGE_EDGE_IDLE_COLOR, false),
                "no messages in the time window",
            );
        }
    }
}

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
    pub confirm_restart_target: Option<(String, Vec<String>)>,
    pub confirm_kill_group: Option<Vec<String>>,
    pub custom_cmd_confirm: Option<(String, Vec<String>, Option<String>)>,
    pub inject_input: String,
    pub loaded_transcript: Option<(String, String)>,
    pub context_menu: Option<ContextMenuState>,
    pub context_menu_opened_this_frame: bool,
    /// Node of the hover tooltip (kept while it fades out).
    pub tooltip_node: Option<NodeIndex>,
    /// Home directory shown as `~` in the tooltip `dir` (from HOME at start).
    pub home_dir: Option<String>,
    /// Time of the last unsaved zoom/pan/panel width change.
    pub view_dirty_since: Option<Instant>,
    /// Fit the view to the visible nodes every frame until the user changes it.
    pub auto_fit: bool,
    /// A saved view was restored: turn on `auto_fit` if it shows no node.
    pub check_saved_view: bool,
    /// The Fit button was pressed: fit once on the next canvas frame.
    pub fit_requested: bool,
    /// Canvas area of the last frame.
    pub canvas_rect: egui::Rect,

    // Config & state
    pub hcom_dir: PathBuf,
    pub config: GraphConfig,
    pub state: GraphState,
    /// Active message time window (span and limit persisted in `state`).
    pub msg_window: MessageWindow,
    /// Key of the windowed edge counts held in `canvas.edge_counts`.
    window_cache_key: Option<WindowCacheKey>,
    pub selection_generation: u64,
    pub search_query: String,

    // Real table data & groups
    pub table_rows: Vec<TableRow>,
    pub catalog_groups: Vec<String>,
    pub tool_groups: Vec<String>,
    pub last_table_refresh: Instant,
    pub last_stats_refresh: Instant,
    pub table_reload_in_flight: bool,
    /// Idle column: listening periods and the one running idle_check.
    pub idle_periods: std::collections::HashMap<String, crate::idle::IdlePeriod>,
    pub idle_job: Option<crate::idle::IdleJob>,
    pub idle_error: Option<String>,
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

    // Dialogs
    pub selected_row_id: Option<String>,
    /// Data row (0-based) under the pointer in the last frame; painted as a
    /// weaker band than the selection.
    table_hover_row: Option<usize>,
    /// Screen bands of the data rows in the last frame, full grid width.
    pub table_row_bands: Vec<egui::Rect>,
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

/// (window, last_event_id, edge_count, live second or 0 for a fixed end).
type WindowCacheKey = (MessageWindow, u64, usize, i64);

/// Time window presets of the top bar: label and span in seconds (None = All).
pub const WINDOW_PRESETS: &[(&str, Option<i64>)] = &[
    ("5m", Some(300)),
    ("15m", Some(900)),
    ("1h", Some(3600)),
    ("6h", Some(21600)),
    ("24h", Some(86400)),
    ("All", None),
];

/// Header text of the active window in UTC, e.g.
/// `Range: 2026-11-01 14:05:00 – 15:05:00 UTC (live) · 37 msgs`.
pub fn format_window_range(
    window: &MessageWindow,
    now: chrono::DateTime<chrono::Utc>,
    msg_count: usize,
) -> String {
    let (start, end) = window.bounds(now);
    let start_text = match start {
        Some(s) => s.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => "start of history".to_string(),
    };
    let mode = if window.end.is_none() { " (live)" } else { "" };
    let limit = match window.last_n {
        Some(n) => format!(", newest {n}"),
        None => String::new(),
    };
    let end_fmt = if start.is_some_and(|s| s.date_naive() == end.date_naive()) {
        "%H:%M:%S"
    } else {
        "%Y-%m-%d %H:%M:%S"
    };
    format!(
        "Range: {start_text} – {} UTC{mode} · {msg_count} msgs{limit}",
        end.format(end_fmt)
    )
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
        layout.set_mode(LayoutMode::from_str_or_default(&state.layout_mode));
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
            context_menu_opened_this_frame: false,
            tooltip_node: None,
            home_dir: std::env::var("HOME").ok().filter(|h| !h.is_empty()),
            view_dirty_since: None,
            auto_fit: false,
            check_saved_view: false,
            fit_requested: false,
            canvas_rect: egui::Rect::NOTHING,
            hcom_dir,
            config,
            msg_window: state.message_window(),
            window_cache_key: None,
            state,
            selection_generation: 0,
            search_query: String::new(),
            table_rows: Vec::new(),
            catalog_groups: Vec::new(),
            tool_groups: Vec::new(),
            last_table_refresh: Instant::now() - Duration::from_secs(100), // trigger immediate load
            last_stats_refresh: Instant::now() - Duration::from_secs(100),
            table_reload_in_flight: true,
            idle_periods: Default::default(),
            idle_job: None,
            idle_error: None,
            stats_reload_in_flight: true,
            cached_event_stats: std::collections::HashMap::new(),
            screen_content: None,
            transcript_content: None,
            events_content: None,
            command_logs: Vec::new(),
            screen_inject_text: String::new(),
            screen_inject_enter: true,
            selected_row_id: None,
            table_hover_row: None,
            table_row_bands: Vec::new(),
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
        app.apply_persisted_view();

        // Dispatch initial table data reload and stats reload
        app.dispatch_action(ActionRequest::ReloadTableData);
        app.dispatch_action(ActionRequest::ReloadStats);
        app
    }

    pub fn new_for_test(config: GraphConfig, state: GraphState, hcom_dir: PathBuf) -> Self {
        Self {
            graph: AgentGraph::new(),
            layout: {
                let mut l = GraphLayout::new();
                l.set_mode(LayoutMode::from_str_or_default(&state.layout_mode));
                l
            },
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
            context_menu_opened_this_frame: false,
            tooltip_node: None,
            home_dir: std::env::var("HOME").ok().filter(|h| !h.is_empty()),
            view_dirty_since: None,
            auto_fit: false,
            check_saved_view: false,
            fit_requested: false,
            canvas_rect: egui::Rect::NOTHING,
            inject_input: String::new(),
            loaded_transcript: None,
            hcom_dir,
            config,
            msg_window: state.message_window(),
            window_cache_key: None,
            state,
            selection_generation: 0,
            search_query: String::new(),
            table_rows: Vec::new(),
            catalog_groups: Vec::new(),
            tool_groups: Vec::new(),
            last_table_refresh: Instant::now(),
            last_stats_refresh: Instant::now(),
            table_reload_in_flight: false,
            idle_periods: Default::default(),
            idle_job: None,
            idle_error: None,
            stats_reload_in_flight: false,
            cached_event_stats: std::collections::HashMap::new(),
            screen_content: None,
            transcript_content: None,
            events_content: None,
            command_logs: Vec::new(),
            screen_inject_text: String::new(),
            screen_inject_enter: true,
            selected_row_id: None,
            table_hover_row: None,
            table_row_bands: Vec::new(),
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

    /// Sets span and limit of the message window, keeps the scrubber end,
    /// and mirrors them into `state` (the caller persists with `save_state`).
    pub fn set_message_window(&mut self, span_secs: Option<i64>, last_n: Option<usize>) {
        self.msg_window.span_secs = span_secs.filter(|s| *s > 0);
        self.msg_window.last_n = last_n.filter(|n| *n > 0);
        self.state.window_span_secs = self.msg_window.span_secs;
        self.state.window_last_n = self.msg_window.last_n;
    }

    /// Clamps the scrubber end to [oldest retained message, now] (live when
    /// there are no messages) and recomputes `canvas.edge_counts` only when
    /// the window, the event watermark, the edge set or the live second changed.
    pub fn refresh_window_counts(&mut self, now: chrono::DateTime<chrono::Utc>) {
        if let Some(end) = self.msg_window.end {
            self.msg_window.end = self
                .graph
                .oldest_message_ts()
                .map(|oldest| end.clamp(oldest, now.max(oldest)));
        }
        if self.msg_window.is_unfiltered() {
            self.canvas.edge_counts = None;
            self.window_cache_key = None;
            return;
        }
        let live_second = if self.msg_window.end.is_none() {
            now.timestamp()
        } else {
            0
        };
        let key = (
            self.msg_window,
            self.graph.last_event_id,
            self.graph.edge_count(),
            live_second,
        );
        if self.window_cache_key == Some(key) && self.canvas.edge_counts.is_some() {
            return;
        }
        self.canvas.edge_counts = Some(self.graph.windowed_edge_counts(&self.msg_window, now));
        self.window_cache_key = Some(key);
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
        // A hidden detail panel loads nothing; showing it calls this again.
        if !self.state.show_side_panel {
            return;
        }
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
        match self.state.detail_tab.as_str() {
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

    /// Shows the detail panel on `tab_name` and loads its content.
    pub fn select_detail_tab(&mut self, tab_name: &str) {
        self.state.detail_tab = tab_name.to_string();
        self.state.show_side_panel = true;
        self.save_state();
        self.on_selection_changed();
    }

    /// F4 toggles the agent table; F5-F9 select a detail panel tab.
    pub fn apply_function_key(&mut self, key: egui::Key) {
        let tab = match key {
            egui::Key::F4 => {
                self.toggle_table();
                return;
            }
            egui::Key::F5 => "Screen",
            egui::Key::F6 => "Transcript",
            egui::Key::F7 => "Events",
            egui::Key::F8 => "Log",
            egui::Key::F9 => "Report",
            _ => return,
        };
        self.select_detail_tab(tab);
    }

    /// Turns pixel sizes from an older state file into window fractions once
    /// the window has a size (a minimized or unmapped window reports 0).
    /// A gui-state.json `sash_pos` is hcom-gui's sash position, so the result
    /// is only an approximation of that layout.
    /// The converted state is marked dirty, so it is saved even without other changes.
    pub fn adopt_legacy_split_sizes(&mut self, window: egui::Vec2) {
        if window.y >= 1.0
            && let Some(px) = self.state.sash_pos.take()
            && self.state.table_height_frac.is_none()
        {
            self.state.table_height_frac =
                Some((px / window.y).clamp(SPLIT_FRAC_MIN, SPLIT_FRAC_MAX));
            self.mark_view_dirty();
        }
        if window.x >= 1.0
            && let Some(px) = self.state.side_panel_width.take()
            && self.state.detail_width_frac.is_none()
        {
            self.state.detail_width_frac =
                Some((px / window.x).clamp(SPLIT_FRAC_MIN, SPLIT_FRAC_MAX));
            self.mark_view_dirty();
        }
    }

    pub fn toggle_table(&mut self) {
        self.state.show_table = !self.state.show_table;
        self.save_state();
    }

    pub fn toggle_detail_panel(&mut self) {
        self.state.show_side_panel = !self.state.show_side_panel;
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

    fn idle_column_shown(&self) -> bool {
        if self.state.columns.is_empty() {
            DEFAULT_COLUMNS.contains(&"idle")
        } else {
            self.state.columns.iter().any(|c| c == "idle")
        }
    }

    /// Follow the listening periods of the loaded rows; a period that lasted
    /// `idle_seconds` without an attempt is queued, only while the Idle column
    /// is shown.
    pub fn update_idle(&mut self, now: u64) {
        let listening: Vec<(String, u64)> = self
            .table_rows
            .iter()
            .filter(|r| !r.instance.is_empty() && r.status == "listening")
            .map(|r| (r.instance.clone(), r.since_seconds.unwrap_or(0)))
            .collect();
        self.idle_periods = crate::idle::update_periods(&self.idle_periods, &listening, now);
        if self.idle_column_shown() {
            crate::idle::queue_due(&mut self.idle_periods, now, self.config.idle_seconds);
        }
        self.fill_idle();
    }

    fn fill_idle(&mut self) {
        for r in &mut self.table_rows {
            r.idle = self
                .idle_periods
                .get(&r.instance)
                .and_then(|p| p.value.clone())
                .unwrap_or_default();
        }
    }

    /// Collect a finished idle_check and start the next one for the queued
    /// periods, at most one at a time.
    fn poll_idle(&mut self) {
        if let Some(job) = &self.idle_job
            && let Some(result) = job.try_result()
        {
            let batch = self
                .idle_job
                .take()
                .map(|mut j| std::mem::take(&mut j.batch))
                .unwrap_or_default();
            let classes = match result {
                Ok(c) => {
                    self.idle_error = None;
                    c
                }
                Err(e) => {
                    if self.idle_error.as_ref() != Some(&e) {
                        self.command_logs.push(format!("[ERROR] {e}"));
                    }
                    self.idle_error = Some(e);
                    Default::default()
                }
            };
            crate::idle::apply_result(&mut self.idle_periods, &batch, &classes);
            self.fill_idle();
        }
        if self.idle_job.is_none() && self.idle_column_shown() {
            let batch = crate::idle::next_batch(&self.idle_periods);
            if !batch.is_empty() {
                self.idle_job = Some(crate::idle::IdleJob::start(batch));
            }
        }
    }

    fn render_table(&mut self, ui: &mut egui::Ui, rows: &[TableRow]) {
        let columns = if !self.state.columns.is_empty() {
            self.state.columns.clone()
        } else {
            DEFAULT_COLUMNS.iter().map(|s| s.to_string()).collect()
        };

        let target_of = |row: &TableRow| {
            if !row.instance.is_empty() {
                row.instance.clone()
            } else {
                row.agent.clone()
            }
        };
        let mut selected_row = self
            .selected_row_id
            .as_deref()
            .and_then(|id| rows.iter().position(|r| target_of(r) == id));
        // Up/Down move the selection in the shown order, unless a text field
        // has the keyboard; the moved row is scrolled into view below.
        let (down, up) = if ui.ctx().text_edit_focused() {
            (false, false)
        } else {
            ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::ArrowUp),
                )
            })
        };
        let last = rows.len().checked_sub(1);
        let key_row = match (down, up, selected_row, last) {
            (_, _, _, None) => None,
            (true, false, Some(i), Some(l)) => Some((i + 1).min(l)),
            (true, false, None, Some(_)) => Some(0),
            (false, true, Some(i), Some(_)) => Some(i.saturating_sub(1)),
            (false, true, None, Some(l)) => Some(l),
            _ => None,
        };
        if let Some(k) = key_row {
            if selected_row != Some(k) {
                self.select_table_row(&rows[k]);
            }
            selected_row = Some(k);
        }
        let hover_row = self.table_hover_row;
        // Grid row 0 is the header; data row i is grid row i + 1. The band
        // covers the full grid width, including spacing and empty cells.
        let row_color = move |row: usize, style: &egui::Style| {
            let data_row = row.checked_sub(1)?;
            let selection = style.visuals.selection.bg_fill;
            if Some(data_row) == selected_row {
                Some(selection)
            } else if Some(data_row) == hover_row {
                Some(selection.gamma_multiply(0.4))
            } else if row % 2 == 1 {
                Some(style.visuals.faint_bg_color)
            } else {
                None
            }
        };

        // Fill the panel, so its height follows the user's resize, not the rows.
        let mut row_spans: Vec<egui::Rangef> = Vec::with_capacity(rows.len());
        let mut grid_x = egui::Rangef::NOTHING;
        let mut spacing_y = 0.0;
        let mut pointer_row = None;
        let mut clicked_row = None;
        let prev_bands = std::mem::take(&mut self.table_row_bands);
        egui::ScrollArea::both()
            .id_salt("table_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                spacing_y = ui.spacing().item_spacing.y;
                // Row bands from the last frame, registered before the cells:
                // labels (tooltips), headers and scroll bars stay on top.
                for (i, band) in prev_bands.iter().take(rows.len()).enumerate() {
                    let resp = ui.interact(
                        *band,
                        ui.scope_id().with(("table_row", i)),
                        egui::Sense::click(),
                    );
                    if resp.contains_pointer() {
                        pointer_row = Some(i);
                    }
                    if resp.clicked() {
                        clicked_row = Some(i);
                    }
                }
                let grid = egui::Grid::new("agent_table_grid")
                    .with_row_color(row_color)
                    .min_col_width(60.0)
                    .max_col_width(300.0)
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
                            let text = RichText::new(format!("{}{}", col, sort_indicator)).strong();
                            if ui.add(egui::Button::new(text).frame(false)).clicked() {
                                self.cycle_sort(col);
                            }
                        }
                        ui.end_row();

                        // Rows: plain truncated labels; the band is the row color.
                        let selected_text = ui.visuals().selection.stroke.color;
                        for (i, row) in rows.iter().enumerate() {
                            let mut span = egui::Rangef::NOTHING;
                            for col in &columns {
                                let mut text = RichText::new(row.get_cell_value(col));
                                if Some(i) == selected_row {
                                    text = text.color(selected_text);
                                }
                                let resp = ui.add(egui::Label::new(text).truncate());
                                span = span.union(resp.rect.y_range());
                            }
                            ui.end_row();
                            row_spans.push(span);
                        }
                    });
                grid_x = grid.response.rect.x_range();
                if let Some(span) = key_row.and_then(|k| row_spans.get(k)) {
                    ui.scroll_to_rect(egui::Rect::from_x_y_ranges(grid_x, *span), None);
                }
            });

        self.table_row_bands = row_spans
            .iter()
            .map(|y| egui::Rect::from_x_y_ranges(grid_x, y.expand(0.5 * spacing_y)))
            .collect();
        if pointer_row != self.table_hover_row {
            self.table_hover_row = pointer_row;
            ui.ctx().request_repaint();
        }
        if let Some(i) = clicked_row {
            self.select_table_row(&rows[i]);
        }
    }

    /// Selects a table row like a click: graph node, detail panel content.
    fn select_table_row(&mut self, row: &TableRow) {
        self.selected_row_id = Some(if !row.instance.is_empty() {
            row.instance.clone()
        } else {
            row.agent.clone()
        });
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

    fn render_report_view(&self, ui: &mut egui::Ui, row: &TableRow) {
        let configured = self.config.report.iter().map(|s| s.as_str());
        let fields: Vec<&str> = if !self.config.report.is_empty() {
            configured.collect()
        } else {
            REPORT_FIELDS.to_vec()
        };

        egui::ScrollArea::vertical()
            .id_salt("report_scroll_view")
            .auto_shrink([false, false])
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

    fn render_window_bar(&mut self, ui: &mut egui::Ui) {
        let now = chrono::Utc::now();
        ui.label("Window:");
        let mut span = self.msg_window.span_secs;
        let mut last_n = self.msg_window.last_n;
        let mut changed = false;
        // Presets persist at once; drag values only when the drag or edit ends.
        let mut persist = false;
        for (label, preset) in WINDOW_PRESETS {
            if ui.selectable_label(span == *preset, *label).clicked() {
                span = *preset;
                changed = true;
                persist = true;
            }
        }

        let mut minutes = span.map(|s| s / 60).unwrap_or(0);
        let resp = ui
            .add(
                egui::DragValue::new(&mut minutes)
                    .range(0..=525_600)
                    .suffix(" min"),
            )
            .on_hover_text("Custom window span in minutes (0 = All)");
        if resp.changed() {
            span = (minutes > 0).then_some(minutes * 60);
            changed = true;
        }
        persist |= resp.drag_stopped() || resp.lost_focus();

        let mut limit = last_n.unwrap_or(0);
        ui.label("Last:");
        let resp = ui
            .add(
                egui::DragValue::new(&mut limit)
                    .range(0..=10_000)
                    .suffix(" msgs"),
            )
            .on_hover_text("Keep only the newest N messages in the window (0 = off)");
        if resp.changed() {
            last_n = (limit > 0).then_some(limit);
            changed = true;
        }
        persist |= resp.drag_stopped() || resp.lost_focus();
        if changed {
            self.set_message_window(span, last_n);
        }
        if persist {
            self.save_state();
        }

        ui.separator();

        // Timeline scrubber: moves the window end through retained history.
        let oldest = self.graph.oldest_message_ts();
        let mut live = self.msg_window.end.is_none();
        ui.add_enabled_ui(oldest.is_some(), |ui| {
            if ui
                .checkbox(&mut live, "Live")
                .on_hover_text("Window end follows the current time")
                .changed()
            {
                self.msg_window.end = if live { None } else { Some(now) };
            }
            let min_ts = oldest.map(|o| o.timestamp()).unwrap_or(now.timestamp());
            let max_ts = now.timestamp().max(min_ts);
            let mut end_ts = self.msg_window.end.unwrap_or(now).timestamp();
            if ui
                .add(
                    egui::Slider::new(&mut end_ts, min_ts..=max_ts)
                        .show_value(false)
                        .text("end"),
                )
                .on_hover_text("Scrub the window end through message history")
                .changed()
            {
                self.msg_window.end = chrono::DateTime::from_timestamp(end_ts, 0);
            }
        });

        self.refresh_window_counts(now);
        if self.msg_window.end.is_none() && !self.msg_window.is_unfiltered() {
            // Live window: messages age out without any input event.
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
        let msg_count: usize = match &self.canvas.edge_counts {
            Some(m) => m.values().map(|c| c.count).sum(),
            None => self.graph.iter_message_edges().map(|m| m.count).sum(),
        };
        let lower_bound = self
            .canvas
            .edge_counts
            .as_ref()
            .is_some_and(|m| m.values().any(|c| c.lower_bound));
        ui.separator();
        let mut text = format_window_range(&self.msg_window, now, msg_count);
        if lower_bound {
            text.push_str(" (retained history, N+ = more may exist)");
        }
        ui.label(RichText::new(text).strong());
    }

    fn render_side_panel_summary(&mut self, ui: &mut egui::Ui) {
        // Priority 1: Selected message edge
        if let Some(edge_idx) = self.canvas.selected_edge
            && let Some(edge_weight) = self.graph.graph.edge_weight(edge_idx)
        {
            match edge_weight {
                EdgeKind::Message(msg) => {
                    let from = msg.from.clone();
                    let to = msg.to.clone();

                    ui.horizontal(|ui| {
                        ui.heading(RichText::new(format!("{} ⇄ {}", from, to)).strong());
                    });
                    ui.separator();

                    if !self.msg_window.is_unfiltered() {
                        let wc = self
                            .canvas
                            .edge_counts
                            .as_ref()
                            .and_then(|m| m.get(&edge_idx).copied())
                            .unwrap_or_default();
                        let plus = if wc.lower_bound { "+" } else { "" };
                        ui.label(format!("Messages in window (directed): {}{plus}", wc.count));
                    }
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

                    let history_title = if self.msg_window.is_unfiltered() {
                        "Conversation History (last 50):"
                    } else {
                        "Conversation History in window (last 50):"
                    };
                    ui.label(RichText::new(history_title).strong());
                    let history = self.graph.windowed_conversation_history(
                        &from,
                        &to,
                        &self.msg_window,
                        chrono::Utc::now(),
                    );
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
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.weak(m.ts.format("%H:%M:%S").to_string());
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
                    ui.heading(RichText::new("Spawn Relation").strong());
                    ui.separator();
                    ui.label(format!("Parent: {}", spawn.parent));
                    ui.label(format!("Child: {}", spawn.child));
                }
            }
            return;
        }

        // Priority 2: Selected node or selected table row
        let selected_node = self
            .canvas
            .selected_node
            .and_then(|idx| self.graph.graph.node_weight(idx).cloned());
        let selected_row = self.selected_row();

        if selected_node.is_none() && selected_row.is_none() {
            ui.heading("Agent Details");
            ui.separator();
            ui.weak("No agent or message edge selected.");
            ui.add_space(6.0);
            ui.label("Click a node or edge in the graph, or a row in the table, to view detailed status, command history, and quick actions.");
            return;
        }

        let agent_name = if let Some(ref n) = selected_node {
            n.name.clone()
        } else if let Some(ref r) = selected_row {
            if !r.instance.is_empty() {
                r.instance.clone()
            } else {
                r.agent.clone()
            }
        } else {
            return;
        };

        let status_desc = if let Some(ref n) = selected_node {
            format!("{:?}", n.status)
        } else if let Some(ref r) = selected_row {
            r.status.clone()
        } else {
            "-".to_string()
        };

        let status_col = if let Some(ref n) = selected_node {
            CanvasView::status_color(&n.status)
        } else if let Some(ref r) = selected_row {
            if r.status == "active" {
                Color32::from_rgb(46, 204, 113)
            } else if r.status == "listening" {
                Color32::from_rgb(52, 152, 219)
            } else if r.status == "blocked" {
                Color32::from_rgb(231, 76, 60)
            } else {
                Color32::from_rgb(127, 140, 141)
            }
        } else {
            Color32::GRAY
        };

        ui.horizontal(|ui| {
            ui.heading(RichText::new(&agent_name).strong());
            ui.colored_label(status_col, status_desc);
        });
        ui.separator();

        egui::Grid::new("agent_meta_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Status Detail:").weak());
                let detail = if let Some(ref n) = selected_node {
                    n.status_context.as_deref().unwrap_or("none").to_string()
                } else if let Some(ref r) = selected_row {
                    if !r.detail.is_empty() {
                        r.detail.clone()
                    } else {
                        "none".to_string()
                    }
                } else {
                    "none".to_string()
                };
                ui.label(detail);
                ui.end_row();

                ui.label(RichText::new("Status Age:").weak());
                let age = if let Some(ref n) = selected_node {
                    format!("{}s", n.status_age_seconds)
                } else if let Some(ref r) = selected_row {
                    r.since_text.clone()
                } else {
                    "-".to_string()
                };
                ui.label(age);
                ui.end_row();

                ui.label(RichText::new("Unread Count:").weak());
                let unread = if let Some(ref n) = selected_node {
                    format!("{}", n.unread_count)
                } else if let Some(ref r) = selected_row {
                    format!("{}", r.unread_count)
                } else {
                    "-".to_string()
                };
                ui.label(unread);
                ui.end_row();

                ui.label(RichText::new("CLI Tool:").weak());
                let tool = if let Some(ref n) = selected_node {
                    n.tool.as_deref().unwrap_or("-").to_string()
                } else if let Some(ref r) = selected_row {
                    if !r.cli.is_empty() {
                        r.cli.clone()
                    } else {
                        "-".to_string()
                    }
                } else {
                    "-".to_string()
                };
                ui.label(tool);
                ui.end_row();

                ui.label(RichText::new("Directory:").weak());
                let dir = if let Some(ref n) = selected_node {
                    n.directory.as_deref().unwrap_or("-").to_string()
                } else if let Some(ref r) = selected_row {
                    if !r.directory.is_empty() {
                        r.directory.clone()
                    } else {
                        "-".to_string()
                    }
                } else {
                    "-".to_string()
                };
                ui.label(dir);
                ui.end_row();

                ui.label(RichText::new("Parent:").weak());
                let parent = if let Some(ref n) = selected_node {
                    n.parent_name.as_deref().unwrap_or("none").to_string()
                } else {
                    "none".to_string()
                };
                ui.label(parent);
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();

        // Quick Actions
        ui.label(RichText::new("Quick Actions:").strong());
        ui.horizontal(|ui| {
            let custom_cmd = self.resolve_attach_cmd(self.selected_row().as_ref(), &agent_name);
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
                .button(RichText::new("Kill").color(Color32::from_rgb(231, 76, 60)))
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
            if ui.button("Inject").clicked() && !self.inject_input.trim().is_empty() {
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

    fn render_screen_tab(&mut self, ui: &mut egui::Ui) {
        let agent = self.selected_agent_name();
        // The inject bar is a bottom panel inside the tab, so it stays visible
        // and wraps in a narrow detail panel; the screen scrolls above it.
        egui::Panel::bottom("screen_inject_bar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
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

        // Terminal lines are 80-200 columns: scroll, never wrap.
        egui::ScrollArea::both()
            .id_salt("screen_terminal_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(ref selected) = agent {
                    if let Some((ref tr_agent, _, ref text)) = self.screen_content {
                        if tr_agent == selected {
                            ui.add(egui::Label::new(RichText::new(text).monospace()).extend());
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
    }

    /// Tab row and content of the right detail panel; returns true when the
    /// user closed the panel.
    fn render_detail_tabs(
        &mut self,
        ui: &mut egui::Ui,
        visible: &std::collections::HashSet<NodeIndex>,
    ) -> bool {
        let mut close_panel = false;
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in DETAIL_TABS.iter().zip([
                "Summary",
                "Report (F9)",
                "Screen (F5)",
                "Transcript (F6)",
                "Events (F7)",
                "Log (F8)",
                "Legend",
            ]) {
                if ui
                    .selectable_label(self.state.detail_tab == *tab, label)
                    .clicked()
                {
                    self.select_detail_tab(tab);
                }
            }
            ui.separator();
            if ui.small_button("🔄 Refresh").clicked() {
                self.dispatch_action(ActionRequest::ReloadTableData);
                self.on_selection_changed();
            }
            if ui
                .small_button("✖")
                .on_hover_text("Close Detail Panel")
                .clicked()
            {
                close_panel = true;
            }
        });

        ui.separator();

        match self.state.detail_tab.as_str() {
            "Screen" => {
                self.render_screen_tab(ui);
            }
            "Transcript" => {
                let agent = self.selected_agent_name();
                egui::ScrollArea::both()
                    .id_salt("transcript_scroll_tab")
                    .auto_shrink([false, false])
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
                    .auto_shrink([false, false])
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
                    .auto_shrink([false, false])
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
                } else if let Some(node) = self
                    .canvas
                    .selected_node
                    .and_then(|idx| self.graph.graph.node_weight(idx))
                {
                    let row = TableRow::from_agent_node(node);
                    self.render_report_view(ui, &row);
                } else {
                    ui.weak("No agent selected");
                }
            }
            "Legend" => {
                let legend = crate::canvas::legend(
                    &self.graph,
                    &self.layout,
                    visible,
                    self.canvas.edge_counts.as_ref(),
                );
                egui::ScrollArea::vertical()
                    .id_salt("legend_scroll_tab")
                    .auto_shrink([false, false])
                    .show(ui, |ui| render_legend(ui, &legend));
            }
            _ => {
                // Scroll, so long content never overflows into the table below.
                egui::ScrollArea::vertical()
                    .id_salt("summary_scroll_tab")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.render_side_panel_summary(ui));
            }
        }
        close_panel
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

    /// Synchronizes nodes in `self.graph` from `self.table_rows` so catalog agents
    /// and running instances are both present in the graph model, preserving existing
    /// parent relations and spawn edges.
    pub fn sync_graph_from_table_rows(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }
        for r in &self.table_rows {
            let name = if !r.instance.is_empty() {
                &r.instance
            } else {
                &r.agent
            };
            if name.is_empty() {
                continue;
            }

            let status = match r.status.to_lowercase().as_str() {
                "active" => AgentStatus::Active,
                "listening" => AgentStatus::Listening,
                "blocked" => AgentStatus::Blocked,
                _ => AgentStatus::Stopped,
            };

            // If node already exists and has a parent_name from live sync,
            // do not overwrite it with None if the table row had None.
            let parent_name = if let Some(ref p) = r.parent_name {
                Some(p.clone())
            } else if let Some(idx) = self.graph.get_node_index(name) {
                self.graph
                    .graph
                    .node_weight(idx)
                    .and_then(|n| n.parent_name.clone())
            } else {
                None
            };

            let node = AgentNode {
                name: name.to_string(),
                status,
                status_context: if !r.detail.is_empty() {
                    Some(r.detail.clone())
                } else {
                    None
                },
                status_age_seconds: r.since_seconds.unwrap_or(0),
                unread_count: r.unread_count as u64,
                parent_name,
                directory: if !r.directory.is_empty() {
                    Some(r.directory.clone())
                } else {
                    None
                },
                tool: if !r.cli.is_empty() {
                    Some(r.cli.clone())
                } else {
                    None
                },
            };
            self.graph.upsert_agent(node);
        }
        self.layout.sync_nodes(&self.graph);
    }

    /// Projects a slice of table rows to the visible node indices in `self.graph`.
    pub fn visible_nodes_for_rows(
        &self,
        rows: &[TableRow],
    ) -> std::collections::HashSet<NodeIndex> {
        rows.iter()
            .filter_map(|r| {
                let name = if !r.instance.is_empty() {
                    &r.instance
                } else {
                    &r.agent
                };
                self.graph.get_node_index(name)
            })
            .collect()
    }

    /// True while a dialog, confirmation or the canvas context menu is open.
    pub fn overlay_open(&self) -> bool {
        self.context_menu.is_some()
            || self.confirm_kill_target.is_some()
            || self.confirm_restart_target.is_some()
            || self.confirm_kill_group.is_some()
            || self.custom_cmd_confirm.is_some()
            || self.show_start_dialog
            || self.show_send_dialog
    }

    /// Node whose hover tooltip shows this frame: the hovered node, unless the
    /// tooltip is off, a popup menu or dialog is open, or the pointer is pressed.
    pub fn tooltip_target(&self, popup_open: bool, pointer_down: bool) -> Option<NodeIndex> {
        if self.config.tooltip.is_empty()
            || popup_open
            || pointer_down
            || self.overlay_open()
            || self.canvas.dragged_node.is_some()
        {
            return None;
        }
        self.canvas.hovered_node
    }

    /// Tooltip header and lines for a node; a node without a table row
    /// (e.g. a placeholder parent) gets its name only. `dir` is shortened
    /// (see `shorten_path`).
    pub fn tooltip_content(&self, idx: NodeIndex) -> Option<(String, Vec<(String, String)>)> {
        let node = self.graph.graph.node_weight(idx)?;
        let mut lines = if self.table_rows.is_empty() {
            tooltip_lines(&TableRow::from_agent_node(node), &self.config.tooltip)
        } else {
            self.table_rows
                .iter()
                .find(|r| r.instance == node.name)
                .map(|r| tooltip_lines(r, &self.config.tooltip))
                .unwrap_or_default()
        };
        for (key, val) in &mut lines {
            if key == "dir" {
                *val = crate::table_data::shorten_path(val, self.home_dir.as_deref(), 48);
            }
        }
        Some((node.name.clone(), lines))
    }

    /// Restores persisted zoom, pan and pinned node positions (state is
    /// already sanitized by `load_state`). Nodes already in the layout (the
    /// initial sync runs first) move to their pin and are pinned.
    pub fn apply_persisted_view(&mut self) {
        if let Some(z) = self.state.canvas_zoom {
            self.canvas.zoom = z;
            self.check_saved_view = true;
        } else {
            self.auto_fit = true;
        }
        if let (Some(x), Some(y)) = (self.state.canvas_pan_x, self.state.canvas_pan_y) {
            self.canvas.pan = egui::vec2(x, y);
        }
        self.layout.pinned_positions = self
            .state
            .pinned_positions
            .iter()
            .map(|(name, p)| (name.clone(), egui::pos2(p.x, p.y)))
            .collect();
        let existing: Vec<(NodeIndex, egui::Pos2)> = self
            .graph
            .graph
            .node_indices()
            .filter_map(|idx| {
                let name = &self.graph.graph[idx].name;
                self.layout.pinned_positions.get(name).map(|p| (idx, *p))
            })
            .collect();
        for (idx, pos) in existing {
            self.layout.set_node_pos(idx, pos, true);
        }
    }

    /// Pins the node at its current position or releases its pin, and saves.
    pub fn toggle_pin(&mut self, idx: NodeIndex) {
        let Some(name) = self.graph.graph.node_weight(idx).map(|n| n.name.clone()) else {
            return;
        };
        let pin = !self.layout.pinned_positions.contains_key(&name);
        if !pin {
            self.layout.set_pinned(idx, &name, false);
            self.state.pinned_positions.remove(&name);
            self.save_state();
            return;
        }
        if self
            .layout
            .nodes
            .get(&idx)
            .is_some_and(|p| p.pos.x.is_finite() && p.pos.y.is_finite())
        {
            self.layout.set_pinned(idx, &name, true);
            self.store_pinned_position(idx);
        }
    }

    /// Stores the new position of a pinned node the user just dropped; a
    /// drag of an unpinned node stores nothing.
    pub fn record_dragged_position(&mut self, idx: NodeIndex) {
        let pinned = self
            .graph
            .graph
            .node_weight(idx)
            .is_some_and(|n| self.layout.pinned_positions.contains_key(&n.name));
        if pinned {
            self.store_pinned_position(idx);
        }
    }

    fn store_pinned_position(&mut self, idx: NodeIndex) {
        let (Some(node), Some(phys)) = (
            self.graph.graph.node_weight(idx),
            self.layout.nodes.get(&idx),
        ) else {
            return;
        };
        let (name, pos) = (node.name.clone(), phys.pos);
        if !(pos.x.is_finite() && pos.y.is_finite()) {
            return;
        }
        self.layout.pinned_positions.insert(name.clone(), pos);
        self.state.pinned_positions.insert(
            name,
            crate::config::SavedNodePos {
                x: pos.x,
                y: pos.y,
                saved_at: chrono::Utc::now().timestamp(),
            },
        );
        self.save_state();
    }

    pub fn mark_view_dirty(&mut self) {
        self.view_dirty_since = Some(Instant::now());
    }

    /// Saves pending view changes once they have been stable for 1 s and no
    /// pointer button is held. Returns true when it saved.
    pub fn flush_view_state(&mut self, now: Instant, pointer_down: bool) -> bool {
        match self.view_dirty_since {
            Some(t) if !pointer_down && now.duration_since(t) >= Duration::from_secs(1) => {
                self.save_state();
                self.view_dirty_since = None;
                true
            }
            _ => false,
        }
    }

    /// Applies the Fit button and auto-fit to `rect`, the canvas area of
    /// this frame. Auto-fit stays off in Manual mode and while a pointer
    /// button is held; the button fits in any mode.
    fn fit_canvas(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        visible: &std::collections::HashSet<NodeIndex>,
    ) {
        let items: Vec<(egui::Pos2, f32)> = self
            .layout
            .nodes
            .iter()
            .filter(|(idx, _)| visible.contains(idx))
            .filter_map(|(idx, phys)| {
                let name = &self.graph.graph.node_weight(*idx)?.name;
                let label_w = ui
                    .painter()
                    .layout_no_wrap(
                        name.clone(),
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    )
                    .size()
                    .x;
                Some((phys.pos, label_w))
            })
            .collect();
        if items.is_empty() {
            return;
        }
        if self.check_saved_view {
            self.check_saved_view = false;
            let in_view = items
                .iter()
                .any(|(p, _)| rect.contains(self.canvas.world_to_screen(*p, rect)));
            if !in_view {
                self.auto_fit = true;
            }
        }
        if self.fit_requested {
            self.fit_requested = false;
            // An explicit Fit is a user view choice: auto-fit must not undo it.
            self.auto_fit = false;
            self.canvas.fit_to(&items, rect, crate::canvas::ZOOM_MAX);
        } else if self.auto_fit
            && self.layout.mode != LayoutMode::Manual
            && !ui.input(|i| i.pointer.any_down())
        {
            self.canvas.fit_to(&items, rect, 1.0);
        }
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, visible: &std::collections::HashSet<NodeIndex>) {
        let rect = ui.available_rect_before_wrap();
        self.canvas_rect = rect;
        self.fit_canvas(ui, rect, visible);
        let dragged_before = self.canvas.dragged_node;
        let view_before = (self.canvas.zoom, self.canvas.pan);
        let (response, action) = self
            .canvas
            .show(ui, &self.graph, &mut self.layout, Some(visible));
        // The first user zoom, pan or node drag ends auto-fit.
        if view_before != (self.canvas.zoom, self.canvas.pan) || self.canvas.dragged_node.is_some()
        {
            self.auto_fit = false;
        }
        if let Some(idx) = dragged_before
            && self.canvas.dragged_node.is_none()
            && self.canvas.is_dragging
        {
            self.record_dragged_position(idx);
        }
        self.handle_canvas_action(action);
        self.render_node_tooltip(ui.ctx(), response.rect);
    }

    /// Hover card beside the hovered node (see `tooltip_placement`), kept
    /// inside the canvas; it switches immediately between nodes and fades
    /// out when the hover ends.
    fn render_node_tooltip(&mut self, ctx: &egui::Context, canvas_rect: egui::Rect) {
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        let shown = self.tooltip_target(egui::Popup::is_any_open(ctx), pointer_down);
        if let Some(idx) = shown {
            self.tooltip_node = Some(idx);
        }
        let opacity = ctx.animate_bool_with_time(
            egui::Id::unique("node_hover_tooltip"),
            shown.is_some(),
            0.15,
        );
        let Some(idx) = self.tooltip_node else {
            return;
        };
        let content = if opacity > 0.0 {
            self.tooltip_content(idx)
        } else {
            None
        };
        let (Some((name, lines)), Some(phys)) = (content, self.layout.nodes.get(&idx)) else {
            self.tooltip_node = None;
            return;
        };
        let card = |ui: &mut egui::Ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(360.0);
                ui.label(RichText::new(&name).strong());
                if lines.is_empty() {
                    return;
                }
                // A separator would take the full max width; the line is
                // drawn across the content width after the grid instead.
                let line_y = ui.cursor().top() + 3.0;
                ui.add_space(6.0);
                egui::Grid::new("node_hover_tooltip_grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .max_col_width(280.0)
                    .show(ui, |ui| {
                        for (key, val) in &lines {
                            ui.label(RichText::new(key).weak());
                            ui.add(egui::Label::new(val).truncate());
                            ui.end_row();
                        }
                    });
                ui.painter().hline(
                    ui.min_rect().x_range(),
                    line_y,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
            });
        };
        // Measure this frame's card in an invisible area first, so the
        // placement never uses a stale size (another node or older content).
        let size = egui::Area::new(egui::Id::unique("node_hover_tooltip_measure"))
            .order(egui::Order::Background)
            .interactable(false)
            .fixed_pos(canvas_rect.min)
            .show(ctx, |ui| {
                ui.set_invisible();
                card(ui);
            })
            .response
            .rect
            .size();
        let pos = crate::canvas::tooltip_placement(
            self.canvas.world_to_screen(phys.pos, canvas_rect),
            crate::canvas::node_extent(self.canvas.zoom),
            size,
            canvas_rect,
        );
        egui::Area::new(egui::Id::unique("node_hover_tooltip_area"))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .fixed_pos(pos)
            .constrain_to(canvas_rect)
            .show(ctx, |ui| {
                ui.multiply_opacity(opacity);
                card(ui);
            });
    }
}

impl eframe::App for HcomGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let window = ui.max_rect();
        self.adopt_legacy_split_sizes(window.size());

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

        // Keyboard shortcuts (F-keys act even while a text field has focus)
        let (clear_search, fkey) = ctx.input(|i| {
            let alt_x = i.modifiers.alt && i.key_pressed(egui::Key::X);
            let fkey = [
                egui::Key::F4,
                egui::Key::F5,
                egui::Key::F6,
                egui::Key::F7,
                egui::Key::F8,
                egui::Key::F9,
            ]
            .into_iter()
            .find(|k| i.key_pressed(*k));
            (alt_x, fkey)
        });

        if clear_search && !self.inject_input_has_focus {
            self.search_query.clear();
        }
        if let Some(key) = fkey {
            self.apply_function_key(key);
        }

        // 1. Ingest non-blocking updates from BackgroundSync
        if let Some(ref sync) = self.sync
            && let Some(updated) = sync.try_get_latest()
        {
            // Node styles come from table reloads, not from the sync snapshot.
            let styles = std::mem::take(&mut self.graph.node_styles);
            self.graph = updated;
            self.graph.node_styles = styles;
            self.sync_graph_from_table_rows();
            // Edge indices may be reused in another order after a swap.
            self.window_cache_key = None;
            self.layout.invalidate_caches();
            self.layout.sync_nodes(&self.graph);
            self.layout.wake_up();
            ctx.request_repaint();
        }

        self.poll_idle();

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
                        self.graph.node_styles = crate::table_data::node_styles_from_rows(&rows);
                        self.table_rows = rows;
                        self.sync_graph_from_table_rows();
                        self.update_idle(chrono::Utc::now().timestamp() as u64);
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

        let visible_node_indices = self.visible_nodes_for_rows(&filtered_rows);

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

                for r in &all_rows {
                    match r.status.to_lowercase().as_str() {
                        "active" => active_count += 1,
                        "listening" => listening_count += 1,
                        "blocked" => blocked_count += 1,
                        _ => stopped_count += 1,
                    }
                }

                ui.label(format!("Agents: {}", total_count));
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
                    if ui
                        .button("Fit")
                        .on_hover_text("Zoom and pan so all shown agents fit the graph")
                        .clicked()
                    {
                        self.fit_requested = true;
                    }
                    ui.label(format!("Zoom: {:.0}%", self.canvas.zoom * 100.0));
                });
            });

            ui.separator();

            // Row 2: Panel toggles + Search Filter + Checkboxes + Count
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.state.show_table, "Table (F4)")
                    .on_hover_text("Show or hide the agent table")
                    .clicked()
                {
                    self.toggle_table();
                }
                if ui
                    .selectable_label(self.state.show_side_panel, "Detail")
                    .on_hover_text("Show or hide the agent detail panel")
                    .clicked()
                {
                    self.toggle_detail_panel();
                }

                ui.separator();

                // Layout mode
                ui.label("Layout:");
                for (mode, label) in [
                    (LayoutMode::Hierarchy, "Hierarchy"),
                    (LayoutMode::Cluster, "Cluster"),
                    (LayoutMode::Manual, "Manual"),
                ] {
                    if ui
                        .selectable_label(self.layout.mode == mode, label)
                        .clicked()
                        && self.layout.mode != mode
                    {
                        self.layout.set_mode(mode);
                        self.state.layout_mode = mode.as_str().to_string();
                        self.save_state();
                    }
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

            // Row 2b: Message time window (presets, custom span/limit, scrubber, range)
            ui.horizontal_wrapped(|ui| {
                self.render_window_bar(ui);
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

        // 6. Bottom agent table across the full window width. Declared before
        // the right panel, so the detail panel and the canvas sit above it.
        // Both panels have an exact size from their window fraction (content
        // never resizes them); only the split handles below change a fraction.
        let mut table_rect = None;
        let table_avail = ui.available_height();
        if self.state.show_table {
            let table_h = split_px(
                self.state.table_height_frac.unwrap_or(DEFAULT_SPLIT_FRAC),
                window.height(),
                TABLE_PANEL_MIN_HEIGHT,
                table_avail,
                CANVAS_MIN_HEIGHT,
            );
            let table_resp = egui::Panel::bottom("agent_table_panel")
                .exact_size(table_h)
                .show(ui, |ui| {
                    self.render_table(ui, &filtered_rows);
                });
            table_rect = Some(table_resp.response.rect);
        }

        // 7. Right detail panel: all tabs of the selected agent
        let mut detail_rect = None;
        let detail_avail = ui.available_width();
        if self.state.show_side_panel {
            let mut close_panel = false;
            let detail_w = split_px(
                self.state.detail_width_frac.unwrap_or(DEFAULT_SPLIT_FRAC),
                window.width(),
                SIDE_PANEL_MIN_WIDTH,
                detail_avail,
                CANVAS_MIN_WIDTH,
            );
            let side_resp = egui::Panel::right("detail_side_panel")
                .exact_size(detail_w)
                .show(ui, |ui| {
                    ui.add_space(4.0);

                    // Error banner
                    let mut dismiss_err = false;
                    // Wrapped: a long unwrapped line would push the fixed-width panel off screen.
                    if let Some(ref err) = self.action_error {
                        ui.horizontal_wrapped(|ui| {
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

                    close_panel = self.render_detail_tabs(ui, &visible_node_indices);
                });
            detail_rect = Some(side_resp.response.rect);

            if close_panel {
                self.toggle_detail_panel();
            }
        }

        // 8. Central panel: the graph canvas
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_canvas(ui, &visible_node_indices);
        });

        // Split handles on the drawn panel edges, above every panel (the canvas
        // would otherwise take the drag and pan). Double-click resets.
        if window.height() >= 1.0
            && let Some(rect) = table_rect
            && let Some(change) = split_handle(ui, "table_split", rect, egui::Direction::BottomUp)
        {
            self.state.table_height_frac = match change {
                SplitChange::Moved(y) => Some(
                    split_px(
                        (window.bottom() - y) / window.height(),
                        window.height(),
                        TABLE_PANEL_MIN_HEIGHT,
                        table_avail,
                        CANVAS_MIN_HEIGHT,
                    ) / window.height(),
                ),
                SplitChange::Reset => None,
            };
            self.mark_view_dirty();
        }
        if window.width() >= 1.0
            && let Some(rect) = detail_rect
            && let Some(change) =
                split_handle(ui, "detail_split", rect, egui::Direction::RightToLeft)
        {
            self.state.detail_width_frac = match change {
                SplitChange::Moved(x) => Some(
                    split_px(
                        (window.right() - x) / window.width(),
                        window.width(),
                        SIDE_PANEL_MIN_WIDTH,
                        detail_avail,
                        CANVAS_MIN_WIDTH,
                    ) / window.width(),
                ),
                SplitChange::Reset => None,
            };
            self.mark_view_dirty();
        }

        // 9. Context Menu Popup
        let just_opened = self.context_menu_opened_this_frame;
        self.context_menu_opened_this_frame = false;

        if let Some(ref menu) = self.context_menu.clone() {
            let mut close_menu = false;
            #[allow(deprecated)]
            let area = egui::Area::new(egui::Id::new("canvas_context_menu"))
                .fixed_pos(menu.screen_pos)
                .order(egui::Order::Foreground);

            let inner = area.show(ui.ctx(), |ui| {
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
                            let pin_label =
                                if self.layout.pinned_positions.contains_key(&agent_name) {
                                    "Unpin Position"
                                } else {
                                    "Pin Position"
                                };
                            if ui.button(pin_label).clicked() {
                                self.toggle_pin(node_idx);
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

            let menu_rect = inner.response.rect;
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let clicked_outside = !just_opened
                && ui.input(|i| {
                    let any_click = i.pointer.primary_clicked() || i.pointer.secondary_clicked();
                    let pos = i.pointer.interact_pos().or_else(|| i.pointer.latest_pos());
                    any_click && pos.is_some_and(|p| !menu_rect.contains(p))
                });

            if close_menu || escape || clicked_outside {
                self.context_menu = None;
            }
        }

        // Zoom, pan and panel width are saved 1 s after the last change.
        let (zoom, pan) = (self.canvas.zoom, self.canvas.pan);
        if !approx_eq(self.state.canvas_zoom, zoom, 1e-3)
            || !approx_eq(self.state.canvas_pan_x, pan.x, 0.5)
            || !approx_eq(self.state.canvas_pan_y, pan.y, 0.5)
        {
            self.state.canvas_zoom = Some(zoom);
            self.state.canvas_pan_x = Some(pan.x);
            self.state.canvas_pan_y = Some(pan.y);
            self.mark_view_dirty();
        }
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        self.flush_view_state(Instant::now(), pointer_down);

        // S3: Periodic repaint so timer and refresh tick even when idle with no input
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn on_exit(&mut self) {
        if self.view_dirty_since.is_some() {
            self.save_state();
        }
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
                    self.context_menu_opened_this_frame = true;
                }
                CanvasAction::EdgeSecondaryClicked(idx, pos) => {
                    self.context_menu = Some(ContextMenuState {
                        target: ContextMenuTarget::Edge(idx),
                        screen_pos: pos,
                    });
                    self.context_menu_opened_this_frame = true;
                }
                CanvasAction::SelectionCleared => {
                    self.context_menu = None;
                }
            }
        }
    }
}

/// True when a persisted value exists and lies within `tol` of `current`
/// (sub-pixel float noise must not mark the view dirty every frame).
pub fn approx_eq(saved: Option<f32>, current: f32, tol: f32) -> bool {
    saved.is_some_and(|s| (s - current).abs() <= tol)
}

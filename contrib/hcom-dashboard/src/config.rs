use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWithAction {
    pub label: String,
    pub cmd: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub if_exists: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    pub label: String,
    pub cmd: Vec<String>,
    #[serde(default)]
    pub confirm: Option<bool>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ScreenBarDef {
    Builtin(String),
    Custom { label: String, keys: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenBarButton {
    pub label: String,
    pub keys: String,
    pub is_inject_dialog: bool,
    pub is_enter_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    #[serde(default = "default_refresh_seconds")]
    pub refresh_seconds: u64,
    #[serde(default = "default_stats_seconds")]
    pub stats_seconds: u64,
    #[serde(default = "default_idle_seconds")]
    pub idle_seconds: u64,
    #[serde(default)]
    pub open_with: Vec<OpenWithAction>,
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
    #[serde(default)]
    pub toolbar_breaks: Vec<String>,
    #[serde(default)]
    pub attach: HashMap<String, Vec<String>>,
    #[serde(default = "default_screen_bar")]
    pub screen_bar: Vec<ScreenBarDef>,
    #[serde(default = "default_report")]
    pub report: Vec<String>,
    /// Fields of the node hover tooltip (same keys as `report`); empty = off.
    #[serde(default = "default_tooltip")]
    pub tooltip: Vec<String>,
}

pub fn default_tooltip() -> Vec<String> {
    [
        "status", "detail", "cli", "since", "age", "dir", "unread", "context",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_refresh_seconds() -> u64 {
    5
}

fn default_stats_seconds() -> u64 {
    60
}

fn default_idle_seconds() -> u64 {
    10
}

fn default_screen_bar() -> Vec<ScreenBarDef> {
    vec![
        ScreenBarDef::Builtin("Inject".into()),
        ScreenBarDef::Builtin("Enter".into()),
        ScreenBarDef::Builtin("Esc".into()),
        ScreenBarDef::Builtin("↑".into()),
        ScreenBarDef::Builtin("↓".into()),
        ScreenBarDef::Builtin("←".into()),
        ScreenBarDef::Builtin("→".into()),
        ScreenBarDef::Builtin("Tab".into()),
        ScreenBarDef::Builtin("Shift+Tab".into()),
        ScreenBarDef::Builtin("1".into()),
        ScreenBarDef::Builtin("2".into()),
        ScreenBarDef::Builtin("3".into()),
    ]
}

pub fn default_report() -> Vec<String> {
    vec![
        "status:status".into(),
        "status:done".into(),
        "status:issues".into(),
        "status:next".into(),
        "status".into(),
        "idle".into(),
        "since".into(),
        "age".into(),
        "agent".into(),
        "instance".into(),
        "roaming".into(),
        "cli".into(),
        "context".into(),
        "quota".into(),
        "detail".into(),
        "unread".into(),
        "dir".into(),
        "branch".into(),
        "git".into(),
        "where".into(),
        "config".into(),
        "starts".into(),
        "runtime".into(),
        "handoffs".into(),
        "msgs".into(),
        "last_start".into(),
        "stops".into(),
        "failed".into(),
    ]
}

pub fn default_columns() -> Vec<String> {
    vec![
        "agent".into(),
        "instance".into(),
        "roaming".into(),
        "cli".into(),
        "context".into(),
        "quota".into(),
        "age".into(),
        "since".into(),
        "status".into(),
        "idle".into(),
        "status:status".into(),
        "status:done".into(),
        "detail".into(),
        "unread".into(),
        "dir".into(),
        "branch".into(),
        "git".into(),
    ]
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            refresh_seconds: default_refresh_seconds(),
            stats_seconds: default_stats_seconds(),
            idle_seconds: default_idle_seconds(),
            open_with: Vec::new(),
            commands: Vec::new(),
            toolbar_breaks: Vec::new(),
            attach: HashMap::new(),
            screen_bar: default_screen_bar(),
            report: default_report(),
            tooltip: default_tooltip(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    #[default]
    Both,
    GraphOnly,
    TableOnly,
}

fn deserialize_view_mode<'de, D>(deserializer: D) -> Result<Option<ViewMode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum RawViewMode {
        Both,
        GraphOnly,
        TableOnly,
        #[serde(other)]
        Other,
    }

    let raw = Option::<RawViewMode>::deserialize(deserializer)?;
    Ok(match raw {
        Some(RawViewMode::Both) => Some(ViewMode::Both),
        Some(RawViewMode::GraphOnly) => Some(ViewMode::GraphOnly),
        Some(RawViewMode::TableOnly) => Some(ViewMode::TableOnly),
        Some(RawViewMode::Other) | None => None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Dark,
    Light,
}

impl ThemePreference {
    /// Resolves user preference against the OS theme.
    /// If Auto and system_theme is None (e.g. headless or unsupported OS),
    /// deterministically falls back to `ResolvedTheme::Dark`.
    pub fn resolve(&self, system_theme: Option<egui::Theme>) -> ResolvedTheme {
        match self {
            Self::Light => ResolvedTheme::Light,
            Self::Dark => ResolvedTheme::Dark,
            Self::Auto => match system_theme {
                Some(egui::Theme::Light) => ResolvedTheme::Light,
                Some(egui::Theme::Dark) | None => ResolvedTheme::Dark,
            },
        }
    }
}

/// Desktop config directory: `$XDG_CONFIG_HOME` (absolute), else `~/.config`.
pub fn desktop_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })
}

/// Light/dark theme read from the desktop's config files, for desktops whose
/// theme eframe does not report (e.g. LXQt on X11): the LXQt palette (used by
/// Qt apps), then the GTK 3 settings (used by GTK apps).
pub fn desktop_theme(config_dir: &Path) -> Option<egui::Theme> {
    let read = |rel: &str| fs::read_to_string(config_dir.join(rel)).ok();
    read("lxqt/lxqt.conf")
        .and_then(|s| lxqt_palette_theme(&s))
        .or_else(|| read("gtk-3.0/settings.ini").and_then(|s| gtk_settings_theme(&s)))
}

/// Value of `key` in `[section]` of an INI text, trimmed and unquoted.
fn ini_value<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut in_section = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line == format!("[{section}]");
        } else if in_section
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().trim_matches('"'));
        }
    }
    None
}

/// Theme from the luminance of `window_color` in the LXQt `[Palette]`.
pub fn lxqt_palette_theme(text: &str) -> Option<egui::Theme> {
    let hex = ini_value(text, "Palette", "window_color")?.strip_prefix('#')?;
    // Qt writes #rrggbb or #aarrggbb.
    let rgb = hex.get(hex.len().checked_sub(6)?..)?;
    let channel = |i: usize| u8::from_str_radix(rgb.get(i..i + 2)?, 16).ok();
    let (r, g, b) = (channel(0)?, channel(2)?, channel(4)?);
    let luminance = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
    Some(if luminance < 128.0 {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    })
}

/// Theme from GTK 3 `settings.ini`: `gtk-application-prefer-dark-theme` or a
/// theme name containing "dark" means dark, any other named theme light.
pub fn gtk_settings_theme(text: &str) -> Option<egui::Theme> {
    let prefer_dark = ini_value(text, "Settings", "gtk-application-prefer-dark-theme");
    if matches!(prefer_dark, Some("true" | "1")) {
        return Some(egui::Theme::Dark);
    }
    let name = ini_value(text, "Settings", "gtk-theme-name").filter(|n| !n.is_empty())?;
    Some(if name.to_lowercase().contains("dark") {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    })
}

pub fn deserialize_theme_preference<'de, D>(
    deserializer: D,
) -> Result<Option<ThemePreference>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum RawTheme {
        Auto,
        Light,
        Dark,
        #[serde(other)]
        Other,
    }

    let raw = Option::<RawTheme>::deserialize(deserializer)?;
    Ok(match raw {
        Some(RawTheme::Auto) => Some(ThemePreference::Auto),
        Some(RawTheme::Light) => Some(ThemePreference::Light),
        Some(RawTheme::Dark) => Some(ThemePreference::Dark),
        Some(RawTheme::Other) | None => None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardState {
    #[serde(default)]
    pub geometry: Option<String>,
    /// Table height as a fraction of the window height.
    #[serde(default)]
    pub table_height_frac: Option<f32>,
    /// Detail panel width in pixels.
    #[serde(default)]
    pub detail_width_px: Option<f32>,
    /// Active panel layout mode: both graph and table, graph only, or table only.
    #[serde(default, deserialize_with = "deserialize_view_mode")]
    pub view_mode: Option<ViewMode>,
    /// Visual theme preference (auto, light, dark).
    #[serde(default, deserialize_with = "deserialize_theme_preference")]
    pub theme: Option<ThemePreference>,
    #[serde(default = "default_true")]
    pub show_table: bool,
    #[serde(default)]
    pub running_only: bool,
    #[serde(default)]
    pub blocked_only: bool,
    #[serde(default = "default_true")]
    pub hide_disabled: bool,
    #[serde(default)]
    pub hide_adhoc: bool,
    #[serde(default = "default_true")]
    pub empty_last: bool,
    #[serde(default = "default_sort_col")]
    pub sort_col: Option<String>,
    #[serde(default)]
    pub sort_reverse: bool,
    /// Active tab of the right detail panel (one of `DETAIL_TABS`).
    #[serde(default)]
    pub detail_tab: String,
    #[serde(default = "default_true")]
    pub show_side_panel: bool,
    #[serde(default = "default_columns")]
    pub columns: Vec<String>,
    #[serde(default)]
    pub columns_known: Vec<String>,
    #[serde(default)]
    pub column_widths: HashMap<String, f32>,
    /// Message time window span in seconds; `None`, 0 or negative = All.
    #[serde(default)]
    pub window_span_secs: Option<i64>,
    /// Keep only the newest N messages in the window; `None` or 0 = off.
    #[serde(default)]
    pub window_last_n: Option<usize>,
    /// Layout mode: "hierarchy", "cluster" (default) or "manual".
    #[serde(default = "default_layout_mode")]
    pub layout_mode: String,
    /// World positions of user-pinned nodes by instance name; invalid entries
    /// are dropped on load so one bad entry cannot reject the file.
    #[serde(default, deserialize_with = "deserialize_pinned_positions")]
    pub pinned_positions: HashMap<String, SavedNodePos>,
    #[serde(default)]
    pub canvas_zoom: Option<f32>,
    #[serde(default)]
    pub canvas_pan_x: Option<f32>,
    #[serde(default)]
    pub canvas_pan_y: Option<f32>,
    /// Edge layers drawn on the canvas: initiator arrows, messages, spawn.
    #[serde(default = "default_true")]
    pub show_initiator_edges: bool,
    #[serde(default = "default_true")]
    pub show_message_edges: bool,
    #[serde(default = "default_true")]
    pub show_spawn_edges: bool,
    /// Message arrow width by text bytes instead of message count.
    #[serde(default)]
    pub edge_width_bytes: bool,
}

/// A saved node position and when it was last stored (unix seconds).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SavedNodePos {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub saved_at: i64,
}

/// Most saved node positions kept; the oldest are dropped first.
pub const MAX_NODE_POSITIONS: usize = 500;
/// Default width of the detail side panel in pixels.
pub const DEFAULT_DETAIL_WIDTH_PX: f32 = 380.0;
/// Minimum width of the detail side panel in pixels.
pub const SIDE_PANEL_MIN_WIDTH: f32 = 260.0;
/// Range a saved split fraction is clamped to.
pub const SPLIT_FRAC_MIN: f32 = 0.05;
pub const SPLIT_FRAC_MAX: f32 = 0.95;

fn deserialize_pinned_positions<'de, D>(d: D) -> Result<HashMap<String, SavedNodePos>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<HashMap<String, serde_json::Value>>::deserialize(d)?.unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| {
            serde_json::from_value::<SavedNodePos>(v)
                .ok()
                .map(|p| (k, p))
        })
        .collect())
}

impl DashboardState {
    /// Drops or clamps values that cannot be used (non-finite numbers,
    /// zoom outside the canvas range, split fractions outside (0, 1), unknown detail
    /// tab) and keeps only
    /// the `MAX_NODE_POSITIONS` newest node positions.
    pub fn sanitize(&mut self) {
        use crate::canvas::{ZOOM_MAX, ZOOM_MIN};
        self.pinned_positions
            .retain(|_, p| p.x.is_finite() && p.y.is_finite());
        if self.pinned_positions.len() > MAX_NODE_POSITIONS {
            let mut by_age: Vec<(i64, String)> = self
                .pinned_positions
                .iter()
                .map(|(k, p)| (p.saved_at, k.clone()))
                .collect();
            by_age.sort();
            let excess = by_age.len() - MAX_NODE_POSITIONS;
            for (_, k) in by_age.into_iter().take(excess) {
                self.pinned_positions.remove(&k);
            }
        }
        self.canvas_zoom = self
            .canvas_zoom
            .filter(|z| z.is_finite())
            .map(|z| z.clamp(ZOOM_MIN, ZOOM_MAX));
        if !(self.canvas_pan_x.is_some_and(f32::is_finite)
            && self.canvas_pan_y.is_some_and(f32::is_finite))
        {
            self.canvas_pan_x = None;
            self.canvas_pan_y = None;
        }
        self.detail_width_px = self
            .detail_width_px
            .filter(|w| w.is_finite() && *w > 0.0)
            .map(|w| w.max(SIDE_PANEL_MIN_WIDTH));
        self.table_height_frac = self
            .table_height_frac
            .filter(|f| f.is_finite() && *f > 0.0 && *f < 1.0)
            .map(|f| f.clamp(SPLIT_FRAC_MIN, SPLIT_FRAC_MAX));
        if !DETAIL_TABS.contains(&self.detail_tab.as_str()) {
            self.detail_tab = DEFAULT_DETAIL_TAB.into();
        }
    }

    /// Persisted window settings as a live window (end follows now);
    /// invalid values (span <= 0, last_n == 0) mean no limit.
    pub fn message_window(&self) -> crate::model::MessageWindow {
        crate::model::MessageWindow {
            span_secs: self.window_span_secs.filter(|s| *s > 0),
            end: None,
            last_n: self.window_last_n.filter(|n| *n > 0),
        }
    }

    pub fn effective_view_mode(&self) -> ViewMode {
        if !self.show_table {
            return ViewMode::GraphOnly;
        }
        match self.view_mode {
            Some(ViewMode::TableOnly) => ViewMode::TableOnly,
            Some(ViewMode::GraphOnly) => ViewMode::GraphOnly,
            Some(ViewMode::Both) | None => ViewMode::Both,
        }
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = Some(mode);
        self.show_table = mode != ViewMode::GraphOnly;
    }
}

fn default_layout_mode() -> String {
    "cluster".into()
}

fn default_true() -> bool {
    true
}

fn default_sort_col() -> Option<String> {
    Some("instance".into())
}

/// Tabs of the right detail panel, in display order.
pub const DETAIL_TABS: [&str; 8] = [
    "Summary",
    "Report",
    "Screen",
    "Transcript",
    "Events",
    "Log",
    "Config",
    "Legend",
];

pub const DEFAULT_DETAIL_TAB: &str = "Summary";

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            geometry: None,
            table_height_frac: None,
            detail_width_px: None,
            view_mode: Some(ViewMode::Both),
            theme: None,
            show_table: true,
            running_only: false,
            blocked_only: false,
            hide_disabled: true,
            hide_adhoc: false,
            empty_last: true,
            sort_col: default_sort_col(),
            sort_reverse: false,
            detail_tab: DEFAULT_DETAIL_TAB.into(),
            show_side_panel: true,
            columns: default_columns(),
            columns_known: Vec::new(),
            column_widths: HashMap::new(),
            window_span_secs: None,
            window_last_n: None,
            layout_mode: default_layout_mode(),
            pinned_positions: HashMap::new(),
            canvas_zoom: None,
            canvas_pan_x: None,
            canvas_pan_y: None,
            show_initiator_edges: true,
            show_message_edges: true,
            show_spawn_edges: true,
            edge_width_bytes: false,
        }
    }
}

pub fn load_config(hcom_dir: &Path) -> DashboardConfig {
    let path = hcom_dir.join("dashboard.json");
    if path.is_file() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<DashboardConfig>(&content) {
                Ok(cfg) => return cfg,
                Err(e) => eprintln!("Warning: failed to parse dashboard.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read dashboard.json: {}", e),
        }
    }
    DashboardConfig::default()
}

pub fn load_state(hcom_dir: &Path) -> DashboardState {
    let mut state = read_state(hcom_dir);
    state.sanitize();
    state
}

fn read_state(hcom_dir: &Path) -> DashboardState {
    let path = hcom_dir.join("dashboard-state.json");
    if path.is_file() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<DashboardState>(&content) {
                Ok(state) => return state,
                Err(e) => eprintln!("Warning: failed to parse dashboard-state.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read dashboard-state.json: {}", e),
        }
    }
    DashboardState::default()
}

/// Saves state to dashboard-state.json.
/// The state is sanitized first (a NaN would be written as null and reject the
/// file on the next load); the temp file is unique per process and save, so
/// concurrent windows never rename each other's partial file (last writer wins).
pub fn save_state(hcom_dir: &Path, state: &DashboardState) -> Result<()> {
    static SAVE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut state = state.clone();
    state.sanitize();
    let target = hcom_dir.join("dashboard-state.json");
    let seq = SAVE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = hcom_dir.join(format!(
        "dashboard-state.json.{}.{}.tmp",
        std::process::id(),
        seq
    ));
    let json = serde_json::to_string_pretty(&state)?;
    if let Err(e) = fs::write(&tmp, json).and_then(|_| fs::rename(&tmp, &target)) {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct PlaceholderContext<'a> {
    pub dir: &'a str,
    pub instance: &'a str,
    pub agent: &'a str,
    pub terminal: &'a str,
    pub pane_id: &'a str,
    pub herdr_workspace: &'a str,
    pub herdr_tab: &'a str,
}

pub fn replace_placeholders_str(
    text: &str,
    ctx: &PlaceholderContext<'_>,
) -> Result<String, String> {
    let result = text
        .replace("{dir}", ctx.dir)
        .replace("{instance}", ctx.instance)
        .replace("{agent}", ctx.agent)
        .replace("{terminal}", ctx.terminal)
        .replace("{pane_id}", ctx.pane_id)
        .replace("{herdr_workspace}", ctx.herdr_workspace)
        .replace("{herdr_tab}", ctx.herdr_tab);

    if let Some(start) = result.find('{')
        && let Some(end) = result[start..].find('}')
    {
        let unknown = &result[start..=start + end];
        return Err(format!("Unknown placeholder in command: {}", unknown));
    }
    Ok(result)
}

pub fn replace_placeholders_vec(
    tokens: &[String],
    ctx: &PlaceholderContext<'_>,
) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        out.push(replace_placeholders_str(token, ctx)?);
    }
    Ok(out)
}

pub fn screen_bar_buttons(defs: &[ScreenBarDef]) -> Vec<ScreenBarButton> {
    defs.iter()
        .filter_map(|def| match def {
            ScreenBarDef::Builtin(name) => match name.as_str() {
                "Inject" => Some(ScreenBarButton {
                    label: "Inject".into(),
                    keys: String::new(),
                    is_inject_dialog: true,
                    is_enter_flag: false,
                }),
                "Enter" => Some(ScreenBarButton {
                    label: "Enter".into(),
                    keys: String::new(),
                    is_inject_dialog: false,
                    is_enter_flag: true,
                }),
                "Esc" => Some(ScreenBarButton {
                    label: "Esc".into(),
                    keys: "\x1b".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "↑" => Some(ScreenBarButton {
                    label: "↑".into(),
                    keys: "\x1b[A".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "↓" => Some(ScreenBarButton {
                    label: "↓".into(),
                    keys: "\x1b[B".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "←" => Some(ScreenBarButton {
                    label: "←".into(),
                    keys: "\x1b[D".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "→" => Some(ScreenBarButton {
                    label: "→".into(),
                    keys: "\x1b[C".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "Tab" => Some(ScreenBarButton {
                    label: "Tab".into(),
                    keys: "\t".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "Shift+Tab" => Some(ScreenBarButton {
                    label: "Shift+Tab".into(),
                    keys: "\x1b[Z".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "1" => Some(ScreenBarButton {
                    label: "1".into(),
                    keys: "1".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "2" => Some(ScreenBarButton {
                    label: "2".into(),
                    keys: "2".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                "3" => Some(ScreenBarButton {
                    label: "3".into(),
                    keys: "3".into(),
                    is_inject_dialog: false,
                    is_enter_flag: false,
                }),
                _ => None,
            },
            ScreenBarDef::Custom { label, keys } => Some(ScreenBarButton {
                label: label.clone(),
                keys: keys.clone(),
                is_inject_dialog: false,
                is_enter_flag: false,
            }),
        })
        .collect()
}

pub fn get_hcom_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HCOM_DIR") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".hcom")
    } else {
        PathBuf::from(".hcom")
    }
}

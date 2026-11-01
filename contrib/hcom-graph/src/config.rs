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
pub struct GraphConfig {
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

impl Default for GraphConfig {
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphState {
    #[serde(default)]
    pub geometry: Option<String>,
    #[serde(default)]
    pub sash_pos: Option<f32>,
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
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
    #[serde(default = "default_bottom_tab")]
    pub bottom_tab: String,
    #[serde(default = "default_true")]
    pub show_side_panel: bool,
    #[serde(default)]
    pub main_split_ratio: Option<f32>,
    #[serde(default = "default_columns")]
    pub columns: Vec<String>,
    #[serde(default)]
    pub columns_known: Vec<String>,
    #[serde(default)]
    pub column_widths: HashMap<String, f32>,
}

fn default_view_mode() -> String {
    "split".into()
}

fn default_true() -> bool {
    true
}

fn default_sort_col() -> Option<String> {
    Some("instance".into())
}

fn default_bottom_tab() -> String {
    "Screen".into()
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            geometry: None,
            sash_pos: None,
            view_mode: default_view_mode(),
            running_only: false,
            blocked_only: false,
            hide_disabled: true,
            hide_adhoc: false,
            empty_last: true,
            sort_col: default_sort_col(),
            sort_reverse: false,
            bottom_tab: default_bottom_tab(),
            show_side_panel: true,
            main_split_ratio: None,
            columns: default_columns(),
            columns_known: Vec::new(),
            column_widths: HashMap::new(),
        }
    }
}

pub fn load_config(hcom_dir: &Path) -> GraphConfig {
    let graph_json = hcom_dir.join("graph.json");
    if graph_json.is_file() {
        match fs::read_to_string(&graph_json) {
            Ok(content) => match serde_json::from_str::<GraphConfig>(&content) {
                Ok(cfg) => return cfg,
                Err(e) => eprintln!("Warning: failed to parse graph.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read graph.json: {}", e),
        }
    }

    let gui_json = hcom_dir.join("gui.json");
    if gui_json.is_file() {
        match fs::read_to_string(&gui_json) {
            Ok(content) => match serde_json::from_str::<GraphConfig>(&content) {
                Ok(cfg) => return cfg,
                Err(e) => eprintln!("Warning: failed to parse gui.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read gui.json: {}", e),
        }
    }

    GraphConfig::default()
}

pub fn load_state(hcom_dir: &Path) -> GraphState {
    let graph_state_json = hcom_dir.join("graph-state.json");
    if graph_state_json.is_file() {
        match fs::read_to_string(&graph_state_json) {
            Ok(content) => match serde_json::from_str::<GraphState>(&content) {
                Ok(state) => return state,
                Err(e) => eprintln!("Warning: failed to parse graph-state.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read graph-state.json: {}", e),
        }
    }

    let gui_state_json = hcom_dir.join("gui-state.json");
    if gui_state_json.is_file() {
        match fs::read_to_string(&gui_state_json) {
            Ok(content) => match serde_json::from_str::<GraphState>(&content) {
                Ok(state) => return state,
                Err(e) => eprintln!("Warning: failed to parse gui-state.json: {}", e),
            },
            Err(e) => eprintln!("Warning: failed to read gui-state.json: {}", e),
        }
    }

    GraphState::default()
}

/// Saves state strictly to graph-state.json without ever mutating gui-state.json
pub fn save_state(hcom_dir: &Path, state: &GraphState) -> Result<()> {
    let target = hcom_dir.join("graph-state.json");
    let tmp = hcom_dir.join("graph-state.json.tmp");
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&tmp, json)?;
    fs::rename(tmp, target)?;
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

//! `hcom agent` — named agent catalog.
//!
//! Deliberately isolated from the rest of the codebase: everything it needs
//! from hcom goes through the public CLI surface (`hcom list --json`,
//! `hcom <tool>`, `hcom r`) invoked on our own binary, plus
//! `crate::terminal::which_bin`. No launcher internals are touched, so this
//! module stays conflict-free across upstream rebases.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use serde::Deserialize;

const GLOBAL_FILE: &str = "agents.json";
const PROJECT_FILE: &str = ".hcom-agents.json";
const DEFAULT_CLI: &str = "claude";
const MAX_WALK_UP: usize = 40;

// ── CLI entry ───────────────────────────────────────────────────────────

#[derive(clap::Parser, Debug)]
#[command(name = "agent", about = "Launch named agents from a JSON catalog")]
pub struct AgentArgs {
    /// Agent name or subcommand, plus flags forwarded to hcom
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub fn help_text() -> String {
    format!(
        "Usage:
  hcom agent <name> [flags] [tool-args...]   Launch a catalog agent (no-op if already running)
  hcom agent ls [--json] [--names]           Catalog entries with live status and source
  hcom agent show <name>                     Effective config and the exact command
  hcom agent attach <name>                   Focus a running agent's window
  hcom agent edit [--project]                Open a catalog in $EDITOR (creates a starter file)
  hcom agent completions bash|zsh|fish       Shell completion for agent names

Catalog (later layers win; env and args merge, scalars replace):
  1. built-in defaults (cli={DEFAULT_CLI})
  2. \"defaults\" in ~/.hcom/{GLOBAL_FILE}          (override path: HCOM_AGENTS_FILE)
  3. agent entry in ~/.hcom/{GLOBAL_FILE}
  4. \"defaults\" in ./{PROJECT_FILE}               (nearest one up to the git root)
  5. agent entry in ./{PROJECT_FILE}
  6. command-line flags

  Relative \"dir\" resolves against $HOME for the global catalog and against the
  catalog's own directory for a project catalog. ~ and $VAR are expanded.

Flags:
  --cli <tool>              claude | codex | gemini | ... (default: {DEFAULT_CLI})
  --dir <path>              Working directory
  --terminal <preset>       hcom terminal preset, or \"here\"
  --terminal-command <cmd>  Raw terminal command with {{script}} (passed via HCOM_TERMINAL)
  --session <name>          Multiplexer session (tmux); empty string disables
  --window <name>           Window name inside the session (default: agent name)
  --tag / --model <val>     Forwarded to hcom / the tool
  --prompt / --system-prompt <text>
  --pre <cmd>               Shell command run in the window before the agent
  --env K=V                 Extra environment variable (repeatable)
  --catalog <path>          Use this file instead of the project catalog
  --no-project              Ignore any {PROJECT_FILE}
  --attach                  Focus the window after launching (or when already running)
  --restart                 Kill a running agent first instead of reporting it
  --fresh                   Start a new session instead of resuming a stopped one
  --dry-run                 Print the commands without running anything

  Any other flag is forwarded verbatim to `hcom <cli>`.

Terminal strategy, in order:
  1. terminal_command set          -> hcom launches with HCOM_TERMINAL=<cmd>
  2. session set + tmux available  -> window is prepared here, agent runs with --terminal here
  3. otherwise                     -> hcom opens the window itself via --terminal <preset>

A named agent is unique: launching one that already runs prints its status and exits 0."
    )
}

pub fn cmd_agent(args: &AgentArgs) -> i32 {
    match run(&args.args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    }
}

fn run(argv: &[String]) -> Result<i32> {
    let Some(first) = argv.first().map(|s| s.as_str()) else {
        println!("{}", help_text());
        return Ok(0);
    };

    match first {
        "ls" | "list" => cmd_ls(&argv[1..]),
        "show" => cmd_show(&argv[1..]),
        "attach" => cmd_attach(&argv[1..]),
        "edit" => cmd_edit(&argv[1..]),
        "completions" => cmd_completions(&argv[1..]),
        _ if first.starts_with('-') => {
            println!("{}", help_text());
            Ok(0)
        }
        name => cmd_launch(name, &argv[1..]),
    }
}

// ── Catalog model ───────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentDef {
    dir: Option<String>,
    cli: Option<String>,
    terminal: Option<String>,
    terminal_command: Option<String>,
    session: Option<String>,
    window: Option<String>,
    tag: Option<String>,
    model: Option<String>,
    prompt: Option<String>,
    system_prompt: Option<String>,
    pre: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    args: Vec<String>,
}

impl AgentDef {
    fn merge_from(&mut self, over: &AgentDef) {
        macro_rules! take {
            ($($f:ident),+ $(,)?) => {$(
                if over.$f.is_some() {
                    self.$f = over.$f.clone();
                }
            )+};
        }
        take!(
            dir,
            cli,
            terminal,
            terminal_command,
            session,
            window,
            tag,
            model,
            prompt,
            system_prompt,
            pre
        );
        for (k, v) in &over.env {
            self.env.insert(k.clone(), v.clone());
        }
        self.args.extend(over.args.iter().cloned());
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    #[serde(default)]
    #[allow(dead_code)]
    version: Option<u32>,
    #[serde(default)]
    defaults: AgentDef,
    #[serde(default)]
    agents: BTreeMap<String, AgentDef>,
}

struct CatalogFile {
    path: PathBuf,
    label: String,
    catalog: Catalog,
}

/// Expand `~` and `$VAR`, then absolutize against `base`.
fn expand_path(raw: &str, base: &Path) -> String {
    let expanded = expand_vars(raw);
    let expanded = if let Some(rest) = expanded.strip_prefix("~/") {
        match home_dir() {
            Some(home) => home.join(rest).to_string_lossy().into_owned(),
            None => expanded,
        }
    } else if expanded == "~" {
        home_dir()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or(expanded)
    } else {
        expanded
    };

    let p = PathBuf::from(&expanded);
    if p.is_absolute() {
        expanded
    } else {
        normalize(&base.join(p)).to_string_lossy().into_owned()
    }
}

/// Drop `.` and resolve `..` lexically (no filesystem access, no symlink
/// resolution — the directory may not exist yet).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn expand_vars(raw: &str) -> String {
    let bytes: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '$' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let (name, next) = if bytes.get(i + 1) == Some(&'{') {
            match bytes[i + 2..].iter().position(|c| *c == '}') {
                Some(end) => (
                    bytes[i + 2..i + 2 + end].iter().collect::<String>(),
                    i + 3 + end,
                ),
                None => (String::new(), i + 1),
            }
        } else {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                j += 1;
            }
            (bytes[i + 1..j].iter().collect::<String>(), j)
        };
        if name.is_empty() {
            out.push('$');
            i += 1;
            continue;
        }
        out.push_str(&std::env::var(&name).unwrap_or_default());
        i = next;
    }
    out
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn load_catalog_file(path: &Path, base: &Path, label: String) -> Result<CatalogFile> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let mut catalog: Catalog = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid JSON in {}: {e}", path.display()))?;

    // Resolve `dir` while we still know which file it came from.
    if let Some(dir) = catalog.defaults.dir.take() {
        catalog.defaults.dir = Some(expand_path(&dir, base));
    }
    for def in catalog.agents.values_mut() {
        if let Some(dir) = def.dir.take() {
            def.dir = Some(expand_path(&dir, base));
        }
    }
    Ok(CatalogFile {
        path: path.to_path_buf(),
        label,
        catalog,
    })
}

fn global_catalog_path() -> PathBuf {
    match std::env::var_os("HCOM_AGENTS_FILE") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => crate::paths::hcom_dir().join(GLOBAL_FILE),
    }
}

/// Nearest `.hcom-agents.json` walking up from `start`; stops after the
/// directory that holds `.git`.
fn find_project_catalog(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    for _ in 0..MAX_WALK_UP {
        let d = dir?;
        let candidate = d.join(PROJECT_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if d.join(".git").exists() {
            return None;
        }
        dir = d.parent();
    }
    None
}

struct Catalogs {
    files: Vec<CatalogFile>,
}

impl Catalogs {
    fn load(no_project: bool, explicit: Option<&Path>) -> Result<Self> {
        let mut files = Vec::new();

        let global = global_catalog_path();
        if global.is_file() {
            let base = home_dir().unwrap_or_else(|| PathBuf::from("."));
            files.push(load_catalog_file(&global, &base, "global".to_string())?);
        }

        let project = match explicit {
            Some(p) => {
                if !p.is_file() {
                    bail!("catalog not found: {}", p.display());
                }
                Some(p.to_path_buf())
            }
            None if no_project => None,
            None => std::env::current_dir()
                .ok()
                .and_then(|cwd| find_project_catalog(&cwd)),
        };
        if let Some(path) = project {
            let base = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            files.push(load_catalog_file(&path, &base, "project".to_string())?);
        }

        Ok(Self { files })
    }

    /// Names in catalog order, mapped to the label of the last file defining them.
    fn names(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for file in &self.files {
            for name in file.catalog.agents.keys() {
                out.entry(name.clone())
                    .and_modify(|l: &mut String| *l = format!("{l}+{}", file.label))
                    .or_insert_with(|| file.label.clone());
            }
        }
        out
    }

    fn resolve(&self, name: &str) -> Option<AgentDef> {
        if !self
            .files
            .iter()
            .any(|f| f.catalog.agents.contains_key(name))
        {
            return None;
        }
        // A catalog's `defaults` only apply to agents that same catalog defines,
        // so a project catalog cannot silently retune unrelated global agents.
        // Adding an empty entry (`"name": {}`) opts an inherited agent in.
        let mut def = AgentDef::default();
        for file in &self.files {
            if let Some(entry) = file.catalog.agents.get(name) {
                def.merge_from(&file.catalog.defaults);
                def.merge_from(entry);
            }
        }
        Some(def)
    }

    fn paths(&self) -> Vec<&Path> {
        self.files.iter().map(|f| f.path.as_path()).collect()
    }
}

fn unknown_agent_error(name: &str, catalogs: &Catalogs) -> anyhow::Error {
    let names = catalogs.names();
    let mut msg = format!("unknown agent '{name}'");
    if let Some(close) = closest(name, names.keys()) {
        msg.push_str(&format!(" (did you mean '{close}'?)"));
    }
    if names.is_empty() {
        let paths = catalogs.paths();
        if paths.is_empty() {
            msg.push_str(&format!(
                "\nNo catalog found. Create {} or run `hcom agent edit`.",
                global_catalog_path().display()
            ));
        } else {
            msg.push_str("\nCatalog is empty: ");
            msg.push_str(
                &paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    } else {
        msg.push_str("\nKnown agents: ");
        msg.push_str(&names.keys().cloned().collect::<Vec<_>>().join(", "));
    }
    anyhow::anyhow!(msg)
}

fn closest<'a, I: Iterator<Item = &'a String>>(target: &str, candidates: I) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        let d = levenshtein(target, c);
        if d <= target.len().div_ceil(2)
            && best.as_ref().is_none_or(|(bd, _)| d < *bd)
        {
            best = Some((d, c.clone()));
        }
    }
    best.map(|(_, name)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

// ── Command-line overrides ──────────────────────────────────────────────

#[derive(Default)]
struct Cli {
    def: AgentDef,
    attach: bool,
    dry_run: bool,
    restart: bool,
    fresh: bool,
    no_project: bool,
    catalog: Option<PathBuf>,
    passthrough: Vec<String>,
}

fn parse_cli(argv: &[String]) -> Result<Cli> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        if arg == "--" {
            cli.passthrough.extend(argv[i + 1..].iter().cloned());
            break;
        }
        let value = || -> Result<String> {
            argv.get(i + 1)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))
        };
        match arg {
            "--cli" | "--tool" => {
                cli.def.cli = Some(value()?);
                i += 2;
            }
            "--dir" => {
                cli.def.dir = Some(expand_path(
                    &value()?,
                    &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                ));
                i += 2;
            }
            "--terminal" => {
                cli.def.terminal = Some(value()?);
                i += 2;
            }
            "--terminal-command" => {
                cli.def.terminal_command = Some(value()?);
                i += 2;
            }
            "--session" => {
                cli.def.session = Some(value()?);
                i += 2;
            }
            "--window" => {
                cli.def.window = Some(value()?);
                i += 2;
            }
            "--tag" => {
                cli.def.tag = Some(value()?);
                i += 2;
            }
            "--model" => {
                cli.def.model = Some(value()?);
                i += 2;
            }
            "--prompt" | "--hcom-prompt" => {
                cli.def.prompt = Some(value()?);
                i += 2;
            }
            "--system-prompt" | "--hcom-system-prompt" => {
                cli.def.system_prompt = Some(value()?);
                i += 2;
            }
            "--pre" => {
                cli.def.pre = Some(value()?);
                i += 2;
            }
            "--env" => {
                let kv = value()?;
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("--env expects KEY=VALUE, got '{kv}'"))?;
                cli.def.env.insert(k.to_string(), v.to_string());
                i += 2;
            }
            "--catalog" => {
                cli.catalog = Some(PathBuf::from(value()?));
                i += 2;
            }
            "--no-project" => {
                cli.no_project = true;
                i += 1;
            }
            "--attach" => {
                cli.attach = true;
                i += 1;
            }
            "--restart" => {
                cli.restart = true;
                i += 1;
            }
            "--fresh" => {
                cli.fresh = true;
                i += 1;
            }
            "--dry-run" => {
                cli.dry_run = true;
                i += 1;
            }
            other => {
                cli.passthrough.push(other.to_string());
                i += 1;
            }
        }
    }
    Ok(cli)
}

// ── Effective configuration ─────────────────────────────────────────────

struct Effective {
    name: String,
    cli: String,
    dir: String,
    window: String,
    session: Option<String>,
    terminal: Option<String>,
    terminal_command: Option<String>,
    pre: Option<String>,
    tag: Option<String>,
    model: Option<String>,
    prompt: Option<String>,
    system_prompt: Option<String>,
    env: BTreeMap<String, String>,
    extra: Vec<String>,
}

fn nonempty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.trim().is_empty())
}

fn effective(name: &str, mut def: AgentDef, cli: &Cli) -> Effective {
    def.merge_from(&cli.def);
    let mut extra = std::mem::take(&mut def.args);
    extra.extend(cli.passthrough.iter().cloned());

    Effective {
        name: name.to_string(),
        cli: nonempty(def.cli).unwrap_or_else(|| DEFAULT_CLI.to_string()),
        dir: nonempty(def.dir).unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".to_string())
        }),
        window: nonempty(def.window).unwrap_or_else(|| name.to_string()),
        session: nonempty(def.session),
        terminal: nonempty(def.terminal),
        terminal_command: nonempty(def.terminal_command),
        pre: nonempty(def.pre),
        tag: nonempty(def.tag),
        model: nonempty(def.model),
        prompt: nonempty(def.prompt),
        system_prompt: nonempty(def.system_prompt),
        env: def.env,
        extra,
    }
}

// ── Terminal strategy ───────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
enum Strategy {
    /// Raw terminal command handed to hcom through HCOM_TERMINAL.
    Custom(String),
    /// Named tmux session/window prepared here; agent runs with `--terminal here`.
    Tmux { session: String, window: String },
    /// hcom opens the window itself.
    Direct(Option<String>),
}

fn choose_strategy(eff: &Effective, tmux_available: bool) -> (Strategy, Vec<String>) {
    let mut warnings = Vec::new();

    if let Some(cmd) = &eff.terminal_command {
        if eff.session.is_some() {
            warnings.push("session ignored: terminal_command takes precedence".to_string());
        }
        return (Strategy::Custom(cmd.clone()), warnings);
    }

    if let Some(session) = &eff.session {
        let mux_ok = eff
            .terminal
            .as_deref()
            .is_none_or(|t| t.starts_with("tmux"));
        if !mux_ok {
            warnings.push(format!(
                "session '{session}' ignored: terminal preset '{}' is not a multiplexer",
                eff.terminal.as_deref().unwrap_or("")
            ));
        } else if !tmux_available {
            warnings.push(format!("session '{session}' ignored: tmux not found on PATH"));
        } else {
            return (
                Strategy::Tmux {
                    session: session.clone(),
                    window: eff.window.clone(),
                },
                warnings,
            );
        }
    }

    (Strategy::Direct(eff.terminal.clone()), warnings)
}

// ── Command construction ────────────────────────────────────────────────

fn hcom_bin() -> String {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hcom".to_string())
}

/// Arguments for a fresh launch (`resume = false`) or `hcom r` (`resume = true`).
fn hcom_argv(eff: &Effective, terminal: Option<&str>, resume: bool) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if resume {
        v.push("r".into());
        v.push(eff.name.clone());
    } else {
        v.push(eff.cli.clone());
        v.push("--as".into());
        v.push(eff.name.clone());
    }
    v.push("--dir".into());
    v.push(eff.dir.clone());
    if let Some(t) = terminal {
        v.push("--terminal".into());
        v.push(t.to_string());
    }
    if let Some(tag) = &eff.tag {
        v.push("--tag".into());
        v.push(tag.clone());
    }
    if let Some(p) = &eff.prompt {
        v.push("--hcom-prompt".into());
        v.push(p.clone());
    }
    if let Some(s) = &eff.system_prompt {
        v.push("--hcom-system-prompt".into());
        v.push(s.clone());
    }
    if resume {
        v.push("--go".into());
    }
    // Tool-level args last.
    if let Some(m) = &eff.model {
        v.push("--model".into());
        v.push(m.clone());
    }
    v.extend(eff.extra.iter().cloned());
    v
}

/// Shell line executed inside a multiplexer window.
fn window_command(eff: &Effective, resume: bool) -> String {
    let mut parts: Vec<String> = eff
        .env
        .iter()
        .map(|(k, v)| format!("{k}={}", shell_words::quote(v)))
        .collect();
    let mut argv = vec![hcom_bin()];
    argv.extend(hcom_argv(eff, Some("here"), resume));
    parts.push(shell_words::join(argv.iter().map(String::as_str)));

    let mut line = parts.join(" ");
    if let Some(pre) = &eff.pre {
        line = format!("{pre} && {line}");
    }
    // Keep the pane alive once the agent exits.
    format!("{line}; exec \"${{SHELL:-sh}}\"")
}

// ── Live state ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LiveAgent {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    tool: String,
    #[serde(default)]
    directory: String,
    #[serde(default)]
    launch_context: serde_json::Value,
}

impl LiveAgent {
    fn pane_id(&self) -> Option<&str> {
        self.launch_context.get("pane_id")?.as_str()
    }
}

fn live_agents() -> Vec<LiveAgent> {
    let out = Command::new(hcom_bin())
        .args(["list", "--json"])
        .stderr(Stdio::null())
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    serde_json::from_slice(&out.stdout).unwrap_or_default()
}

fn find_live(name: &str) -> Option<LiveAgent> {
    live_agents().into_iter().find(|a| a.name == name)
}

fn has_stopped_history(name: &str) -> bool {
    Command::new(hcom_bin())
        .args(["list", "--stopped", name])
        .stderr(Stdio::null())
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).starts_with("Stopped:"))
        .unwrap_or(false)
}

// ── tmux backend ────────────────────────────────────────────────────────

fn tmux_bin() -> Option<String> {
    crate::terminal::which_bin("tmux")
}

fn tmux_ok(args: &[&str]) -> bool {
    let Some(bin) = tmux_bin() else {
        return false;
    };
    Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tmux_capture(args: &[&str]) -> Option<String> {
    let bin = tmux_bin()?;
    let out = Command::new(bin).args(args).stderr(Stdio::null()).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn tmux_run(args: &[String], dry_run: bool) -> Result<()> {
    if dry_run {
        let mut shown = vec!["tmux".to_string()];
        shown.extend(args.iter().cloned());
        println!("{}", shell_words::join(shown.iter().map(String::as_str)));
        return Ok(());
    }
    let bin = tmux_bin().ok_or_else(|| anyhow::anyhow!("tmux not found on PATH"))?;
    let status = Command::new(bin).args(args).status()?;
    if !status.success() {
        bail!("tmux {} failed", args.first().cloned().unwrap_or_default());
    }
    Ok(())
}

fn tmux_window_exists(session: &str, window: &str) -> bool {
    tmux_capture(&["list-windows", "-t", &format!("={session}"), "-F", "#{window_name}"])
        .map(|out| out.lines().any(|l| l == window))
        .unwrap_or(false)
}

/// Create the session (with the agent as its first window) or add a window to it.
fn tmux_launch(
    session: &str,
    window: &str,
    dir: &str,
    command: &str,
    dry_run: bool,
) -> Result<()> {
    let session_exists = !dry_run && tmux_ok(&["has-session", "-t", &format!("={session}")]);

    let args: Vec<String> = if !session_exists {
        vec![
            "new-session".into(),
            "-d".into(),
            "-s".into(),
            session.into(),
            "-n".into(),
            window.into(),
            "-c".into(),
            dir.into(),
            command.into(),
        ]
    } else if tmux_window_exists(session, window) {
        // Window left over from a previous run; reuse it instead of stacking duplicates.
        vec![
            "respawn-window".into(),
            "-k".into(),
            "-t".into(),
            format!("{session}:{window}"),
            "-c".into(),
            dir.into(),
            command.into(),
        ]
    } else {
        vec![
            "new-window".into(),
            "-d".into(),
            "-t".into(),
            format!("{session}:"),
            "-n".into(),
            window.into(),
            "-c".into(),
            dir.into(),
            command.into(),
        ]
    };
    tmux_run(&args, dry_run)
}

/// Focus a window, either by tmux target or by a recorded pane id.
fn tmux_focus(target: &str) -> Result<()> {
    if tmux_bin().is_none() {
        bail!("tmux not found on PATH");
    }
    tmux_run(
        &["select-window".to_string(), "-t".to_string(), target.to_string()],
        false,
    )?;
    let inside = std::env::var_os("TMUX").is_some();
    let verb = if inside { "switch-client" } else { "attach-session" };
    tmux_run(
        &[verb.to_string(), "-t".to_string(), target.to_string()],
        false,
    )
}

fn focus_running(eff: &Effective, live: &LiveAgent) -> Result<()> {
    if let Some(pane) = live.pane_id() {
        return tmux_focus(pane);
    }
    match &eff.session {
        Some(session) => tmux_focus(&format!("{session}:{}", eff.window)),
        None => bail!(
            "no pane recorded for '{}' — nothing to attach to",
            eff.name
        ),
    }
}

// ── Subcommand: launch ──────────────────────────────────────────────────

fn cmd_launch(name: &str, rest: &[String]) -> Result<i32> {
    let cli = parse_cli(rest)?;
    let catalogs = Catalogs::load(cli.no_project, cli.catalog.as_deref())?;
    let def = catalogs
        .resolve(name)
        .ok_or_else(|| unknown_agent_error(name, &catalogs))?;
    let eff = effective(name, def, &cli);

    let live = find_live(name);
    if let Some(live) = &live {
        if cli.restart {
            if cli.dry_run {
                println!("{} kill {}", hcom_bin(), name);
            } else {
                let _ = Command::new(hcom_bin()).args(["kill", name]).status();
            }
        } else {
            let where_ = match &eff.session {
                Some(s) => format!(", tmux {s}:{}", eff.window),
                None => String::new(),
            };
            println!(
                "agent '{name}' already running ({}, {}, {}{where_})",
                live.tool,
                live.status,
                shorten_home(&live.directory)
            );
            if cli.attach {
                focus_running(&eff, live)?;
            }
            return Ok(0);
        }
    }

    let resume = live.is_none() && !cli.fresh && has_stopped_history(name);
    launch(&eff, &cli, resume)
}

fn launch(eff: &Effective, cli: &Cli, resume: bool) -> Result<i32> {
    let (strategy, warnings) = choose_strategy(eff, tmux_bin().is_some());
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    if resume {
        println!(
            "resuming '{}' ({}) — use --fresh to start a new session instead",
            eff.name, eff.cli
        );
    }

    match strategy {
        Strategy::Tmux { session, window } => {
            let command = window_command(eff, resume);
            tmux_launch(&session, &window, &eff.dir, &command, cli.dry_run)?;
            if !cli.dry_run {
                println!(
                    "launched '{}' ({}) in tmux {session}:{window} [{}]",
                    eff.name, eff.cli, eff.dir
                );
                if cli.attach {
                    tmux_focus(&format!("{session}:{window}"))?;
                }
            }
            Ok(0)
        }
        Strategy::Custom(term_cmd) => {
            let argv = hcom_argv(eff, None, resume);
            run_hcom(eff, &argv, Some(&term_cmd), cli.dry_run)
        }
        Strategy::Direct(terminal) => {
            let argv = hcom_argv(eff, terminal.as_deref(), resume);
            run_hcom(eff, &argv, None, cli.dry_run)
        }
    }
}

fn run_hcom(
    eff: &Effective,
    argv: &[String],
    terminal_command: Option<&str>,
    dry_run: bool,
) -> Result<i32> {
    if dry_run {
        let mut prefix: Vec<String> = eff
            .env
            .iter()
            .map(|(k, v)| format!("{k}={}", shell_words::quote(v)))
            .collect();
        if let Some(tc) = terminal_command {
            prefix.push(format!("HCOM_TERMINAL={}", shell_words::quote(tc)));
        }
        let mut shown = vec![hcom_bin()];
        shown.extend(argv.iter().cloned());
        let line = shell_words::join(shown.iter().map(String::as_str));
        println!(
            "{}{line}",
            if prefix.is_empty() {
                String::new()
            } else {
                format!("{} ", prefix.join(" "))
            }
        );
        return Ok(0);
    }

    let mut cmd = Command::new(hcom_bin());
    cmd.args(argv);
    for (k, v) in &eff.env {
        cmd.env(k, v);
    }
    if let Some(tc) = terminal_command {
        cmd.env("HCOM_TERMINAL", tc);
    }
    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}

// ── Subcommand: ls / show / attach / edit / completions ──────────────────

fn cmd_ls(rest: &[String]) -> Result<i32> {
    let cli = parse_cli(rest)?;
    let json = cli.passthrough.iter().any(|a| a == "--json");
    let names_only = cli.passthrough.iter().any(|a| a == "--names");
    let catalogs = Catalogs::load(cli.no_project, cli.catalog.as_deref())?;
    let names = catalogs.names();

    if names_only {
        for name in names.keys() {
            println!("{name}");
        }
        return Ok(0);
    }

    if names.is_empty() {
        println!(
            "No agents defined. Create {} or run `hcom agent edit`.",
            global_catalog_path().display()
        );
        return Ok(0);
    }

    let live: BTreeMap<String, LiveAgent> =
        live_agents().into_iter().map(|a| (a.name.clone(), a)).collect();

    if json {
        let mut out = Vec::new();
        for (name, source) in &names {
            let def = catalogs.resolve(name).unwrap_or_default();
            let eff = effective(name, def, &Cli::default());
            out.push(serde_json::json!({
                "name": name,
                "source": source,
                "cli": eff.cli,
                "dir": eff.dir,
                "session": eff.session,
                "window": eff.window,
                "terminal": eff.terminal,
                "status": live.get(name).map(|a| a.status.clone()),
            }));
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(0);
    }

    let rows: Vec<[String; 5]> = names
        .iter()
        .map(|(name, source)| {
            let def = catalogs.resolve(name).unwrap_or_default();
            let eff = effective(name, def, &Cli::default());
            let where_ = match (&eff.session, &eff.terminal) {
                (Some(s), _) => format!("{s}:{}", eff.window),
                (None, Some(t)) => t.clone(),
                (None, None) => "-".to_string(),
            };
            let status = match live.get(name) {
                Some(a) => format!("● {}", a.status),
                None => "○ -".to_string(),
            };
            [
                name.clone(),
                eff.cli,
                where_,
                shorten_home(&eff.dir),
                format!("{status}  [{source}]"),
            ]
        })
        .collect();

    let headers = ["NAME", "CLI", "WHERE", "DIR", "STATUS"];
    let mut widths = headers.map(str::len);
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let render = |cells: &[String; 5]| {
        let mut line = String::new();
        for (i, cell) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{:<w$}  ", cell, w = widths[i]));
            }
        }
        line
    };
    println!("{}", render(&headers.map(String::from)));
    for row in &rows {
        println!("{}", render(row));
    }
    Ok(0)
}

fn shorten_home(path: &str) -> String {
    match home_dir() {
        Some(home) => {
            let home = home.to_string_lossy();
            match path.strip_prefix(home.as_ref()) {
                Some(rest) => format!("~{rest}"),
                None => path.to_string(),
            }
        }
        None => path.to_string(),
    }
}

fn cmd_show(rest: &[String]) -> Result<i32> {
    let Some(name) = rest.first().filter(|a| !a.starts_with('-')).cloned() else {
        bail!("Usage: hcom agent show <name>");
    };
    let cli = parse_cli(&rest[1..])?;
    let catalogs = Catalogs::load(cli.no_project, cli.catalog.as_deref())?;
    let def = catalogs
        .resolve(&name)
        .ok_or_else(|| unknown_agent_error(&name, &catalogs))?;
    let eff = effective(&name, def, &cli);
    let (strategy, warnings) = choose_strategy(&eff, tmux_bin().is_some());

    println!("name:      {}", eff.name);
    println!("cli:       {}", eff.cli);
    println!("dir:       {}", eff.dir);
    match &strategy {
        Strategy::Tmux { session, window } => println!("terminal:  tmux {session}:{window} (--terminal here)"),
        Strategy::Custom(c) => println!("terminal:  HCOM_TERMINAL={c}"),
        Strategy::Direct(Some(t)) => println!("terminal:  preset {t}"),
        Strategy::Direct(None) => println!("terminal:  hcom default (hcom config terminal)"),
    }
    if let Some(tag) = &eff.tag {
        println!("tag:       {tag}");
    }
    if let Some(m) = &eff.model {
        println!("model:     {m}");
    }
    if let Some(p) = &eff.pre {
        println!("pre:       {p}");
    }
    if !eff.env.is_empty() {
        println!("env:       {}", eff
            .env
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" "));
    }
    if !eff.extra.is_empty() {
        println!("args:      {}", shell_words::join(eff.extra.iter().map(String::as_str)));
    }
    println!("sources:   {}", catalogs
        .paths()
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", "));
    for w in warnings {
        println!("warning:   {w}");
    }
    println!();
    println!("command:");
    match &strategy {
        Strategy::Tmux { .. } => println!("  {}", window_command(&eff, false)),
        Strategy::Custom(_) | Strategy::Direct(_) => {
            let terminal = match &strategy {
                Strategy::Direct(t) => t.clone(),
                _ => None,
            };
            let mut shown = vec![hcom_bin()];
            shown.extend(hcom_argv(&eff, terminal.as_deref(), false));
            println!("  {}", shell_words::join(shown.iter().map(String::as_str)));
        }
    }
    Ok(0)
}

fn cmd_attach(rest: &[String]) -> Result<i32> {
    let Some(name) = rest.first().filter(|a| !a.starts_with('-')).cloned() else {
        bail!("Usage: hcom agent attach <name>");
    };
    let cli = parse_cli(&rest[1..])?;
    let catalogs = Catalogs::load(cli.no_project, cli.catalog.as_deref())?;
    let def = catalogs.resolve(&name).unwrap_or_default();
    let eff = effective(&name, def, &cli);
    match find_live(&name) {
        Some(live) => {
            focus_running(&eff, &live)?;
            Ok(0)
        }
        None => {
            eprintln!("agent '{name}' is not running");
            Ok(1)
        }
    }
}

const STARTER: &str = r#"{
  "version": 1,
  "defaults": {
    "cli": "claude"
  },
  "agents": {
    "example": {
      "dir": "~/projects/example",
      "cli": "codex",
      "session": "example"
    }
  }
}
"#;

fn cmd_edit(rest: &[String]) -> Result<i32> {
    let project = rest.iter().any(|a| a == "--project");
    let path = if project {
        std::env::current_dir()?.join(PROJECT_FILE)
    } else {
        global_catalog_path()
    };
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, STARTER)?;
        println!("created {}", path.display());
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let parts = shell_words::split(&editor)?;
    let Some((bin, args)) = parts.split_first() else {
        bail!("empty $EDITOR");
    };
    let status = Command::new(bin).args(args).arg(&path).status()?;
    Ok(status.code().unwrap_or(1))
}

fn cmd_completions(rest: &[String]) -> Result<i32> {
    let shell = rest.first().map(String::as_str).unwrap_or("bash");
    let script = match shell {
        "bash" => {
            "_hcom_agent() {\n  local cur=${COMP_WORDS[COMP_CWORD]}\n  if [ \"$COMP_CWORD\" -eq 2 ]; then\n    COMPREPLY=( $(compgen -W \"$(hcom agent ls --names 2>/dev/null) ls show attach edit completions\" -- \"$cur\") )\n  fi\n}\ncomplete -F _hcom_agent hcom\n"
        }
        "zsh" => {
            "#compdef hcom\n_hcom_agent_names() {\n  local -a names\n  names=(${(f)\"$(hcom agent ls --names 2>/dev/null)\"})\n  compadd -a names\n}\ncompdef _hcom_agent_names hcom-agent\n"
        }
        "fish" => {
            "complete -c hcom -n '__fish_seen_subcommand_from agent' -f -a \"(hcom agent ls --names 2>/dev/null)\"\n"
        }
        other => bail!("unsupported shell '{other}' (bash | zsh | fish)"),
    };
    print!("{script}");
    Ok(0)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn def_from(json: &str) -> AgentDef {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn merge_replaces_scalars_and_accumulates_env_and_args() {
        let mut base = def_from(r#"{"cli":"claude","env":{"A":"1"},"args":["--x"]}"#);
        let over = def_from(r#"{"cli":"codex","env":{"B":"2"},"args":["--y"]}"#);
        base.merge_from(&over);
        assert_eq!(base.cli.as_deref(), Some("codex"));
        assert_eq!(base.env.get("A").map(String::as_str), Some("1"));
        assert_eq!(base.env.get("B").map(String::as_str), Some("2"));
        assert_eq!(base.args, vec!["--x", "--y"]);
    }

    #[test]
    fn merge_keeps_base_when_override_omits_field() {
        let mut base = def_from(r#"{"session":"wdt"}"#);
        base.merge_from(&def_from(r#"{"cli":"codex"}"#));
        assert_eq!(base.session.as_deref(), Some("wdt"));
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = serde_json::from_str::<AgentDef>(r#"{"drr":"typo"}"#);
        assert!(err.is_err());
    }

    fn catalogs_from(pairs: &[(&str, &str, &str)]) -> Catalogs {
        // (label, base_dir, json)
        Catalogs {
            files: pairs
                .iter()
                .map(|(label, base, json)| {
                    let mut catalog: Catalog = serde_json::from_str(json).unwrap();
                    let base = PathBuf::from(base);
                    if let Some(dir) = catalog.defaults.dir.take() {
                        catalog.defaults.dir = Some(expand_path(&dir, &base));
                    }
                    for d in catalog.agents.values_mut() {
                        if let Some(dir) = d.dir.take() {
                            d.dir = Some(expand_path(&dir, &base));
                        }
                    }
                    CatalogFile {
                        path: base.join("catalog.json"),
                        label: label.to_string(),
                        catalog,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn project_layer_overrides_global_and_can_add_agents() {
        let c = catalogs_from(&[
            (
                "global",
                "/home/u",
                r#"{"defaults":{"cli":"claude"},"agents":{"a":{"dir":"/g","model":"opus"}}}"#,
            ),
            (
                "project",
                "/repo",
                r#"{"defaults":{"cli":"codex"},"agents":{"a":{"model":"gpt-5"},"b":{"dir":"."}}}"#,
            ),
        ]);
        let a = c.resolve("a").unwrap();
        assert_eq!(a.cli.as_deref(), Some("codex"), "project defaults win");
        // An agent the project catalog never mentions keeps the global defaults.
        let c2 = catalogs_from(&[
            ("global", "/home/u", r#"{"defaults":{"cli":"claude"},"agents":{"only_global":{}}}"#),
            ("project", "/repo", r#"{"defaults":{"cli":"codex"},"agents":{"other":{}}}"#),
        ]);
        assert_eq!(c2.resolve("only_global").unwrap().cli.as_deref(), Some("claude"));
        assert_eq!(a.model.as_deref(), Some("gpt-5"));
        assert_eq!(a.dir.as_deref(), Some("/g"), "global dir survives");

        let b = c.resolve("b").unwrap();
        assert_eq!(b.dir.as_deref(), Some("/repo"), "project dir is relative to the catalog");
        assert!(c.resolve("missing").is_none());
        assert_eq!(c.names().get("a").map(String::as_str), Some("global+project"));
    }

    #[test]
    fn relative_dir_resolves_against_its_own_catalog_base() {
        assert_eq!(expand_path("sub/x", Path::new("/base")), "/base/sub/x");
        assert_eq!(expand_path("/abs", Path::new("/base")), "/abs");
        assert_eq!(expand_path("./a/../b", Path::new("/base")), "/base/b");
    }

    #[test]
    fn expand_vars_substitutes_known_and_drops_unknown() {
        unsafe { std::env::set_var("HCOM_TEST_VAR", "val") };
        assert_eq!(expand_vars("x/$HCOM_TEST_VAR/y"), "x/val/y");
        assert_eq!(expand_vars("x/${HCOM_TEST_VAR}/y"), "x/val/y");
        assert_eq!(expand_vars("$HCOM_TEST_MISSING_VAR/y"), "/y");
        assert_eq!(expand_vars("100% $"), "100% $");
    }

    fn eff_of(json: &str, cli_args: &[&str]) -> Effective {
        let cli = parse_cli(&cli_args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        effective("wdt_main", def_from(json), &cli)
    }

    #[test]
    fn cli_flags_win_over_catalog_and_unknown_flags_pass_through() {
        let eff = eff_of(
            r#"{"cli":"claude","dir":"/w","args":["--from-catalog"]}"#,
            &["--cli", "codex", "--model", "gpt-5", "--dangerously-skip-permissions"],
        );
        assert_eq!(eff.cli, "codex");
        assert_eq!(eff.model.as_deref(), Some("gpt-5"));
        assert_eq!(eff.extra, vec!["--from-catalog", "--dangerously-skip-permissions"]);
    }

    #[test]
    fn double_dash_forwards_everything_verbatim() {
        let eff = eff_of(r#"{"dir":"/w"}"#, &["--", "--cli", "not-consumed"]);
        assert_eq!(eff.cli, DEFAULT_CLI);
        assert_eq!(eff.extra, vec!["--cli", "not-consumed"]);
    }

    #[test]
    fn window_defaults_to_agent_name_and_empty_session_disables_mux() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &[]);
        assert_eq!(eff.window, "wdt_main");
        assert_eq!(eff.session.as_deref(), Some("wdt"));

        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &["--session", ""]);
        assert!(eff.session.is_none());
    }

    #[test]
    fn strategy_prefers_terminal_command_then_mux_then_preset() {
        let eff = eff_of(r#"{"dir":"/w","terminal_command":"myterm {script}"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true);
        assert_eq!(s, Strategy::Custom("myterm {script}".into()));

        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &[]);
        let (s, w) = choose_strategy(&eff, true);
        assert_eq!(
            s,
            Strategy::Tmux {
                session: "wdt".into(),
                window: "wdt_main".into()
            }
        );
        assert!(w.is_empty());

        let eff = eff_of(r#"{"dir":"/w","terminal":"wezterm-tab"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true);
        assert_eq!(s, Strategy::Direct(Some("wezterm-tab".into())));

        let eff = eff_of(r#"{"dir":"/w"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true);
        assert_eq!(s, Strategy::Direct(None));
    }

    #[test]
    fn session_falls_back_to_preset_without_tmux() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt","terminal":"tmux"}"#, &[]);
        let (s, warnings) = choose_strategy(&eff, false);
        assert_eq!(s, Strategy::Direct(Some("tmux".into())));
        assert!(warnings[0].contains("tmux not found"));
    }

    #[test]
    fn session_with_non_mux_preset_warns_and_falls_back() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt","terminal":"kitty-tab"}"#, &[]);
        let (s, warnings) = choose_strategy(&eff, true);
        assert_eq!(s, Strategy::Direct(Some("kitty-tab".into())));
        assert!(warnings[0].contains("not a multiplexer"));
    }

    #[test]
    fn hcom_argv_puts_hcom_flags_before_tool_args() {
        let eff = eff_of(
            r#"{"dir":"/w","cli":"codex","tag":"wdt","prompt":"hi","args":["--foo"]}"#,
            &["--model", "gpt-5"],
        );
        let argv = hcom_argv(&eff, Some("here"), false);
        assert_eq!(
            argv,
            vec![
                "codex",
                "--as",
                "wdt_main",
                "--dir",
                "/w",
                "--terminal",
                "here",
                "--tag",
                "wdt",
                "--hcom-prompt",
                "hi",
                "--model",
                "gpt-5",
                "--foo",
            ]
        );
    }

    #[test]
    fn resume_argv_uses_r_and_go() {
        let eff = eff_of(r#"{"dir":"/w","cli":"codex"}"#, &[]);
        let argv = hcom_argv(&eff, Some("here"), true);
        assert_eq!(argv[0], "r");
        assert_eq!(argv[1], "wdt_main");
        assert!(argv.contains(&"--go".to_string()));
        assert!(!argv.contains(&"--as".to_string()));
    }

    #[test]
    fn window_command_carries_env_pre_and_keeps_pane_alive() {
        let eff = eff_of(
            r#"{"dir":"/w","env":{"AWS_PROFILE":"wdt"},"pre":"source .venv/bin/activate"}"#,
            &[],
        );
        let cmd = window_command(&eff, false);
        assert!(cmd.starts_with("source .venv/bin/activate && AWS_PROFILE=wdt "));
        assert!(cmd.contains("--terminal here"));
        assert!(cmd.ends_with("; exec \"${SHELL:-sh}\""));
    }

    #[test]
    fn closest_suggests_near_miss_only() {
        let names = ["wdt_main".to_string(), "gtm_cli".to_string()];
        assert_eq!(closest("wdt_mian", names.iter()).as_deref(), Some("wdt_main"));
        assert_eq!(closest("zzzzzzzzzz", names.iter()), None);
    }

    #[test]
    fn help_starts_with_usage() {
        assert!(help_text().starts_with("Usage:"));
    }
}

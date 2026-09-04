//! `hcom agent` — named agent catalog.
//!
//! Catalog loading and named-agent lifecycle commands.

use std::collections::{BTreeMap, BTreeSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::db::HcomDb;

const GLOBAL_FILE: &str = "agents.json";
const PROJECT_DIR: &str = ".hcom";
const PROJECT_FILE: &str = "agents.json";
const EXTRA_CATALOGS_ENV: &str = "HCOM_AGENT_CATALOGS";
const DEFAULT_CLI: &str = "claude";

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
  hcom agent @<group> [flags]                Launch every agent in a catalog group
  hcom agent list [@<group>] [--all] [--local] [--json] [--names|--groups]
                 [--for-agents|--for-humans]
                                             Catalog entries, filtered names, or reachable groups
  hcom agent show <name>                     Effective config and the exact command
  hcom agent attach <name>                   Focus a running agent's window
  hcom agent edit [--project]                Open a catalog in $EDITOR (creates a starter file)
  hcom agent completions bash|zsh|fish       Shell completion for agent names

Catalog and bundles (weakest to strongest, regardless of launch directory):
  1. built-in defaults (cli={DEFAULT_CLI})
  2. \"defaults\" in ~/.hcom/{GLOBAL_FILE}          (override path: HCOM_AGENTS_FILE)
  3. matching catalog \"defaults\"
  4. the named agent entry
  5. the matching tools.<effective-cli> profile
  6. command-line flags

  Steps 3-4 repeat for each matching imported, additive, or project catalog in
  catalog order. Imports are recursive and apply before the importing file's
  local entries; {EXTRA_CATALOGS_ENV} catalogs apply left to right. Project
  .hcom discovery is Git-independent: every .hcom on the path from the launch
  directory up applies, outermost weakest, so a nested project also addresses
  the enclosing project's agents. Global defaults remain the lowest catalog
  layer for a project agent whether hcom runs inside or outside that project.
  env and args merge; later scalar values replace earlier ones. In particular,
  system_prompt replaces rather than appends, and an explicit empty string clears it.

  Every catalog may use \"imports\" to reference all or selected agents from other
  catalogs. A project agent is addressable, by `hcom agent` and by `hcom send`,
  from inside its project, and elsewhere only where a catalog in scope imports
  it - a selective import's \"agents\" list keeps the project's other agents
  private to that project.
  A sibling agents/<name>/SOUL.md also defines an agent and is appended to its
  system instructions after the fixed JSON system_prompt. Bundle-local skills are
  discovered from agents/<name>/skills/*/SKILL.md. AGENTS.md is not a fallback.
  A project agent ignores a
  same-named non-project entry but still inherits global defaults.
  External bundles are granted through each CLI's additional-workspace mechanism;
  agy and antigravity use --add-dir.
  Relative import paths resolve against the importing file. Relative \"dir\" resolves
  against $HOME globally, the parent of project .hcom (also when imported), or its
  file for other catalogs.
  ~ and $VAR are expanded.

Catalog groups:
  \"groups\": [\"review\", \"all\"] assigns launch-only groups independently of the
  runtime messaging \"tag\". Group launch traverses all recursively reachable
  imports, including agents omitted by a selective import's \"agents\" list.

Listing:
  @<group>                  Show only members of one catalog group
  --all                     Include all agents from recursively reachable imports
  --local                   Show only project agents, direct and imported
  --for-agents              Name and \"description\" only, for another agent to read
  --for-humans              Full table, even when output is not a terminal
  Without either flag, a terminal gets the table and anything else --for-agents.
  Table output shows the effective model; JSON also includes reasoning.

Flags:
  --cli <tool>              claude | codex | gemini | ... (default: {DEFAULT_CLI})
  --dir <path>              Working directory
  --terminal <preset>       hcom terminal preset, or \"here\"
  --terminal-command <cmd>  Raw terminal command with {{script}} (passed via HCOM_TERMINAL)
  --session <name>          tmux session or Herdr space/workspace; empty string disables
  --window <name>           tmux window or Herdr tab (default: agent name)
  --as <name>               Run under a different instance name
  --tag / --model <val>     Forwarded to hcom / the tool
  --reasoning <val>         Reasoning effort (Claude, Antigravity, and Codex)
  --hcom-prompt <text>      Initial user prompt (catalog key: prompt)
  --hcom-system-prompt <text>
                            Invocation-local instructions (catalog key: system_prompt)
  --pre <cmd>               Shell command run in the window before the agent
  --env K=V                 Extra environment variable (repeatable)
  --catalog <path>          Use this file instead of the project catalog
  --no-project              Ignore every enclosing project .hcom/{PROJECT_FILE}
  --attach                  Focus the window after launching (or when already running)
  --restart                 Kill a running agent first instead of reporting it
  --resume                  Continue the agent's previous session
  --clean                   Start a clean session (overrides configured resume)
  --dry-run                 Print the commands without running anything

  Any other flag is forwarded verbatim to `hcom <cli>`.

Catalog tool profiles:
  \"tools\": {{ \"claude\": {{ \"model\": \"sonnet\", \"reasoning\": \"high\" }} }}
  The profile for the effective --cli may set model, reasoning, prompt,
  system_prompt, and args. Reasoning maps to --effort for Claude/Antigravity
  and model_reasoning_effort for Codex.
  Common fields are applied first, then the tool profile, then command-line flags.

Start mode:
  \"resume\": true|false may be set in catalog defaults or an agent entry.
  Unset falls back to false (clean); --resume and --clean override the catalog.

Terminal strategy, in order:
  1. terminal_command set          -> hcom launches with HCOM_TERMINAL=<cmd>
  2. terminal preset selected      -> hcom launches through that preset
  3. otherwise                     -> hcom uses its configured default terminal

For tmux, session and window select the tmux session/window. For Herdr, session selects
the space (workspace), window selects the tab, and the agent runs in a split pane.
Configured-default Herdr launches use the catalog placement, not a parent agent's location.

An instance name is unique: launching one that already runs prints its status and exits 0.
Use --as <name> to launch the same catalog definition as another independent instance."
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
        "list" => cmd_list(&argv[1..]),
        "show" => cmd_show(&argv[1..]),
        "attach" => cmd_attach(&argv[1..]),
        "edit" => cmd_edit(&argv[1..]),
        "completions" => cmd_completions(&argv[1..]),
        _ if first.starts_with('-') => {
            println!("{}", help_text());
            Ok(0)
        }
        group if group.starts_with('@') => cmd_launch_group(&group[1..], &argv[1..]),
        name => cmd_launch(name, &argv[1..]),
    }
}

// ── Catalog model ───────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentDef {
    description: Option<String>,
    dir: Option<String>,
    #[serde(default)]
    skills_dir: Option<serde_json::Value>,
    cli: Option<String>,
    terminal: Option<String>,
    terminal_command: Option<String>,
    session: Option<String>,
    window: Option<String>,
    tag: Option<String>,
    #[serde(default)]
    groups: Vec<String>,
    model: Option<String>,
    reasoning: Option<String>,
    prompt: Option<String>,
    system_prompt: Option<String>,
    pre: Option<String>,
    resume: Option<bool>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    tools: BTreeMap<String, ToolDef>,
    #[serde(skip)]
    agent_dir: Option<String>,
    #[serde(skip)]
    instructions: Option<String>,
    #[serde(skip)]
    instructions_content: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolDef {
    model: Option<String>,
    reasoning: Option<String>,
    prompt: Option<String>,
    system_prompt: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

impl ToolDef {
    fn merge_from(&mut self, over: &ToolDef) {
        if over.model.is_some() {
            self.model = over.model.clone();
        }
        if over.reasoning.is_some() {
            self.reasoning = over.reasoning.clone();
        }
        if over.prompt.is_some() {
            self.prompt = over.prompt.clone();
        }
        if over.system_prompt.is_some() {
            self.system_prompt = over.system_prompt.clone();
        }
        self.args.extend(over.args.iter().cloned());
    }
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
            description,
            dir,
            cli,
            terminal,
            terminal_command,
            session,
            window,
            tag,
            model,
            reasoning,
            prompt,
            system_prompt,
            pre,
            resume,
            agent_dir,
            instructions,
            instructions_content
        );
        for (k, v) in &over.env {
            self.env.insert(k.clone(), v.clone());
        }
        for group in &over.groups {
            if !self.groups.contains(group) {
                self.groups.push(group.clone());
            }
        }
        self.args.extend(over.args.iter().cloned());
        for (tool, profile) in &over.tools {
            self.tools
                .entry(tool.clone())
                .or_default()
                .merge_from(profile);
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    #[serde(default)]
    #[allow(dead_code)]
    version: Option<u32>,
    #[serde(default)]
    imports: Vec<CatalogImport>,
    #[serde(default)]
    defaults: AgentDef,
    #[serde(default)]
    agents: BTreeMap<String, AgentDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogImport {
    from: String,
    /// Omitted imports every agent; an empty list intentionally imports none.
    agents: Option<Vec<String>>,
}

#[derive(Debug)]
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
    let mut catalog: Catalog = if path.is_file() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("invalid JSON in {}: {e}", path.display()))?
    } else {
        Catalog::default()
    };

    for (owner, def) in std::iter::once(("defaults", &catalog.defaults)).chain(
        catalog
            .agents
            .iter()
            .map(|(name, def)| (name.as_str(), def)),
    ) {
        for group in &def.groups {
            if !crate::identity::is_valid_base_name(group) {
                bail!(
                    "invalid agent group '{group}' in {owner}: use lowercase letters, numbers, and underscore"
                );
            }
        }
    }

    // Resolve `dir` while we still know which file it came from.
    if let Some(dir) = catalog.defaults.dir.take() {
        catalog.defaults.dir = Some(expand_path(&dir, base));
    }
    reject_legacy_skills_dir(&catalog.defaults, "defaults", path)?;
    for def in catalog.agents.values_mut() {
        if let Some(dir) = def.dir.take() {
            def.dir = Some(expand_path(&dir, base));
        }
        reject_legacy_skills_dir(def, "agent", path)?;
    }

    let agents_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("agents");
    if agents_dir.is_dir() {
        for entry in std::fs::read_dir(&agents_dir)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", agents_dir.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let dir = std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path());
            let instructions = dir.join("SOUL.md");
            if instructions.is_file() {
                if !crate::identity::is_valid_base_name(&name) {
                    bail!("invalid agent bundle name '{}': {}", name, dir.display());
                }
                catalog.agents.entry(name.clone()).or_default();
            }
            if let Some(def) = catalog.agents.get_mut(&name) {
                def.agent_dir = Some(dir.to_string_lossy().into_owned());
                if instructions.is_file() {
                    def.instructions = Some(instructions.to_string_lossy().into_owned());
                    def.instructions_content =
                        Some(std::fs::read_to_string(&instructions).map_err(|e| {
                            anyhow::anyhow!("cannot read {}: {e}", instructions.display())
                        })?);
                }
            }
        }
    }
    Ok(CatalogFile {
        path: path.to_path_buf(),
        label,
        catalog,
    })
}

/// Load a catalog and its imports in precedence order. Imports are expanded
/// before the importing file, and may themselves import other catalogs.
#[cfg(test)]
fn load_catalog_tree(
    path: &Path,
    base: &Path,
    label: String,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<CatalogFile>> {
    load_catalog_tree_with_mode(path, base, label, stack, false)
}

fn load_catalog_tree_with_mode(
    path: &Path,
    base: &Path,
    label: String,
    stack: &mut Vec<PathBuf>,
    include_all_import_agents: bool,
) -> Result<Vec<CatalogFile>> {
    let absolute = if path.is_absolute() {
        normalize(path)
    } else {
        normalize(&base.join(path))
    };
    let identity = std::fs::canonicalize(&absolute).unwrap_or_else(|_| absolute.clone());
    if let Some(pos) = stack.iter().position(|p| p == &identity) {
        let mut cycle = stack[pos..]
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(identity.display().to_string());
        bail!("catalog import cycle: {}", cycle.join(" -> "));
    }

    stack.push(identity);
    let result = (|| {
        let file_base = absolute
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut current = load_catalog_file(&absolute, base, label.clone())?;
        let imports = std::mem::take(&mut current.catalog.imports);
        let mut files = Vec::new();

        for import in imports {
            let imported_path = PathBuf::from(expand_path(&import.from, &file_base));
            let imported_label = format!("import:{}", imported_path.display());
            let imported_base = catalog_relative_base(&imported_path);
            let mut imported = load_catalog_tree_with_mode(
                &imported_path,
                &imported_base,
                imported_label,
                stack,
                include_all_import_agents,
            )?;
            if let Some(selected) = import.agents {
                let selected: BTreeSet<String> = selected.into_iter().collect();
                let available: BTreeSet<String> = imported
                    .iter()
                    .flat_map(|f| f.catalog.agents.keys().cloned())
                    .collect();
                let missing = selected.difference(&available).cloned().collect::<Vec<_>>();
                if !missing.is_empty() {
                    bail!(
                        "catalog import {} requests unknown agent(s): {}",
                        imported_path.display(),
                        missing.join(", ")
                    );
                }
                if !include_all_import_agents {
                    for file in &mut imported {
                        file.catalog
                            .agents
                            .retain(|name, _| selected.contains(name));
                    }
                }
            }
            files.extend(
                imported
                    .into_iter()
                    .filter(|f| !f.catalog.agents.is_empty()),
            );
        }
        files.push(current);
        Ok(files)
    })();
    stack.pop();
    result
}

fn catalog_relative_base(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if path.file_name().is_some_and(|name| name == PROJECT_FILE)
        && parent.file_name().is_some_and(|name| name == PROJECT_DIR)
    {
        parent.parent().unwrap_or(parent).to_path_buf()
    } else {
        parent.to_path_buf()
    }
}

fn global_catalog_path() -> PathBuf {
    match std::env::var_os("HCOM_AGENTS_FILE") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => crate::paths::hcom_dir().join(GLOBAL_FILE),
    }
}

/// Every project `.hcom` walking up from `start`, outermost first, independent
/// of Git roots. A nested project sees the enclosing project's agents, with the
/// nearest catalog strongest. The user's global hcom directory is not a project
/// scope.
fn find_project_hcom_dirs(start: &Path) -> Vec<PathBuf> {
    let global = global_catalog_path()
        .parent()
        .map(normalize)
        .unwrap_or_else(|| normalize(&crate::paths::hcom_dir()));
    let mut found = Vec::new();
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = normalize(&d.join(PROJECT_DIR));
        if candidate.is_dir() && candidate != global {
            found.push(candidate);
        }
        dir = d.parent();
    }
    found.reverse();
    found
}

/// Root a project catalog's relative paths resolve against.
fn project_root_of(hcom_dir: &Path) -> PathBuf {
    if hcom_dir.file_name().is_some_and(|name| name == PROJECT_DIR) {
        hcom_dir.parent().unwrap_or(hcom_dir).to_path_buf()
    } else {
        hcom_dir.to_path_buf()
    }
}

struct Catalogs {
    base_files: Vec<CatalogFile>,
    project_files: Vec<CatalogFile>,
    project_root: Option<PathBuf>,
}

impl Catalogs {
    fn load(no_project: bool, explicit: Option<&Path>) -> Result<Self> {
        Self::load_with_mode(no_project, explicit, false)
    }

    fn load_for_groups(no_project: bool, explicit: Option<&Path>) -> Result<Self> {
        Self::load_with_mode(no_project, explicit, true)
    }

    fn load_with_mode(
        no_project: bool,
        explicit: Option<&Path>,
        include_all_import_agents: bool,
    ) -> Result<Self> {
        let mut base_files = Vec::new();
        let mut stack = Vec::new();

        let global = global_catalog_path();
        let global_bundles = global.parent().is_some_and(|p| p.join("agents").is_dir());
        if global.is_file() || global_bundles {
            let base = home_dir().unwrap_or_else(|| PathBuf::from("."));
            if global.is_file() {
                base_files.extend(load_catalog_tree_with_mode(
                    &global,
                    &base,
                    "global".to_string(),
                    &mut stack,
                    include_all_import_agents,
                )?);
            } else {
                base_files.push(load_catalog_file(&global, &base, "global".to_string())?);
            }
        }

        if let Some(raw) = std::env::var_os(EXTRA_CATALOGS_ENV).filter(|v| !v.is_empty()) {
            for path in std::env::split_paths(&raw) {
                if !path.is_file() {
                    bail!(
                        "catalog from {EXTRA_CATALOGS_ENV} not found: {}",
                        path.display()
                    );
                }
                let base = path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."));
                base_files.extend(load_catalog_tree_with_mode(
                    &path,
                    &base,
                    format!("extra:{}", path.display()),
                    &mut stack,
                    include_all_import_agents,
                )?);
            }
        }

        let project_hcom = match explicit {
            Some(p) => {
                if !p.is_file() {
                    bail!("catalog not found: {}", p.display());
                }
                p.parent().map(Path::to_path_buf).into_iter().collect()
            }
            None if no_project => Vec::new(),
            None => std::env::current_dir()
                .ok()
                .map(|cwd| find_project_hcom_dirs(&cwd))
                .unwrap_or_default(),
        };
        let project_root = project_hcom.last().map(|dir| project_root_of(dir));
        let mut project_files = Vec::new();
        for hcom_dir in &project_hcom {
            let path = explicit
                .map(Path::to_path_buf)
                .unwrap_or_else(|| hcom_dir.join(PROJECT_FILE));
            let base = project_root_of(hcom_dir);
            if path.is_file() {
                project_files.extend(load_catalog_tree_with_mode(
                    &path,
                    &base,
                    "project".to_string(),
                    &mut stack,
                    include_all_import_agents,
                )?);
            } else {
                project_files.push(load_catalog_file(&path, &base, "project".to_string())?);
            }
        }

        Ok(Self {
            base_files,
            project_files,
            project_root,
        })
    }

    /// Names in catalog order, mapped to the label of the last file defining them.
    fn names(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for file in &self.base_files {
            for name in file.catalog.agents.keys() {
                out.entry(name.clone())
                    .and_modify(|l: &mut String| *l = format!("{l}+{}", file.label))
                    .or_insert_with(|| file.label.clone());
            }
        }
        for file in &self.project_files {
            for name in file.catalog.agents.keys() {
                out.insert(name.clone(), file.label.clone());
            }
        }
        out
    }

    /// Project-scoped names, including agents from the project's imports.
    fn local_names(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for file in &self.project_files {
            for name in file.catalog.agents.keys() {
                out.insert(name.clone(), file.label.clone());
            }
        }
        out
    }

    fn resolve(&self, name: &str) -> Option<AgentDef> {
        if self
            .project_files
            .iter()
            .any(|f| f.catalog.agents.contains_key(name))
        {
            return Self::resolve_from(&self.project_files, &self.base_files, name);
        }
        Self::resolve_from(&self.base_files, &self.base_files, name)
    }

    fn group_members(&self, group: &str) -> Vec<String> {
        self.names()
            .into_keys()
            .filter(|name| {
                self.resolve(name)
                    .is_some_and(|def| def.groups.iter().any(|item| item == group))
            })
            .collect()
    }

    fn group_names(&self) -> BTreeSet<String> {
        self.names()
            .into_keys()
            .filter_map(|name| self.resolve(&name))
            .flat_map(|def| def.groups)
            .collect()
    }

    fn resolve_from(
        files: &[CatalogFile],
        base_files: &[CatalogFile],
        name: &str,
    ) -> Option<AgentDef> {
        if !files.iter().any(|f| f.catalog.agents.contains_key(name)) {
            return None;
        }
        // Global defaults are the lowest-precedence base for every catalog
        // agent, including project agents and agents brought in by imports.
        // Apply them before the catalog-order merge so imported and project
        // definitions can override them.
        let mut def = AgentDef::default();
        for file in base_files.iter().filter(|file| file.label == "global") {
            def.merge_from(&file.catalog.defaults);
        }
        for file in files {
            let entry = file.catalog.agents.get(name);
            if file.label != "global" && entry.is_some() {
                def.merge_from(&file.catalog.defaults);
            }
            if let Some(entry) = entry {
                def.merge_from(entry);
            }
        }
        Some(def)
    }

    fn paths(&self) -> Vec<&Path> {
        self.base_files
            .iter()
            .chain(&self.project_files)
            .map(|f| f.path.as_path())
            .filter(|path| path.is_file())
            .collect()
    }
}

/// Catalog agent identities available for message routing, including stopped agents.
pub(crate) fn catalog_message_targets() -> Result<Vec<(String, Option<String>)>> {
    let catalogs = Catalogs::load(false, None)?;
    Ok(catalogs
        .names()
        .keys()
        .filter(|name| !name.contains(':'))
        .filter_map(|name| {
            let def = catalogs.resolve(name)?;
            let eff = effective(name, def, &Cli::default());
            Some((name.clone(), eff.tag))
        })
        .collect())
}

/// How long autostart waits for the launched agent's instance row to appear.
///
/// This wait is the one place where expiry really does drop the message (a
/// send with no recipient row has nowhere to queue), so it is generous: the
/// row is normally reserved within milliseconds of the launch starting.
const CATALOG_REGISTRATION_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// What a launch exit code means for the send that triggered the autostart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutostartOutcome {
    /// Agent reported ready.
    Started,
    /// Still launching, or blocked on user attention: the process is up, only
    /// readiness was not observed within the inline wait.
    StillLaunching,
    /// The agent could not be spawned at all.
    Failed,
}

pub(crate) fn classify_autostart_exit(code: i32) -> AutostartOutcome {
    match code {
        0 => AutostartOutcome::Started,
        2 => AutostartOutcome::StillLaunching,
        _ => AutostartOutcome::Failed,
    }
}

/// Start a configured agent using its effective catalog start mode.
///
/// A launch that is merely still starting must not abort the send: the message
/// is written to the DB after the instance row is reserved, so the agent
/// receives it once its delivery loop starts. Treating that as a start failure
/// used to discard the payload while reporting a failure that had not happened.
pub(crate) fn autostart_catalog_agent(db: &HcomDb, name: &str) -> Result<()> {
    let code = cmd_launch(name, &[])?;
    match classify_autostart_exit(code) {
        AutostartOutcome::Started => {
            wait_for_catalog_agent_registration(db, name, CATALOG_REGISTRATION_WAIT)
        }
        AutostartOutcome::StillLaunching => {
            wait_for_catalog_agent_registration(db, name, CATALOG_REGISTRATION_WAIT)?;
            println!(
                "Agent '{name}' is still starting - message queued, it is delivered when the agent is ready."
            );
            Ok(())
        }
        AutostartOutcome::Failed => {
            bail!("could not start catalog agent '{name}' (exit {code})")
        }
    }
}

fn wait_for_catalog_agent_registration(
    db: &HcomDb,
    name: &str,
    timeout: std::time::Duration,
) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if db
            .get_instance_full(name)
            .ok()
            .flatten()
            .is_some_and(|row| {
                row.status != "stopped"
                    && row.status_context != "launch_failed"
                    && !(row.status == "inactive" && row.status_context.starts_with("exit:"))
            })
        {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!("catalog agent '{name}' did not register before timeout");
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// Scope note for a name this directory cannot address.
///
/// A project keeps its internal agents to itself: a catalog exports what its
/// import selects and nothing else, and that isolation is the point. So this
/// says that scope depends on the directory and stops there — never whether
/// some other name exists elsewhere, in which file, or how to widen an import.
/// Someone standing outside the project learns nothing about its inside; the
/// person who can act on this reads it in a shell that is already in scope.
pub(crate) const OUT_OF_SCOPE_NOTE: &str = "Catalog scope depends on the directory: a project's .hcom/agents.json is in scope inside that project, and elsewhere only for the agents another catalog in scope imports.";

fn unknown_agent_error(name: &str, catalogs: &Catalogs) -> anyhow::Error {
    let names = catalogs.names();
    let mut msg = format!("unknown agent '{name}'");
    if let Some(close) = closest(name, names.keys()) {
        msg.push_str(&format!(" (did you mean '{close}'?)"));
    }
    if !names.is_empty() {
        msg.push('\n');
        msg.push_str(OUT_OF_SCOPE_NOTE);
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
        if d <= target.len().div_ceil(2) && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
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
    as_name: Option<String>,
    attach: bool,
    dry_run: bool,
    restart: bool,
    resume: Option<bool>,
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
            "--skills-dir" => bail!(
                "--skills-dir was removed; move each skill to agents/<name>/skills/<skill>/SKILL.md"
            ),
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
            "--as" => {
                cli.as_name = Some(value()?);
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
            "--reasoning" => {
                cli.def.reasoning = Some(value()?);
                i += 2;
            }
            "--hcom-prompt" => {
                cli.def.prompt = Some(value()?);
                i += 2;
            }
            "--hcom-system-prompt" => {
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
            "--resume" => {
                cli.resume = Some(true);
                i += 1;
            }
            "--clean" => {
                cli.resume = Some(false);
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
    description: Option<String>,
    cli: String,
    dir: String,
    skills: Vec<AgentSkill>,
    skill_warnings: Vec<String>,
    window: String,
    session: Option<String>,
    terminal: Option<String>,
    terminal_command: Option<String>,
    pre: Option<String>,
    tag: Option<String>,
    groups: Vec<String>,
    model: Option<String>,
    reasoning: Option<String>,
    reasoning_error: Option<String>,
    prompt: Option<String>,
    system_prompt: Option<String>,
    agent_dir: Option<String>,
    instructions: Option<String>,
    bundle_args: Vec<String>,
    bundle_access_error: Option<String>,
    resume: bool,
    env: BTreeMap<String, String>,
    extra: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentSkill {
    name: String,
    description: String,
    path: String,
}

fn reject_legacy_skills_dir(def: &AgentDef, owner: &str, path: &Path) -> Result<()> {
    if def.skills_dir.is_some() {
        bail!(
            "skills_dir in {owner} of {} was removed; move each skill to agents/<name>/skills/<skill>/SKILL.md",
            path.display()
        );
    }
    Ok(())
}

fn discover_agent_skills(bundle: &str) -> (Vec<AgentSkill>, Vec<String>) {
    let bundle = Path::new(bundle);
    let canonical_bundle = match std::fs::canonicalize(bundle) {
        Ok(path) => path,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let root = bundle.join("skills");
    let Ok(entries) = std::fs::read_dir(&root) else {
        return (Vec::new(), Vec::new());
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    let mut skills = Vec::new();
    let mut warnings = Vec::new();
    let mut names = BTreeSet::new();
    for dir in paths {
        let skill_file = dir.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }
        let Ok(canonical) = std::fs::canonicalize(&skill_file) else {
            warnings.push(format!(
                "cannot canonicalize skill {}",
                skill_file.display()
            ));
            continue;
        };
        if !canonical.starts_with(&canonical_bundle) {
            warnings.push(format!(
                "skipped skill outside bundle: {}",
                canonical.display()
            ));
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&canonical) else {
            warnings.push(format!("cannot read skill {}", canonical.display()));
            continue;
        };
        let basename = dir.file_name().and_then(|s| s.to_str()).unwrap_or("skill");
        let (name, description, metadata_ok) = parse_skill_metadata(&body, basename);
        if !metadata_ok {
            warnings.push(format!(
                "invalid or incomplete metadata in {}",
                canonical.display()
            ));
        }
        if !names.insert(name.clone()) {
            warnings.push(format!(
                "duplicate skill name '{name}' at {}",
                canonical.display()
            ));
        }
        skills.push(AgentSkill {
            name,
            description,
            path: canonical.to_string_lossy().into_owned(),
        });
    }
    (skills, warnings)
}

fn parse_skill_metadata(body: &str, basename: &str) -> (String, String, bool) {
    let normalized = body.replace("\r\n", "\n");
    let body = normalized.as_str();
    let mut name = None;
    let mut description = None;
    let mut metadata_ok = false;
    let mut content = body;
    if let Some(rest) = body.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        metadata_ok = true;
        for line in rest[..end].lines() {
            if let Some(value) = line.strip_prefix("name:") {
                name = Some(value.trim().trim_matches(['\'', '"']).to_string());
            } else if let Some(value) = line.strip_prefix("description:") {
                description = Some(value.trim().trim_matches(['\'', '"']).to_string());
            }
        }
        content = &rest[end + 4..];
    }
    let fallback_description = content
        .lines()
        .find_map(|line| {
            let heading = line
                .trim()
                .strip_prefix('#')?
                .trim_start_matches('#')
                .trim();
            (!heading.is_empty()).then(|| heading.to_string())
        })
        .unwrap_or_else(|| "Agent-local skill; read SKILL.md for details.".into());
    let valid = metadata_ok
        && name.as_ref().is_some_and(|v| !v.is_empty())
        && description.as_ref().is_some_and(|v| !v.is_empty());
    (
        name.filter(|v| !v.is_empty())
            .unwrap_or_else(|| basename.to_string()),
        description
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback_description),
        valid,
    )
}

fn build_agent_instructions(
    fixed: Option<String>,
    bundle_dir: Option<&str>,
    instruction_path: Option<String>,
    instruction_body: Option<String>,
    skills: &[AgentSkill],
) -> Option<String> {
    let mut sections = Vec::new();
    if let Some(fixed) = fixed {
        sections.push(fixed);
    }
    if let (Some(dir), Some(path), Some(body)) = (bundle_dir, instruction_path, instruction_body) {
        sections.push(format!(
            "# Agent bundle instructions\n\nBundle directory: `{dir}`\nInstruction file: `{path}`\n\nResolve relative paths mentioned by these instructions from the bundle directory.\nThe bundle is editable when the selected CLI supports the granted workspace access.\n\n{body}"
        ));
    }
    if !skills.is_empty() {
        let mut manifest = String::from(
            "# Available agent skills\n\nThe following skills belong to this agent. When a task matches a description,\nread the referenced SKILL.md completely before using it. Resolve its relative\nreferences from that skill directory.",
        );
        for skill in skills {
            manifest.push_str(&format!(
                "\n\n## {}\n\nDescription: {}\nInstructions: `{}`",
                skill.name, skill.description, skill.path
            ));
        }
        sections.push(manifest);
    }
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

fn nonempty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.trim().is_empty())
}

/// Collapse whitespace so a multi-line description stays on one output line.
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn effective(name: &str, mut def: AgentDef, cli: &Cli) -> Effective {
    let selected_cli = cli
        .def
        .cli
        .as_deref()
        .or(def.cli.as_deref())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(DEFAULT_CLI)
        .to_string();
    if let Some(profile) = def.tools.get(&selected_cli).cloned() {
        if profile.model.is_some() {
            def.model = profile.model;
        }
        if profile.reasoning.is_some() {
            def.reasoning = profile.reasoning;
        }
        if profile.prompt.is_some() {
            def.prompt = profile.prompt;
        }
        if profile.system_prompt.is_some() {
            def.system_prompt = profile.system_prompt;
        }
        def.args.extend(profile.args);
    }
    def.merge_from(&cli.def);
    let mut extra = std::mem::take(&mut def.args);
    extra.extend(cli.passthrough.iter().cloned());

    let (skills, skill_warnings) = def
        .agent_dir
        .as_deref()
        .map(discover_agent_skills)
        .unwrap_or_default();
    let system_prompt = build_agent_instructions(
        nonempty(def.system_prompt),
        def.agent_dir.as_deref(),
        nonempty(def.instructions.clone()),
        nonempty(def.instructions_content),
        &skills,
    );

    let reasoning = nonempty(def.reasoning);
    let reasoning_error = reasoning.as_ref().and_then(|_| match selected_cli.as_str() {
        "claude" | "agy" | "antigravity" | "codex" => None,
        cli => Some(format!(
            "reasoning is not supported for cli '{cli}'; use tools.{cli}.args for a tool-specific setting"
        )),
    });
    let mut effective = Effective {
        name: name.to_string(),
        description: nonempty(def.description).map(|text| single_line(&text)),
        cli: selected_cli,
        dir: nonempty(def.dir).unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".to_string())
        }),
        skills,
        skill_warnings,
        window: nonempty(def.window).unwrap_or_else(|| name.to_string()),
        session: nonempty(def.session),
        terminal: nonempty(def.terminal),
        terminal_command: nonempty(def.terminal_command),
        pre: nonempty(def.pre),
        tag: nonempty(def.tag),
        groups: def.groups,
        model: nonempty(def.model),
        reasoning,
        reasoning_error,
        prompt: nonempty(def.prompt),
        system_prompt,
        agent_dir: nonempty(def.agent_dir),
        instructions: nonempty(def.instructions),
        bundle_args: Vec::new(),
        bundle_access_error: None,
        resume: cli.resume.or(def.resume).unwrap_or(false),
        env: def.env,
        extra,
    };
    apply_bundle_access(&mut effective);
    effective
}

fn apply_bundle_access(eff: &mut Effective) {
    let Some(dir) = eff.agent_dir.clone() else {
        return;
    };
    let workspace =
        std::fs::canonicalize(&eff.dir).unwrap_or_else(|_| normalize(Path::new(&eff.dir)));
    let bundle = std::fs::canonicalize(&dir).unwrap_or_else(|_| normalize(Path::new(&dir)));
    if bundle.starts_with(&workspace) {
        return;
    }

    match eff.cli.as_str() {
        "claude" | "codex" | "agy" | "antigravity" => {
            eff.bundle_args.extend(["--add-dir".into(), dir]);
        }
        "gemini" => {
            eff.bundle_args
                .extend(["--include-directories".into(), dir]);
        }
        "omp" | "copilot" => eff.bundle_args.push(format!("--add-dir={dir}")),
        "opencode" | "kilo" => {
            let key = if eff.cli == "opencode" {
                "OPENCODE_CONFIG_CONTENT"
            } else {
                "KILO_CONFIG_CONTENT"
            };
            let pattern = format!("{}/**", dir.trim_end_matches(['/', '\\']));
            merge_inline_json_env(
                &mut eff.env,
                key,
                serde_json::json!({
                    "permission": { "external_directory": { (pattern): "allow" } }
                }),
            );
        }
        _ => {
            eff.bundle_access_error = Some(format!(
                "{} cannot make external agent bundle {} writable; place the bundle inside the working directory or use a CLI with additional-directory support",
                eff.cli, dir
            ));
        }
    }
}

fn merge_inline_json_env(
    env: &mut BTreeMap<String, String>,
    key: &str,
    addition: serde_json::Value,
) {
    let existing = env.get(key).cloned().or_else(|| std::env::var(key).ok());
    let mut base = match existing {
        Some(raw) if !raw.trim().is_empty() => match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => return,
        },
        _ => serde_json::json!({}),
    };
    merge_json(&mut base, addition);
    env.insert(key.to_string(), base.to_string());
}

fn merge_json(base: &mut serde_json::Value, addition: serde_json::Value) {
    match (base, addition) {
        (serde_json::Value::Object(base), serde_json::Value::Object(addition)) => {
            for (key, value) in addition {
                merge_json(base.entry(key).or_insert(serde_json::Value::Null), value);
            }
        }
        (serde_json::Value::Array(base), serde_json::Value::Array(addition)) => {
            base.extend(addition);
        }
        (base, addition) => *base = addition,
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

fn choose_strategy(
    eff: &Effective,
    tmux_available: bool,
    configured_terminal: Option<&str>,
) -> (Strategy, Vec<String>) {
    let mut warnings = Vec::new();

    if let Some(cmd) = &eff.terminal_command {
        if eff.session.is_some() {
            warnings.push("session ignored: terminal_command takes precedence".to_string());
        }
        return (Strategy::Custom(cmd.clone()), warnings);
    }

    if let Some(session) = &eff.session {
        if eff.terminal.as_deref().or(configured_terminal) == Some("herdr") {
            return (Strategy::Direct(eff.terminal.clone()), warnings);
        }
        let mux_ok = eff
            .terminal
            .as_deref()
            .is_some_and(|t| t.starts_with("tmux"));
        if !mux_ok {
            let preset = eff.terminal.as_deref().unwrap_or("default");
            warnings.push(format!(
                "session '{session}' ignored: terminal preset '{preset}' is not a multiplexer"
            ));
        } else if !tmux_available {
            warnings.push(format!(
                "session '{session}' ignored: tmux not found on PATH"
            ));
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

/// The configured terminal matters when an agent inherits hcom's default.
/// In particular, a Herdr default owns session/workspace placement even if the
/// catalog does not repeat `terminal = "herdr"`.
fn configured_terminal() -> Option<String> {
    crate::config::HcomConfig::load(None)
        .ok()
        .map(|config| config.terminal)
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
    v.extend(eff.bundle_args.iter().cloned());
    v.push("--go".into());
    // Tool-level args last.
    if let Some(m) = &eff.model {
        v.push("--model".into());
        v.push(m.clone());
    }
    if let Some(reasoning) = &eff.reasoning {
        match eff.cli.as_str() {
            "claude" | "agy" | "antigravity" => {
                v.push("--effort".into());
                v.push(reasoning.clone());
            }
            "codex" => {
                v.push("-c".into());
                v.push(format!(
                    "model_reasoning_effort={}",
                    serde_json::to_string(reasoning).expect("string serialization cannot fail")
                ));
            }
            _ => {}
        }
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
    // tmux executes a supplied window command through a non-interactive shell.
    // Re-enter the user's login+interactive shell from the target directory so
    // its init files establish the same environment as a normal terminal.
    // Keep the pane in that initialized shell once the agent exits.
    line.push_str("; exec \"${SHELL:-/bin/bash}\" -l");
    format!(
        "exec \"${{SHELL:-/bin/bash}}\" -lic {}",
        shell_words::quote(&line)
    )
}

// ── Live state ──────────────────────────────────────────────────────────

#[derive(Debug)]
struct LiveAgent {
    name: String,
    status: String,
    tool: String,
    directory: String,
    launch_context: serde_json::Value,
}

impl LiveAgent {
    fn from_instance(data: &crate::db::InstanceRow, status: String) -> Self {
        Self {
            name: data.name.clone(),
            status,
            tool: data.tool.clone(),
            directory: data.directory.clone(),
            launch_context: data
                .launch_context
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or_else(|| serde_json::json!({})),
        }
    }
}

impl LiveAgent {
    fn pane_id(&self) -> Option<&str> {
        self.launch_context.get("pane_id")?.as_str()
    }

    fn location(&self) -> Option<String> {
        let terminal = self
            .launch_context
            .get("terminal_preset_effective")?
            .as_str()?;
        Some(match self.pane_id() {
            Some(pane) => format!(", {terminal} {pane}"),
            None => format!(", {terminal}"),
        })
    }
}

fn live_agents() -> Vec<LiveAgent> {
    let Ok(db) = HcomDb::open() else {
        return Vec::new();
    };
    db.iter_instances_full()
        .unwrap_or_default()
        .iter()
        .map(|data| {
            let computed = crate::instance_lifecycle::get_instance_status(data, &db);
            LiveAgent::from_instance(data, computed.status)
        })
        .collect()
}

fn find_live(name: &str) -> Option<LiveAgent> {
    let db = HcomDb::open().ok()?;
    let instance = db.get_instance_full(name).ok().flatten()?;
    let computed = crate::instance_lifecycle::get_instance_status(&instance, &db);
    if crate::launcher::instance_is_replaceable(&instance, &computed) {
        return None;
    }
    Some(LiveAgent::from_instance(&instance, computed.status))
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
    let out = Command::new(bin)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
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
    tmux_capture(&[
        "list-windows",
        "-t",
        &format!("={session}"),
        "-F",
        "#{window_name}",
    ])
    .map(|out| out.lines().any(|l| l == window))
    .unwrap_or(false)
}

/// Create the session (with the agent as its first window) or add a window to it.
fn tmux_launch(session: &str, window: &str, dir: &str, command: &str, dry_run: bool) -> Result<()> {
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
        &[
            "select-window".to_string(),
            "-t".to_string(),
            target.to_string(),
        ],
        false,
    )?;
    let inside = std::env::var_os("TMUX").is_some();
    let verb = if inside {
        "switch-client"
    } else {
        "attach-session"
    };
    tmux_run(
        &[verb.to_string(), "-t".to_string(), target.to_string()],
        false,
    )
}

fn focus_running(eff: &Effective, live: &LiveAgent) -> Result<()> {
    if live
        .launch_context
        .get("terminal_preset_effective")
        .and_then(|value| value.as_str())
        == Some("herdr")
    {
        let status = Command::new("herdr")
            .args(["agent", "focus", &eff.name])
            .status()?;
        if !status.success() {
            bail!("could not focus Herdr agent '{}'", eff.name);
        }
        return Ok(());
    }
    if let Some(pane) = live.pane_id() {
        return tmux_focus(pane);
    }
    match &eff.session {
        Some(session) => tmux_focus(&format!("{session}:{}", eff.window)),
        None => bail!("no pane recorded for '{}' — nothing to attach to", eff.name),
    }
}

// ── Subcommand: launch ──────────────────────────────────────────────────

fn cmd_launch(name: &str, rest: &[String]) -> Result<i32> {
    let cli = parse_cli(rest)?;
    let catalogs = Catalogs::load(cli.no_project, cli.catalog.as_deref())?;
    launch_named(name, &cli, &catalogs)
}

fn launch_named(name: &str, cli: &Cli, catalogs: &Catalogs) -> Result<i32> {
    let def = catalogs
        .resolve(name)
        .ok_or_else(|| unknown_agent_error(name, &catalogs))?;
    let instance_name = cli.as_name.as_deref().unwrap_or(name);
    let window_explicit = def.window.is_some() || cli.def.window.is_some();
    let mut eff = effective(instance_name, def, &cli);
    let configured_terminal = configured_terminal();
    apply_herdr_placement(
        &mut eff,
        catalogs.project_root.as_deref(),
        window_explicit,
        configured_terminal.as_deref(),
    );

    let live = find_live(instance_name);
    if let Some(live) = &live {
        if cli.restart {
            if cli.dry_run {
                println!("{} kill {}", hcom_bin(), instance_name);
            } else {
                let _ = Command::new(hcom_bin())
                    .args(["kill", instance_name])
                    .status();
            }
        } else {
            let where_ = live.location().unwrap_or_default();
            println!(
                "agent '{instance_name}' already running ({}, {}, {}{where_})",
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

    launch(&eff, &cli, eff.resume)
}

fn cmd_launch_group(group: &str, rest: &[String]) -> Result<i32> {
    if group.is_empty() || !crate::identity::is_valid_base_name(group) {
        bail!(
            "invalid agent group '{group}': use @ followed by lowercase letters, numbers, and underscore"
        );
    }
    let cli = parse_cli(rest)?;
    if cli.as_name.is_some() {
        bail!("--as cannot be used when launching an agent group");
    }
    if cli.attach {
        bail!("--attach cannot be used when launching an agent group");
    }

    let catalogs = Catalogs::load_for_groups(cli.no_project, cli.catalog.as_deref())?;
    let members = catalogs.group_members(group);
    if members.is_empty() {
        let groups = catalogs.group_names();
        let mut message = format!("unknown or empty agent group '@{group}'");
        if let Some(close) = closest(group, groups.iter()) {
            message.push_str(&format!(" (did you mean '@{close}'?)"));
        }
        if !groups.is_empty() {
            message.push_str(&format!(
                "\nAvailable groups: {}",
                groups
                    .into_iter()
                    .map(|name| format!("@{name}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        bail!(message);
    }

    for name in &members {
        let def = catalogs.resolve(name).unwrap_or_default();
        let mut eff = effective(name, def, &cli);
        let configured_terminal = configured_terminal();
        apply_herdr_placement(
            &mut eff,
            catalogs.project_root.as_deref(),
            false,
            configured_terminal.as_deref(),
        );
        let (strategy, _) =
            choose_strategy(&eff, tmux_bin().is_some(), configured_terminal.as_deref());
        if strategy == Strategy::Direct(Some("here".to_string())) {
            bail!(
                "agent '{name}' in group '@{group}' would launch in the current terminal; groups require separate terminal panes"
            );
        }
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    for name in &members {
        match launch_named(name, &cli, &catalogs) {
            Ok(0) => succeeded += 1,
            Ok(code) => {
                failed += 1;
                eprintln!("agent '{name}' failed with exit {code}");
            }
            Err(error) => {
                failed += 1;
                eprintln!("agent '{name}' failed: {error:#}");
            }
        }
    }
    println!(
        "group '@{group}': {succeeded} succeeded, {failed} failed ({} total)",
        members.len()
    );
    Ok(if failed == 0 { 0 } else { 1 })
}

fn apply_herdr_placement(
    eff: &mut Effective,
    project_root: Option<&Path>,
    window_explicit: bool,
    configured_terminal: Option<&str>,
) {
    if eff.terminal_command.is_some()
        || eff.terminal.as_deref().or(configured_terminal) != Some("herdr")
    {
        return;
    }
    if eff.session.is_none() {
        eff.session = project_root
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(str::to_string);
    }
    if !window_explicit {
        eff.window = eff.name.clone();
    }
    if let Some(workspace) = &eff.session {
        eff.env
            .insert("HCOM_HERDR_WORKSPACE".to_string(), workspace.clone());
        eff.env
            .insert("HCOM_HERDR_TAB".to_string(), eff.window.clone());
    }
}

fn launch(eff: &Effective, cli: &Cli, resume: bool) -> Result<i32> {
    if let Some(error) = &eff.reasoning_error {
        bail!("{error}");
    }
    if let Some(error) = &eff.bundle_access_error {
        bail!("{error}");
    }
    if resume {
        let db = HcomDb::open()?;
        crate::commands::resume::validate_tracked_resume(&db, &eff.name)?;
    }
    let configured_terminal = configured_terminal();
    let (strategy, warnings) =
        choose_strategy(eff, tmux_bin().is_some(), configured_terminal.as_deref());
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    if resume {
        println!("resuming '{}' ({})", eff.name, eff.cli);
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

// ── Subcommand: list / show / attach / edit / completions ────────────────

fn cmd_list(rest: &[String]) -> Result<i32> {
    let cli = parse_cli(rest)?;
    let json = cli.passthrough.iter().any(|a| a == "--json");
    let names_only = cli.passthrough.iter().any(|a| a == "--names");
    let groups_only = cli.passthrough.iter().any(|a| a == "--groups");
    let all = cli.passthrough.iter().any(|a| a == "--all");
    let local = cli.passthrough.iter().any(|a| a == "--local");
    let for_agents = cli.passthrough.iter().any(|a| a == "--for-agents");
    let for_humans = cli.passthrough.iter().any(|a| a == "--for-humans");
    let group_filters: Vec<&str> = cli
        .passthrough
        .iter()
        .filter_map(|arg| arg.strip_prefix('@'))
        .collect();
    if names_only && groups_only {
        bail!("--names and --groups cannot be used together");
    }
    if group_filters.len() > 1 {
        bail!("agent list accepts at most one @<group> filter");
    }
    if groups_only && !group_filters.is_empty() {
        bail!("@<group> and --groups cannot be used together");
    }
    if for_agents && for_humans {
        bail!("--for-agents and --for-humans cannot be used together");
    }
    // Without an explicit choice, an interactive terminal gets the full table and
    // everything else (an agent's shell, a pipe) gets the name/description view.
    let brief = for_agents || (!for_humans && !std::io::stdout().is_terminal());
    let catalogs = if groups_only || all || !group_filters.is_empty() {
        Catalogs::load_for_groups(cli.no_project, cli.catalog.as_deref())?
    } else {
        Catalogs::load(cli.no_project, cli.catalog.as_deref())?
    };
    let mut names = if local {
        catalogs.local_names()
    } else {
        catalogs.names()
    };
    if groups_only {
        let groups = names
            .keys()
            .filter_map(|name| catalogs.resolve(name))
            .flat_map(|def| def.groups)
            .collect::<BTreeSet<_>>();
        for group in groups {
            println!("@{group}");
        }
        return Ok(0);
    }
    if let Some(group) = group_filters.first() {
        if group.is_empty() || !crate::identity::is_valid_base_name(group) {
            bail!(
                "invalid agent group '{group}': use @ followed by lowercase letters, numbers, and underscore"
            );
        }
        let members = names
            .keys()
            .filter(|name| {
                catalogs
                    .resolve(name)
                    .is_some_and(|def| def.groups.iter().any(|item| item == group))
            })
            .cloned()
            .collect::<Vec<_>>();
        if members.is_empty() {
            let groups = names
                .keys()
                .filter_map(|name| catalogs.resolve(name))
                .flat_map(|def| def.groups)
                .collect::<BTreeSet<_>>();
            let mut message = format!("unknown or empty agent group '@{group}'");
            if let Some(close) = closest(group, groups.iter()) {
                message.push_str(&format!(" (did you mean '@{close}'?)"));
            }
            if !groups.is_empty() {
                message.push_str(&format!(
                    "\nAvailable groups: {}",
                    groups
                        .into_iter()
                        .map(|name| format!("@{name}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            bail!(message);
        }
        names.retain(|name, _| members.contains(name));
    }

    if names_only {
        for name in names.keys() {
            println!("{name}");
        }
        return Ok(0);
    }

    if names.is_empty() {
        println!(
            "No {}agents defined. {}",
            if local { "project " } else { "" },
            if local {
                "Create .hcom/agents.json or run `hcom agent edit --project`.".to_string()
            } else {
                format!(
                    "Create {} or run `hcom agent edit`.",
                    global_catalog_path().display()
                )
            }
        );
        return Ok(0);
    }

    if brief && !json {
        let width = names
            .keys()
            .map(|name| name.chars().count())
            .max()
            .unwrap_or(0);
        for name in names.keys() {
            let def = catalogs.resolve(name).unwrap_or_default();
            let description = effective(name, def, &Cli::default())
                .description
                .unwrap_or_else(|| "-".to_string());
            println!("{name:<width$}  {description}");
        }
        return Ok(0);
    }

    let live: BTreeMap<String, LiveAgent> = live_agents()
        .into_iter()
        .map(|a| (a.name.clone(), a))
        .collect();

    if json {
        let mut out = Vec::new();
        for (name, source) in &names {
            let def = catalogs.resolve(name).unwrap_or_default();
            let eff = effective(name, def, &Cli::default());
            out.push(serde_json::json!({
                "name": name,
                "description": eff.description,
                "source": source,
                "cli": eff.cli,
                "model": eff.model,
                "reasoning": eff.reasoning,
                "dir": eff.dir,
                "agent_dir": eff.agent_dir,
                "instructions": eff.instructions,
                "skills": eff.skills,
                "session": eff.session,
                "window": eff.window,
                "terminal": eff.terminal,
                "groups": eff.groups,
                "status": live.get(name).map(|a| a.status.clone()),
            }));
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(0);
    }

    let rows: Vec<[String; 6]> = names
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
                eff.model.unwrap_or_else(|| "-".to_string()),
                where_,
                shorten_home(&eff.dir),
                format!("{status}  [{source}]"),
            ]
        })
        .collect();

    let headers = ["NAME", "CLI", "MODEL", "WHERE", "DIR", "STATUS"];
    let mut widths = headers.map(str::len);
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let render = |cells: &[String; 6]| {
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
    let instance_name = cli.as_name.as_deref().unwrap_or(&name);
    let window_explicit = def.window.is_some() || cli.def.window.is_some();
    let mut eff = effective(instance_name, def, &cli);
    let configured_terminal = configured_terminal();
    apply_herdr_placement(
        &mut eff,
        catalogs.project_root.as_deref(),
        window_explicit,
        configured_terminal.as_deref(),
    );
    let (strategy, mut warnings) =
        choose_strategy(&eff, tmux_bin().is_some(), configured_terminal.as_deref());
    warnings.extend(eff.skill_warnings.iter().cloned());
    if let Some(error) = &eff.bundle_access_error {
        warnings.push(error.clone());
    }
    if let Some(error) = &eff.reasoning_error {
        warnings.push(error.clone());
    }

    println!("name:      {}", eff.name);
    if let Some(description) = &eff.description {
        println!("description: {description}");
    }
    println!("cli:       {}", eff.cli);
    println!("dir:       {}", eff.dir);
    for skill in &eff.skills {
        println!(
            "skill:     {} — {} ({})",
            skill.name, skill.description, skill.path
        );
    }
    if let Some(dir) = &eff.agent_dir {
        println!("bundle:    {dir}");
    }
    if let Some(path) = &eff.instructions {
        println!("instructions: {path}");
    }
    println!("start:     {}", if eff.resume { "resume" } else { "clean" });
    match &strategy {
        Strategy::Tmux { session, window } => {
            println!("terminal:  tmux {session}:{window} (--terminal here)")
        }
        Strategy::Custom(c) => println!("terminal:  HCOM_TERMINAL={c}"),
        Strategy::Direct(Some(t)) => println!("terminal:  preset {t}"),
        Strategy::Direct(None) => match configured_terminal.as_deref() {
            Some(terminal) if terminal != "default" => {
                println!("terminal:  preset {terminal} (hcom config default)")
            }
            _ => println!("terminal:  hcom default (hcom config terminal)"),
        },
    }
    if let Some(tag) = &eff.tag {
        println!("tag:       {tag}");
    }
    if !eff.groups.is_empty() {
        println!("groups:    {}", eff.groups.join(", "));
    }
    if let Some(m) = &eff.model {
        println!("model:     {m}");
    }
    if let Some(reasoning) = &eff.reasoning {
        println!("reasoning: {reasoning}");
    }
    if let Some(p) = &eff.pre {
        println!("pre:       {p}");
    }
    if !eff.env.is_empty() {
        println!(
            "env:       {}",
            eff.env
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    if !eff.extra.is_empty() {
        println!(
            "args:      {}",
            shell_words::join(eff.extra.iter().map(String::as_str))
        );
    }
    println!(
        "sources:   {}",
        catalogs
            .paths()
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for w in warnings {
        println!("warning:   {w}");
    }
    println!();
    println!("command:");
    match &strategy {
        Strategy::Tmux { .. } => println!("  {}", window_command(&eff, eff.resume)),
        Strategy::Custom(_) | Strategy::Direct(_) => {
            let terminal = match &strategy {
                Strategy::Direct(t) => t.clone(),
                _ => None,
            };
            let mut shown = vec![hcom_bin()];
            shown.extend(hcom_argv(&eff, terminal.as_deref(), eff.resume));
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
      "description": "what this agent is for, shown to other agents",
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
        let cwd = std::env::current_dir()?;
        find_project_hcom_dirs(&cwd)
            .pop()
            .unwrap_or_else(|| cwd.join(PROJECT_DIR))
            .join(PROJECT_FILE)
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
            "_hcom_agent() {\n  local cur=${COMP_WORDS[COMP_CWORD]}\n  if [ \"$COMP_CWORD\" -eq 2 ]; then\n    COMPREPLY=( $(compgen -W \"$(hcom agent list --names 2>/dev/null) $(hcom agent list --groups 2>/dev/null) list show attach edit completions\" -- \"$cur\") )\n  fi\n}\ncomplete -F _hcom_agent hcom\n"
        }
        "zsh" => {
            "#compdef hcom\n_hcom_agent_names() {\n  local -a names groups\n  names=(${(f)\"$(hcom agent list --names 2>/dev/null)\"})\n  groups=(${(f)\"$(hcom agent list --groups 2>/dev/null)\"})\n  compadd -a names\n  compadd -a groups\n}\ncompdef _hcom_agent_names hcom-agent\n"
        }
        "fish" => {
            "complete -c hcom -n '__fish_seen_subcommand_from agent' -f -a \"(hcom agent list --names 2>/dev/null) (hcom agent list --groups 2>/dev/null)\"\n"
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

    /// Exit 2 is "readiness not observed yet", not "start failed". Sends must
    /// survive it, or the payload is lost while the agent comes up fine.
    #[test]
    fn still_launching_is_not_an_autostart_failure() {
        assert_eq!(classify_autostart_exit(0), AutostartOutcome::Started);
        assert_eq!(classify_autostart_exit(2), AutostartOutcome::StillLaunching);
        assert_eq!(classify_autostart_exit(1), AutostartOutcome::Failed);
        assert_eq!(classify_autostart_exit(127), AutostartOutcome::Failed);
    }

    fn live_agent(name: &str, status: &str) -> LiveAgent {
        LiveAgent {
            name: name.to_string(),
            status: status.to_string(),
            tool: "codex".to_string(),
            directory: "/tmp/project".to_string(),
            launch_context: serde_json::json!({}),
        }
    }

    #[test]
    fn live_location_uses_recorded_terminal_context() {
        let mut live = live_agent("milo", "listening");
        live.launch_context = serde_json::json!({
            "terminal_preset_effective": "herdr",
            "pane_id": "wM:p1"
        });
        assert_eq!(live.location().as_deref(), Some(", herdr wM:p1"));
    }

    #[test]
    fn catalog_autostart_waits_for_deliverable_registration() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("hcom.db");
        let db = HcomDb::open_at(&db_path).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at)
                 VALUES ('reviewer', 'stopped', '', 1.0)",
                [],
            )
            .unwrap();

        let writer_path = db_path.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let writer_db = HcomDb::open_at(&writer_path).unwrap();
            writer_db
                .conn()
                .execute(
                    "UPDATE instances
                     SET status = 'inactive', status_context = 'new'
                     WHERE name = 'reviewer'",
                    [],
                )
                .unwrap();
        });

        wait_for_catalog_agent_registration(&db, "reviewer", std::time::Duration::from_secs(1))
            .unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn catalog_autostart_registration_timeout_rejects_stopped_row() {
        let tmp = tempfile::tempdir().unwrap();
        let db = HcomDb::open_at(&tmp.path().join("hcom.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, created_at)
                 VALUES ('reviewer', 'stopped', 1.0)",
                [],
            )
            .unwrap();

        let error = wait_for_catalog_agent_registration(&db, "reviewer", std::time::Duration::ZERO)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("did not register before timeout")
        );
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
    fn merge_accumulates_tool_args_and_replaces_tool_scalars() {
        let mut base = def_from(
            r#"{"tools":{"claude":{"model":"sonnet","reasoning":"low","args":["--a"]},"codex":{"model":"gpt-5"}}}"#,
        );
        let over =
            def_from(r#"{"tools":{"claude":{"model":"opus","reasoning":"high","args":["--b"]}}}"#);
        base.merge_from(&over);
        let claude = &base.tools["claude"];
        assert_eq!(claude.model.as_deref(), Some("opus"));
        assert_eq!(claude.reasoning.as_deref(), Some("high"));
        assert_eq!(claude.args, vec!["--a", "--b"]);
        assert_eq!(base.tools["codex"].model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn merge_replaces_system_prompt_and_empty_string_clears_it() {
        let mut def = def_from(r#"{"system_prompt":"global"}"#);
        def.merge_from(&def_from(r#"{"system_prompt":"project"}"#));
        assert_eq!(def.system_prompt.as_deref(), Some("project"));

        def.merge_from(&def_from(r#"{"system_prompt":""}"#));
        assert_eq!(effective("agent", def, &Cli::default()).system_prompt, None);
    }

    #[test]
    fn merge_accumulates_unique_groups() {
        let mut base = def_from(r#"{"groups":["all","review"]}"#);
        base.merge_from(&def_from(r#"{"groups":["review","backend"]}"#));
        assert_eq!(base.groups, vec!["all", "review", "backend"]);
    }

    #[test]
    fn merge_keeps_base_when_override_omits_field() {
        let mut base = def_from(r#"{"session":"wdt","resume":true}"#);
        base.merge_from(&def_from(r#"{"cli":"codex"}"#));
        assert_eq!(base.session.as_deref(), Some("wdt"));
        assert_eq!(base.resume, Some(true));
    }

    #[test]
    fn merge_replaces_resume_when_override_defines_it() {
        let mut base = def_from(r#"{"resume":true}"#);
        base.merge_from(&def_from(r#"{"resume":false}"#));
        assert_eq!(base.resume, Some(false));
    }

    #[test]
    fn merge_replaces_description_and_effective_collapses_it_to_one_line() {
        let mut base = def_from(r#"{"description":"old"}"#);
        base.merge_from(&def_from("{\"description\":\"new  desc\\n  second line\"}"));
        let eff = effective("a", base, &Cli::default());
        assert_eq!(eff.description.as_deref(), Some("new desc second line"));
    }

    #[test]
    fn blank_description_is_treated_as_unset() {
        let eff = effective("a", def_from(r#"{"description":"  "}"#), &Cli::default());
        assert_eq!(eff.description, None);
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = serde_json::from_str::<AgentDef>(r#"{"drr":"typo"}"#);
        assert!(err.is_err());
    }

    fn catalogs_from(pairs: &[(&str, &str, &str)]) -> Catalogs {
        // (label, base_dir, json)
        let files = pairs
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
            .collect::<Vec<_>>();
        let (project_files, base_files) =
            files.into_iter().partition(|file| file.label == "project");
        Catalogs {
            base_files,
            project_files,
            project_root: None,
        }
    }

    #[test]
    fn project_layer_overrides_global_and_can_add_agents() {
        let c = catalogs_from(&[
            (
                "global",
                "/home/u",
                r#"{"defaults":{"cli":"claude","terminal":"herdr"},"agents":{"a":{"dir":"/g","model":"opus"}}}"#,
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
            (
                "global",
                "/home/u",
                r#"{"defaults":{"cli":"claude"},"agents":{"only_global":{}}}"#,
            ),
            (
                "project",
                "/repo",
                r#"{"defaults":{"cli":"codex"},"agents":{"other":{}}}"#,
            ),
        ]);
        assert_eq!(
            c2.resolve("only_global").unwrap().cli.as_deref(),
            Some("claude")
        );
        assert_eq!(a.model.as_deref(), Some("gpt-5"));
        assert_eq!(a.dir, None, "project agent ignores same-named global entry");

        let b = c.resolve("b").unwrap();
        assert_eq!(
            b.dir.as_deref(),
            Some("/repo"),
            "project dir is relative to the catalog"
        );
        assert_eq!(
            b.terminal.as_deref(),
            Some("herdr"),
            "global defaults fill fields omitted by the project"
        );
        assert!(c.resolve("missing").is_none());
        assert_eq!(c.names().get("a").map(String::as_str), Some("project"));
    }

    #[test]
    fn relative_dir_resolves_against_its_own_catalog_base() {
        assert_eq!(expand_path("sub/x", Path::new("/base")), "/base/sub/x");
        assert_eq!(expand_path("/abs", Path::new("/base")), "/abs");
        assert_eq!(expand_path("./a/../b", Path::new("/base")), "/base/b");
    }

    #[test]
    fn bundle_instructions_follow_fixed_system_prompt() {
        let mut def = def_from(r#"{"dir":"/work","system_prompt":"fixed"}"#);
        def.agent_dir = Some("/work/.hcom/agents/reviewer".into());
        def.instructions = Some("/work/.hcom/agents/reviewer/SOUL.md".into());
        def.instructions_content = Some("learned".into());
        let eff = effective("reviewer", def, &Cli::default());
        let prompt = eff.system_prompt.unwrap();
        assert!(prompt.starts_with("fixed\n\n# Agent bundle instructions\n\nBundle directory: `/work/.hcom/agents/reviewer`\nInstruction file: `/work/.hcom/agents/reviewer/SOUL.md`"));
        assert!(prompt.ends_with("learned"));
        assert!(eff.bundle_args.is_empty(), "bundle is inside workspace");
    }

    #[test]
    fn skill_metadata_fallbacks_and_manifest_are_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("reviewer");
        std::fs::create_dir_all(bundle.join("skills/zeta")).unwrap();
        std::fs::create_dir_all(bundle.join("skills/alpha")).unwrap();
        std::fs::write(
            bundle.join("skills/zeta/SKILL.md"),
            "---\nname: inspect\ndescription: Inspect releases\n---\n# Ignored",
        )
        .unwrap();
        std::fs::write(bundle.join("skills/alpha/SKILL.md"), "# Alpha fallback").unwrap();

        let (skills, warnings) = discover_agent_skills(bundle.to_str().unwrap());
        assert_eq!(
            skills.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["alpha", "inspect"]
        );
        assert_eq!(skills[0].description, "Alpha fallback");
        assert_eq!(warnings.len(), 1);

        let prompt = build_agent_instructions(None, None, None, None, &skills).unwrap();
        assert!(prompt.starts_with("# Available agent skills"));
        assert!(prompt.find("## alpha").unwrap() < prompt.find("## inspect").unwrap());
        assert!(prompt.contains(&skills[0].path));
    }

    #[test]
    fn skill_metadata_accepts_windows_crlf() {
        let (name, description, valid) = parse_skill_metadata(
            "---\r\nname: windows\r\ndescription: Works everywhere\r\n---\r\n# Heading\r\n",
            "fallback",
        );
        assert!(valid);
        assert_eq!(name, "windows");
        assert_eq!(description, "Works everywhere");
    }

    #[test]
    fn duplicate_skill_names_warn_without_dropping_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("reviewer");
        for dir in ["one", "two"] {
            std::fs::create_dir_all(bundle.join("skills").join(dir)).unwrap();
            std::fs::write(
                bundle.join("skills").join(dir).join("SKILL.md"),
                "---\nname: same\ndescription: Shared\n---\n",
            )
            .unwrap();
        }
        let (skills, warnings) = discover_agent_skills(bundle.to_str().unwrap());
        assert_eq!(skills.len(), 2);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("duplicate skill name"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn skill_symlink_escape_is_skipped() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("reviewer");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(bundle.join("skills")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("SKILL.md"), "# Escape").unwrap();
        symlink(&outside, bundle.join("skills/escape")).unwrap();
        let (skills, warnings) = discover_agent_skills(bundle.to_str().unwrap());
        assert!(skills.is_empty());
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("outside bundle"))
        );
    }

    #[test]
    fn external_bundle_uses_cli_access_or_reports_unsupported_tool() {
        for (cli, expected) in [
            ("claude", vec!["--add-dir", "/agent/reviewer"]),
            ("codex", vec!["--add-dir", "/agent/reviewer"]),
            ("agy", vec!["--add-dir", "/agent/reviewer"]),
            ("antigravity", vec!["--add-dir", "/agent/reviewer"]),
            ("gemini", vec!["--include-directories", "/agent/reviewer"]),
            ("omp", vec!["--add-dir=/agent/reviewer"]),
            ("copilot", vec!["--add-dir=/agent/reviewer"]),
        ] {
            let mut def = def_from(&format!(r#"{{"dir":"/work","cli":"{cli}"}}"#));
            def.agent_dir = Some("/agent/reviewer".into());
            let eff = effective("reviewer", def, &Cli::default());
            assert_eq!(eff.bundle_args, expected, "cli={cli}");
            assert!(eff.bundle_access_error.is_none(), "cli={cli}");
        }

        for (cli, key) in [
            ("opencode", "OPENCODE_CONFIG_CONTENT"),
            ("kilo", "KILO_CONFIG_CONTENT"),
        ] {
            let mut def = def_from(&format!(r#"{{"dir":"/work","cli":"{cli}"}}"#));
            def.agent_dir = Some("/agent/reviewer".into());
            let eff = effective("reviewer", def, &Cli::default());
            let config: serde_json::Value = serde_json::from_str(&eff.env[key]).unwrap();
            assert_eq!(
                config["permission"]["external_directory"]["/agent/reviewer/**"],
                "allow"
            );
        }

        let mut pi_def = def_from(r#"{"dir":"/work","cli":"pi"}"#);
        pi_def.agent_dir = Some("/agent/reviewer".into());
        let pi = effective("reviewer", pi_def, &Cli::default());
        assert!(
            pi.bundle_access_error
                .as_deref()
                .is_some_and(|e| e.contains("pi"))
        );
    }

    #[test]
    fn bundle_discovery_rejects_invalid_agent_directory_name() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("agents/Bad-Name");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("SOUL.md"), "instructions").unwrap();
        let error = load_catalog_file(&tmp.path().join("agents.json"), tmp.path(), "test".into())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("invalid agent bundle name 'Bad-Name'"),
            "{error}"
        );
    }

    #[test]
    fn imports_selected_agents_and_keeps_source_relative_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let overlay = tmp.path().join("overlay");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&overlay).unwrap();
        std::fs::write(
            project.join(PROJECT_FILE),
            r#"{"defaults":{"cli":"codex"},"agents":{
                "wdt_main":{"dir":"work","model":"base"},
                "private":{"dir":"private"}}}"#,
        )
        .unwrap();
        std::fs::write(
            overlay.join("hermes.json"),
            r#"{"imports":[{"from":"../project/agents.json","agents":["wdt_main"]}],
                "agents":{"wdt_main":{"model":"overlay"},"hermes_local":{"dir":"."}}}"#,
        )
        .unwrap();

        let files = load_catalog_tree(
            &overlay.join("hermes.json"),
            &overlay,
            "extra".to_string(),
            &mut Vec::new(),
        )
        .unwrap();
        let catalogs = Catalogs {
            base_files: files,
            project_files: Vec::new(),
            project_root: None,
        };
        let wdt = catalogs.resolve("wdt_main").unwrap();
        assert_eq!(wdt.cli.as_deref(), Some("codex"));
        assert_eq!(wdt.model.as_deref(), Some("overlay"));
        assert_eq!(
            wdt.dir.as_deref(),
            Some(project.join("work").to_string_lossy().as_ref())
        );
        assert!(catalogs.resolve("private").is_none());
        assert_eq!(
            catalogs.resolve("hermes_local").unwrap().dir.as_deref(),
            Some(overlay.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn group_loading_sees_agents_hidden_by_selective_imports() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.json");
        let root = tmp.path().join("root.json");
        std::fs::write(
            &source,
            r#"{"agents":{"public":{"groups":["crew"]},"private":{"groups":["crew"]}}}"#,
        )
        .unwrap();
        std::fs::write(
            &root,
            r#"{"imports":[{"from":"source.json","agents":["public"]}]}"#,
        )
        .unwrap();

        let visible = load_catalog_tree(&root, tmp.path(), "root".into(), &mut Vec::new()).unwrap();
        let all =
            load_catalog_tree_with_mode(&root, tmp.path(), "root".into(), &mut Vec::new(), true)
                .unwrap();
        let visible = Catalogs {
            base_files: visible,
            project_files: Vec::new(),
            project_root: None,
        };
        let all = Catalogs {
            base_files: all,
            project_files: Vec::new(),
            project_root: None,
        };

        assert_eq!(visible.group_members("crew"), vec!["public"]);
        assert_eq!(all.group_members("crew"), vec!["private", "public"]);
    }

    #[test]
    fn invalid_group_name_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("agents.json");
        std::fs::write(&path, r#"{"agents":{"a":{"groups":["Bad-Group"]}}}"#).unwrap();
        let error = load_catalog_file(&path, tmp.path(), "test".into())
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid agent group 'Bad-Group'"), "{error}");
    }

    #[test]
    fn imported_project_catalog_resolves_dirs_from_parent_of_hcom() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let project_hcom = project.join(PROJECT_DIR);
        let overlay = tmp.path().join("overlay");
        std::fs::create_dir_all(&project_hcom).unwrap();
        std::fs::create_dir_all(&overlay).unwrap();
        std::fs::write(
            project_hcom.join(PROJECT_FILE),
            r#"{"agents":{"wdt_main":{"dir":"."}}}"#,
        )
        .unwrap();
        std::fs::write(
            overlay.join("agents.json"),
            format!(
                r#"{{"imports":[{{"from":{}}}]}}"#,
                serde_json::to_string(&project_hcom.join(PROJECT_FILE)).unwrap()
            ),
        )
        .unwrap();

        let files = load_catalog_tree(
            &overlay.join("agents.json"),
            &overlay,
            "global".to_string(),
            &mut Vec::new(),
        )
        .unwrap();
        let catalogs = Catalogs {
            base_files: files,
            project_files: Vec::new(),
            project_root: None,
        };
        assert_eq!(
            catalogs.resolve("wdt_main").unwrap().dir.as_deref(),
            Some(project.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn imports_are_transitive_and_cycles_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.json");
        let b = tmp.path().join("b.json");
        let c = tmp.path().join("c.json");
        std::fs::write(&a, r#"{"imports":[{"from":"b.json"}],"agents":{"a":{}}}"#).unwrap();
        std::fs::write(&b, r#"{"imports":[{"from":"c.json"}],"agents":{"b":{}}}"#).unwrap();
        std::fs::write(&c, r#"{"agents":{"c":{}}}"#).unwrap();

        let files = load_catalog_tree(&a, tmp.path(), "root".to_string(), &mut Vec::new()).unwrap();
        let catalogs = Catalogs {
            base_files: files,
            project_files: Vec::new(),
            project_root: None,
        };
        assert_eq!(
            catalogs.names().keys().cloned().collect::<Vec<_>>(),
            ["a", "b", "c"]
        );

        std::fs::write(&c, r#"{"imports":[{"from":"a.json"}],"agents":{"c":{}}}"#).unwrap();
        let error = load_catalog_tree(&a, tmp.path(), "root".to_string(), &mut Vec::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("catalog import cycle"), "{error}");
        assert!(error.contains("a.json"), "{error}");
    }

    #[test]
    fn selected_import_rejects_unknown_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.json");
        let root = tmp.path().join("root.json");
        std::fs::write(&source, r#"{"agents":{"known":{}}}"#).unwrap();
        std::fs::write(
            &root,
            r#"{"imports":[{"from":"source.json","agents":["missing"]}]}"#,
        )
        .unwrap();
        let error = load_catalog_tree(&root, tmp.path(), "root".to_string(), &mut Vec::new())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("requests unknown agent(s): missing"),
            "{error}"
        );
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
            &[
                "--cli",
                "codex",
                "--model",
                "gpt-5",
                "--dangerously-skip-permissions",
            ],
        );
        assert_eq!(eff.cli, "codex");
        assert_eq!(eff.model.as_deref(), Some("gpt-5"));
        assert_eq!(
            eff.extra,
            vec!["--from-catalog", "--dangerously-skip-permissions"]
        );
    }

    #[test]
    fn effective_cli_selects_matching_tool_profile() {
        let json = r#"{
            "cli":"claude",
            "model":"legacy",
            "reasoning":"low",
            "prompt":"shared",
            "args":["--common"],
            "tools":{
                "claude":{"model":"sonnet","reasoning":"high","prompt":"claude prompt","args":["--agent","reviewer"]},
                "codex":{"model":"gpt-5","reasoning":"xhigh","system_prompt":"codex system","args":["--sandbox","workspace-write"]}
            }
        }"#;

        let claude = eff_of(json, &[]);
        assert_eq!(claude.cli, "claude");
        assert_eq!(claude.model.as_deref(), Some("sonnet"));
        assert_eq!(claude.reasoning.as_deref(), Some("high"));
        assert_eq!(claude.prompt.as_deref(), Some("claude prompt"));
        assert_eq!(claude.extra, vec!["--common", "--agent", "reviewer"]);

        let codex = eff_of(json, &["--cli", "codex"]);
        assert_eq!(codex.cli, "codex");
        assert_eq!(codex.model.as_deref(), Some("gpt-5"));
        assert_eq!(codex.reasoning.as_deref(), Some("xhigh"));
        assert_eq!(codex.prompt.as_deref(), Some("shared"));
        assert_eq!(codex.system_prompt.as_deref(), Some("codex system"));
        assert_eq!(
            codex.extra,
            vec!["--common", "--sandbox", "workspace-write"]
        );
    }

    #[test]
    fn command_line_overrides_selected_tool_profile() {
        let eff = eff_of(
            r#"{"cli":"claude","tools":{"codex":{"model":"profile","reasoning":"high","prompt":"profile prompt","args":["--profile"]}}}"#,
            &[
                "--cli",
                "codex",
                "--model",
                "cli",
                "--reasoning",
                "xhigh",
                "--hcom-prompt",
                "cli prompt",
                "--extra",
            ],
        );
        assert_eq!(eff.model.as_deref(), Some("cli"));
        assert_eq!(eff.reasoning.as_deref(), Some("xhigh"));
        assert_eq!(eff.prompt.as_deref(), Some("cli prompt"));
        assert_eq!(eff.extra, vec!["--profile", "--extra"]);
    }

    #[test]
    fn double_dash_forwards_everything_verbatim() {
        let eff = eff_of(r#"{"dir":"/w"}"#, &["--", "--cli", "not-consumed"]);
        assert_eq!(eff.cli, DEFAULT_CLI);
        assert_eq!(eff.extra, vec!["--cli", "not-consumed"]);
    }

    #[test]
    fn window_defaults_to_agent_name_and_empty_session_disables_mux() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt","terminal":"tmux"}"#, &[]);
        assert_eq!(eff.window, "wdt_main");
        assert_eq!(eff.session.as_deref(), Some("wdt"));

        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &["--session", ""]);
        assert!(eff.session.is_none());
    }

    #[test]
    fn strategy_prefers_terminal_command_then_mux_then_preset() {
        let eff = eff_of(r#"{"dir":"/w","terminal_command":"myterm {script}"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true, None);
        assert_eq!(s, Strategy::Custom("myterm {script}".into()));

        let eff = eff_of(r#"{"dir":"/w","session":"wdt","terminal":"tmux"}"#, &[]);
        let (s, w) = choose_strategy(&eff, true, None);
        assert_eq!(
            s,
            Strategy::Tmux {
                session: "wdt".into(),
                window: "wdt_main".into()
            }
        );
        assert!(w.is_empty());

        let eff = eff_of(r#"{"dir":"/w","terminal":"wezterm-tab"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true, None);
        assert_eq!(s, Strategy::Direct(Some("wezterm-tab".into())));

        let eff = eff_of(r#"{"dir":"/w"}"#, &[]);
        let (s, _) = choose_strategy(&eff, true, None);
        assert_eq!(s, Strategy::Direct(None));
    }

    #[test]
    fn session_falls_back_to_preset_without_tmux() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt","terminal":"tmux"}"#, &[]);
        let (s, warnings) = choose_strategy(&eff, false, None);
        assert_eq!(s, Strategy::Direct(Some("tmux".into())));
        assert!(warnings[0].contains("tmux not found"));
    }

    #[test]
    fn session_with_non_mux_preset_warns_and_falls_back() {
        let eff = eff_of(
            r#"{"dir":"/w","session":"wdt","terminal":"kitty-tab"}"#,
            &[],
        );
        let (s, warnings) = choose_strategy(&eff, true, None);
        assert_eq!(s, Strategy::Direct(Some("kitty-tab".into())));
        assert!(warnings[0].contains("not a multiplexer"));
    }

    #[test]
    fn herdr_uses_session_as_workspace_and_window_as_tab() {
        let mut eff = eff_of(
            r#"{"dir":"/work/repo","session":"repo","window":"review","terminal":"herdr"}"#,
            &[],
        );
        apply_herdr_placement(&mut eff, Some(Path::new("/work/repo")), true, None);
        assert_eq!(
            eff.env.get("HCOM_HERDR_WORKSPACE").map(String::as_str),
            Some("repo")
        );
        assert_eq!(
            eff.env.get("HCOM_HERDR_TAB").map(String::as_str),
            Some("review")
        );
        let (strategy, warnings) = choose_strategy(&eff, false, None);
        assert_eq!(strategy, Strategy::Direct(Some("herdr".into())));
        assert!(warnings.is_empty());
    }

    #[test]
    fn herdr_defaults_to_project_space_and_agent_tab() {
        let mut eff = eff_of(r#"{"dir":"/work/repo","terminal":"herdr"}"#, &[]);
        apply_herdr_placement(&mut eff, Some(Path::new("/work/repo")), false, None);
        assert_eq!(eff.session.as_deref(), Some("repo"));
        assert_eq!(eff.window, eff.name);
    }

    #[test]
    fn configured_herdr_overrides_parent_placement() {
        let mut eff = eff_of(
            r#"{"dir":"/work/repo","session":"child-space","env":{"HCOM_HERDR_WORKSPACE":"parent-space","HCOM_HERDR_TAB":"parent-tab"}}"#,
            &[],
        );
        apply_herdr_placement(
            &mut eff,
            Some(Path::new("/work/repo")),
            false,
            Some("herdr"),
        );
        assert_eq!(
            eff.env.get("HCOM_HERDR_WORKSPACE").map(String::as_str),
            Some("child-space")
        );
        assert_eq!(
            eff.env.get("HCOM_HERDR_TAB").map(String::as_str),
            Some("wdt_main")
        );
    }

    #[test]
    fn session_without_terminal_does_not_select_tmux() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &[]);
        let (s, warnings) = choose_strategy(&eff, true, None);
        assert_eq!(s, Strategy::Direct(None));
        assert!(warnings[0].contains("terminal preset 'default'"));
    }

    #[test]
    fn session_uses_configured_herdr_without_catalog_terminal() {
        let eff = eff_of(r#"{"dir":"/w","session":"wdt"}"#, &[]);
        let (strategy, warnings) = choose_strategy(&eff, true, Some("herdr"));
        assert_eq!(strategy, Strategy::Direct(None));
        assert!(warnings.is_empty());
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
                "--go",
                "--model",
                "gpt-5",
                "--foo",
            ]
        );
    }

    #[test]
    fn hcom_argv_translates_reasoning_for_supported_clis() {
        let claude = eff_of(r#"{"cli":"claude","reasoning":"high"}"#, &[]);
        assert!(
            hcom_argv(&claude, None, false)
                .ends_with(&["--effort".to_string(), "high".to_string(),])
        );

        let agy = eff_of(r#"{"cli":"agy","reasoning":"medium"}"#, &[]);
        assert!(
            hcom_argv(&agy, None, false)
                .ends_with(&["--effort".to_string(), "medium".to_string(),])
        );

        let codex = eff_of(r#"{"cli":"codex","reasoning":"xhigh"}"#, &[]);
        assert!(hcom_argv(&codex, None, false).ends_with(&[
            "-c".to_string(),
            "model_reasoning_effort=\"xhigh\"".to_string(),
        ]));
    }

    #[test]
    fn unsupported_cli_reports_reasoning_error() {
        let eff = eff_of(r#"{"cli":"gemini","reasoning":"high"}"#, &[]);
        assert_eq!(
            eff.reasoning_error.as_deref(),
            Some(
                "reasoning is not supported for cli 'gemini'; use tools.gemini.args for a tool-specific setting"
            )
        );
    }

    #[test]
    fn skills_dir_has_targeted_migration_error() {
        let error = parse_cli(&["--skills-dir".into(), "/old".into()])
            .err()
            .expect("legacy flag rejected");
        assert!(
            error
                .to_string()
                .contains("agents/<name>/skills/<skill>/SKILL.md")
        );
    }

    #[test]
    fn start_mode_defaults_clean_and_cli_flags_override() {
        let parse = |args: &[&str]| {
            parse_cli(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
        };
        assert_eq!(parse(&[]).resume, None);
        assert_eq!(parse(&["--resume"]).resume, Some(true));
        assert_eq!(parse(&["--clean"]).resume, Some(false));
        assert_eq!(parse(&["--resume", "--clean"]).resume, Some(false));
        assert_eq!(parse(&["--clean", "--resume"]).resume, Some(true));
        // Start-mode flags are ours, not something the tool should see.
        assert!(parse(&["--resume"]).passthrough.is_empty());
        assert!(parse(&["--clean"]).passthrough.is_empty());
    }

    #[test]
    fn effective_start_mode_uses_catalog_then_cli_override() {
        assert!(!eff_of(r#"{}"#, &[]).resume, "default must stay clean");
        assert!(eff_of(r#"{"resume":true}"#, &[]).resume);
        assert!(!eff_of(r#"{"resume":true}"#, &["--clean"]).resume);
        assert!(eff_of(r#"{"resume":false}"#, &["--resume"]).resume);
    }

    #[test]
    fn resume_argv_uses_r_and_go() {
        let eff = eff_of(r#"{"dir":"/w","cli":"codex","reasoning":"xhigh"}"#, &[]);
        let argv = hcom_argv(&eff, Some("here"), true);
        assert_eq!(argv[0], "r");
        assert_eq!(argv[1], "wdt_main");
        assert!(argv.contains(&"--go".to_string()));
        assert!(!argv.contains(&"--as".to_string()));
        assert!(argv.ends_with(&[
            "-c".to_string(),
            "model_reasoning_effort=\"xhigh\"".to_string(),
        ]));
    }

    #[test]
    fn fresh_argv_uses_go() {
        let eff = eff_of(r#"{"dir":"/w","cli":"codex"}"#, &[]);
        let argv = hcom_argv(&eff, Some("here"), false);
        assert!(argv.contains(&"--go".to_string()));
    }

    #[test]
    fn window_command_carries_env_pre_and_keeps_pane_alive() {
        let eff = eff_of(
            r#"{"dir":"/w","env":{"AWS_PROFILE":"wdt"},"pre":"source .venv/bin/activate"}"#,
            &[],
        );
        let cmd = window_command(&eff, false);
        assert!(cmd.starts_with("exec \"${SHELL:-/bin/bash}\" -lic "));
        assert!(cmd.contains("source .venv/bin/activate && AWS_PROFILE=wdt "));
        assert!(cmd.contains("--terminal here"));
        assert!(cmd.contains("; exec \"${SHELL:-/bin/bash}\" -l"));
    }

    #[test]
    fn closest_suggests_near_miss_only() {
        let names = ["wdt_main".to_string(), "gtm_cli".to_string()];
        assert_eq!(
            closest("wdt_mian", names.iter()).as_deref(),
            Some("wdt_main")
        );
        assert_eq!(closest("zzzzzzzzzz", names.iter()), None);
    }

    #[test]
    fn help_starts_with_usage() {
        let help = help_text();
        assert!(help.starts_with("Usage:"));
        assert!(help.contains("--reasoning <val>"));
        assert!(help.contains("model_reasoning_effort for Codex"));
        assert!(help.contains("not a parent agent's location"));
    }
}

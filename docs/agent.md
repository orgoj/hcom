# Named agents

`hcom agent` launches recurring agents from JSON catalogs and editable bundles. A catalog stores launch settings; `agents/<name>/SOUL.md` stores evolving instructions and references supporting files in the same bundle.

```bash
hcom agent wdt_main                 # launch, or report that it is already running
hcom agent @wdt                     # launch every member of the wdt catalog group
hcom agent wdt_main --as wdt_review # same config, independent instance named wdt_review
hcom agent wdt_main --cli codex     # override the configured CLI
hcom agent list                     # catalog entries, effective CLI/model, status, and source
hcom agent list --for-agents        # only names and descriptions, for another agent to read
hcom agent list @wdt                # show only members of the wdt catalog group
hcom agent list --all               # include agents hidden by recursive selective imports
hcom agent list --local             # only direct and imported agents from this project
hcom agent list --json              # full JSON output with effective config and status
hcom agent list --names             # output agent names only
hcom agent list --groups            # output catalog group names only
hcom agent show wdt_main            # effective configuration and exact command
hcom agent attach wdt_main          # focus its managed terminal window
hcom agent edit                     # edit the global catalog
hcom agent edit --project           # edit the current project's catalog
hcom agent completions bash         # output shell completions for bash, zsh, or fish
```

Run `hcom agent --help` for all flags. `--dry-run` prints commands without launching them.

## Configured agents and running instances

Two separate registries answer two different questions:

| question | command |
|---|---|
| who is running right now | `hcom list` |
| who can be addressed at all | `hcom agent list` |

`hcom list` shows live instances only, so a configured agent missing from it is stopped, not unknown. Address it by its exact name: `hcom send @<name>` starts a missing or stopped configured agent before delivering. A name never resolves to a similar one: `bin` is the agent `bin`, not a running `project1_bin`. If the agent is still starting when the readiness wait ends, the message is queued and delivered once the agent is ready; the send still succeeds.

## Catalogs and precedence

hcom resolves effective values through this chain, from weakest to strongest, regardless of the
directory where the command runs:

1. built-in defaults
2. `defaults` in `~/.hcom/agents.json`
3. the matching catalog's `defaults`
4. the named agent entry
5. the matching `tools.<effective-cli>` profile
6. command-line flags

Set `HCOM_AGENTS_FILE` to replace the global catalog path. `HCOM_AGENT_CATALOGS` is a
platform-specific path-separated list of additive catalogs; it does not replace the global one.

Steps 3-4 repeat for every matching imported, additive, or project catalog in catalog order.
Imports are recursive and apply before the importing file's local entries. Additive catalogs apply
left to right.

The project search walks parent directories until it finds the nearest `.hcom`, independently of
Git boundaries. The global `~/.hcom` is never a project scope. A project agent ignores a same-named
global/additive entry, but global `defaults` remain its lowest catalog layer. Consequently, the
same project agent inherits the same global defaults whether hcom runs inside the project or from
an external catalog that imports it.

`env`, `args`, `groups`, and tool-profile definitions merge; later scalar values replace earlier
ones. In particular, a later `system_prompt` replaces rather than appends to the earlier text, and
an explicit empty string clears it. Relative `dir` paths use `$HOME` in the global catalog, the
directory containing `.hcom` in project catalogs (including when imported), and the catalog
directory in other imported/additive catalogs. hcom expands `~` and environment variables.

Keep paths in project catalogs relative to the project root. Do not store machine-specific
absolute paths in a versioned `.hcom/agents.json`; use absolute or `~`-based paths only in
machine-local catalogs such as `~/.hcom/agents.json`.

Global catalog `defaults` apply to every agent. Project defaults apply only to project agents and
override global defaults field by field.

Unknown catalog fields are errors. This catches misspelled configuration instead of silently
ignoring it.

## Agent bundles

Global and project layouts are identical:

```text
~/.hcom/                         <project>/.hcom/
├── agents.json                 ├── agents.json
└── agents/                     └── agents/
    └── reviewer/                   └── reviewer/
        ├── SOUL.md                     ├── SOUL.md
        └── skills/                     └── skills/
            └── review/                     └── review/
                └── SKILL.md                    └── SKILL.md
```

`agents/<name>/SOUL.md` defines an agent even without a matching JSON entry. A JSON-only agent
also remains valid. Discovered directory names must contain only lowercase letters, numbers, and
underscores. Bundle `AGENTS.md` files are not read as a fallback.

The effective `agent_instructions` contains the fixed JSON `system_prompt`, then an `# Agent bundle
instructions` section with absolute bundle and `SOUL.md` paths and its current contents, then an
`# Available agent skills` manifest. Empty sections are omitted. `prompt` remains the initial user
message. hcom rereads `SOUL.md` and skills on every clean start and named-agent resume, so an
agent may improve its bundle for its next launch. Relative references resolve from the bundle or
skill directory named in the manifest.

If a bundle is outside the working directory, hcom grants only that directory through the CLI's
additional-workspace mechanism. Launch fails clearly when the CLI cannot make it writable at
startup. Antigravity (`agy` or `antigravity`) receives the bundle through its repeatable
`--add-dir` option.

## Example: project fleet with local memory

A machine catalog can keep general-purpose agents alongside selective imports from project
catalogs. In this example, `general_research` is available everywhere, while only the project's
coordinator is published outside its repository:

```jsonc
{
  "version": 1,
  "imports": [
    {
      "from": "~/work/example-project/.hcom/agents.json",
      "agents": ["project_main"]
    }
  ],
  "agents": {
    "general_research": {
      "description": "General web and documentation research",
      "dir": "~/work/research",
      "cli": "antigravity"
    },
    "agent_coach": {
      "description": "Maintain local agent catalogs and bundles",
      "dir": "~",
      "cli": "claude"
    }
  }
}
```

The project catalog keeps its specialist roles private to the project. Different roles can use
different CLIs without changing how they communicate:

```jsonc
{
  "version": 1,
  "defaults": {
    "dir": ".",
    "session": "example-project",
    "system_prompt": "Read NOTES.md when resuming unfinished work. Consult relevant MEMORY/*.md files before related tasks. Keep portable project knowledge in the repository, not in bundle memory."
  },
  "agents": {
    "project_main": {
      "description": "Coordinate project work and delegate specialist tasks",
      "cli": "claude"
    },
    "project_research": {
      "description": "Research project-specific external sources",
      "cli": "antigravity"
    },
    "project_senior": {
      "description": "Review architecture and high-risk changes",
      "cli": "codex"
    },
    "project_security": {
      "description": "Review security-sensitive changes and findings",
      "cli": "codex"
    }
  }
}
```

A bundle may use a small, file-based memory convention:

```text
.hcom/agents/project_main/
├── SOUL.md                 # stable role, boundaries, and critical rules
├── NOTES.md                # current local work and decisions
└── MEMORY/
    └── release-checks.md   # longer lessons recalled for related tasks
```

hcom automatically injects `SOUL.md`; `NOTES.md` and `MEMORY/` are ordinary files, not special
hcom features. The catalog `system_prompt` or `SOUL.md` must tell the agent when to read them.
Clean starts and named-agent resumes reread the bundle, while an already-running session retains
the prompt it started with.

### Safe unattended execution with Dippy

When multiple agents run in parallel and execute shell commands, interactive approval prompts can cause severe click fatigue, while running completely unrestricted carries obvious safety risks.

A recommended solution is [Dippy](https://github.com/orgoj/Dippy), an approval firewall for AI CLI tools built on a full Bash AST parser (`parable`). Unlike naive regexes, Dippy analyzes the complete syntactic tree (pipelines, subshells, compound commands, and option flags) against declarative allowrules.

In a recurring agent fleet, this enables a self-improving autonomy loop:
- Safe, vetted commands execute unattended with zero prompts.
- Interactive approval is requested only for genuinely unvetted or sensitive commands.
- Meta-agents (e.g. a dedicated tooling developer or coach) can analyze execution audit logs to refine allowrules, progressively increasing fleet autonomy without sacrificing safety.

## Catalog format

```jsonc
{
  "version": 1,
  "defaults": {
    "cli": "claude",
    "resume": false
  },
  "agents": {
    "wdt_main": {
      "description": "WDT infrastructure: Ansible roles, deployments, runbooks",
      "dir": "~/work/wdt/ansible-wdt",
      "cli": "codex",
      "session": "wdt",
      "window": "review",
      "terminal": "wezterm-tab",
      "groups": ["wdt", "review"],
      "env": { "AWS_PROFILE": "wdt" },
      "pre": "source .venv/bin/activate",
      "prompt": "Review the current change",
      "system_prompt": "Concentrate on deployment safety",
      "tools": {
        "claude": {
          "model": "sonnet",
          "args": ["--agent", "reviewer"]
        },
        "codex": {
          "model": "gpt-5.4",
          "args": ["--sandbox", "workspace-write"]
        }
      }
    }
  }
}
```

Supported agent fields:

| Field | Purpose |
|---|---|
| `description` | One line on what the agent is for, shown to other agents |
| `dir` | CLI working directory |
| `cli` | CLI selected by default |
| `terminal` | hcom terminal preset, or `here` |
| `terminal_command` | Raw terminal command containing `{script}` |
| `session` | tmux session or Herdr workspace; ignored by other terminals |
| `window` | tmux window or Herdr tab; defaults to the instance name |
| `tag` | hcom group tag |
| `groups` | Catalog-only groups used by `hcom agent @<group>` |
| `model` | Default model passed to the selected CLI |
| `reasoning` | Reasoning effort for Claude, Antigravity, or Codex |
| `prompt` | Initial user prompt |
| `system_prompt` | Additional system prompt |
| `pre` | Shell command run before the CLI |
| `resume` | Resume the previous session by default |
| `env` | Environment variables merged by key |
| `args` | Additional CLI arguments |
| `tools.<cli>` | Per-CLI `model`, `reasoning`, `prompt`, `system_prompt`, and `args` overrides |

The selected `tools.<cli>` profile replaces shared scalar values and appends its `args`. Command-line flags override both. Top-level `model`, `reasoning`, and `args` remain useful for agents that always use one CLI.

`reasoning` maps to `--effort` for Claude and Antigravity (`agy`), and to Codex's `model_reasoning_effort`. Other CLIs reject the field at launch; use `tools.<cli>.args` when that CLI has its own reasoning control. `--reasoning` overrides the catalog value.

`groups` is independent of `tag`: it does not change runtime display names, message routing, or `hcom kill tag:...`. An agent can belong to multiple catalog groups. Group membership merges additively across catalog layers and duplicate names are ignored. Group names use lowercase letters, numbers, and underscores.

## Command-line flags

`hcom agent <name>` and `hcom agent show <name>` accept the following flags:

| Flag | Purpose |
|---|---|
| `--cli <tool>`, `--tool <tool>` | Select CLI (`claude`, `codex`, `gemini`, `opencode`, `kilo`, `pi`, `omp`, `antigravity`, `cursor`, `kimi`, `copilot`, `hermes`) |
| `--dir <path>` | Working directory |
| `--terminal <preset>` | Terminal preset, or `here` |
| `--terminal-command <cmd>` | Raw terminal command with `{script}` (sets `HCOM_TERMINAL`) |
| `--session <name>` | tmux session or Herdr workspace (empty string disables) |
| `--window <name>` | tmux window or Herdr tab (defaults to agent name) |
| `--as <name>` | Launch under an alternate runtime instance name |
| `--tag <val>` | Runtime group tag forwarded to hcom |
| `--model <val>` | Forwarded model name |
| `--reasoning <val>` | Reasoning effort (Claude, Antigravity, and Codex) |
| `--hcom-prompt <text>` | Initial user prompt (overrides catalog `prompt`) |
| `--hcom-system-prompt <text>` | Invocation-local instructions (overrides catalog `system_prompt`) |
| `--pre <cmd>` | Shell command run before the agent starts |
| `--env KEY=VALUE` | Extra environment variable (repeatable) |
| `--catalog <path>` | Use this file instead of the project catalog |
| `--no-project` | Ignore nearest project `.hcom/agents.json` |
| `--attach` | Focus window after launching |
| `--restart` | Kill running agent first instead of reporting it |
| `--resume` | Continue previous session |
| `--clean` | Start clean session (overrides configured resume) |
| `--dry-run` | Print commands without launching anything |
| `[tool-args...]` | Arguments after `--` or unparsed flags are forwarded to the CLI |

`hcom agent list` accepts the following options:

| Option | Purpose |
|---|---|
| `@<group>` | Show only members of one catalog group |
| `--all` | Include agents from all reachable imports, including selective omissions |
| `--local` | Show only direct and imported agents from the current project |
| `--json` | Output full JSON array with effective configurations and live statuses |
| `--names` | Output agent names only, one per line |
| `--groups` | Output reachable catalog group names (`@<group>`), one per line |
| `--for-agents` | Output `<name>  <description>` format only (default for non-terminals and pipes) |
| `--for-humans` | Output full table even when stdout is not a terminal |

## Imports

A catalog can import all agents or a selected subset from another catalog:

```json
{
  "imports": [
    {
      "from": "~/work/wdt/ansible-wdt/.hcom/agents.json",
      "agents": ["wdt_main"]
    },
    { "from": "../shared/.hcom/agents.json" }
  ],
  "agents": {
    "local_override": { "dir": ".", "cli": "codex" }
  }
}
```

Omitting `agents` imports all entries; an empty list imports none. Relative `from` paths use the importing catalog's directory. Imports are recursive, load before local entries, preserve the source catalog's defaults and path base, and reject cycles or unknown selected agents. Every imported or additive catalog discovers bundles in an `agents/` directory beside that catalog file; bundle-only agents participate in selective imports normally.

For normal listing, direct launch, and message routing, an import's `agents` list remains a visibility boundary. `hcom agent list --all` and `hcom agent @<group>` traverse every recursively reachable import and consider all agents, including entries omitted by a selective import. `--all` also works with `--names` and `--json`. This lets a global catalog act as a registry from which agents can be inspected or a project group can be started without switching to that project's directory. hcom does not scan the filesystem for unrelated project catalogs; they must be reachable through an import.

Addressability follows that same boundary. An agent defined only in a project catalog can be launched and messaged from inside its project, and from anywhere else only if a catalog in scope imports it. A selective import publishes exactly the listed agents across repositories and keeps the project's other agents private to that project. An unreachable name is reported as unknown with a note that catalog scope depends on the directory; hcom does not disclose to a caller out of scope whether the name exists elsewhere or in which catalog.

`hcom agent list --local` limits any listing format to the nearest project's direct and imported agents, excluding global and additive catalogs. Combine it with `--all` to include project-imported agents hidden by selective imports.

The table produced by `hcom agent list` shows each agent's effective CLI and model. An unset model appears as `-`; JSON output includes it as `model: null`. Per-CLI tool profiles are resolved before displaying values.

`hcom agent list --for-agents` prints one `<name>  <description>` line per agent (no CLI, model, directory, terminal, or status). It is the listing meant for another agent deciding where to delegate; an agent without a `description` shows `-`. Without either flag, an interactive terminal receives the table and any other output (a pipe, an agent shell) receives `--for-agents`; `--for-humans` forces the table. `--json`, `--names`, and `--groups` are unaffected. A multi-line catalog description is collapsed to one line on output.

## Private agent skills

Private skills live under `<bundle>/skills/`. Each immediate child directory containing `SKILL.md` is listed in the common lazy-loading manifest; hcom does not register it through a CLI-specific skill system. User, project, and plugin skills remain available.

Use one child directory per skill:

```text
.hcom/agents/wdt_main/skills/
├── review/
│   ├── SKILL.md
│   └── references/
└── release/
    ├── SKILL.md
    └── scripts/
```

YAML frontmatter supplies `name` and `description`. Missing or malformed values fall back to the directory basename and first Markdown heading, then to `Agent-local skill; read SKILL.md for details.` `hcom agent show` warns about malformed metadata, duplicate names, and skipped symlinks that escape the bundle. `hcom agent list --json` exposes `{name, description, path}` entries.

Legacy `skills_dir` and `--skills-dir` are unsupported. Move each skill to `agents/<name>/skills/<skill>/SKILL.md`; old configurations produce a migration error.

## Instruction transport

| CLI | Invocation-local transport |
|---|---|
| Claude / Claude PTY | `--append-system-prompt` |
| Codex | `-c developer_instructions=...` |
| Gemini | per-instance `GEMINI_SYSTEM_MD` file |
| Pi | `--append-system-prompt` |
| OMP | `--append-system-prompt=...` |
| OpenCode / Kilo | per-instance Markdown path merged into inline `instructions` |
| Copilot | per-instance `COPILOT_CUSTOM_INSTRUCTIONS_DIRS` |
| Hermes | `HERMES_EPHEMERAL_SYSTEM_PROMPT` |
| Antigravity / Cursor / Kimi | marked fallback in the existing one-time hcom bootstrap |

Per-instance files are stored under `$HCOM_DIR/system-prompts/<tool>/<instance>/`. The fallback is an approximation: it is subordinate to genuine system/developer messages, though agents are instructed to prioritize it above ordinary task text. It is not emitted as a separate user task or meta-turn.

Use `hcom agent show <name> --cli <tool>` to inspect the exact effective command before launch.

## Starting, resuming, and messaging

The default start mode is clean. Set `"resume": true` in defaults or an agent entry to continue its stopped session. `--resume` and `--clean` override the catalog; if both occur, the last one wins. `--restart` first replaces a running instance, then applies the selected start mode.

An hcom-managed agent can invoke another AI CLI directly. The child inherits the parent environment, but hcom rejects hooks whose actual CLI does not match the tool bound to the inherited process identity. This keeps nested CLI sessions from replacing the parent's session, transcript, or delivery binding.

An instance name is unique. Launching an already-running name reports its status and exits without opening another instance. To run one catalog definition concurrently, assign each instance a different name:

```bash
hcom agent wdt_main --as wdt_review
hcom agent wdt_main --as wdt_backend
hcom agent show wdt_main --as wdt_review # inspect the aliased command
```

`wdt_main` remains the catalog key used to resolve configuration. The `--as` value becomes the runtime identity used by `hcom list`, messages, duplicate detection, `--restart`, `--attach`, and resume history. It is also the default tmux window or Herdr tab name; an explicit catalog or CLI `window` still takes precedence. Address an aliased instance directly, for example `hcom send @wdt_review -- "Review this"`. Catalog-driven message startup uses the canonical catalog name and does not invent or restart aliases.

To launch several named agents together, add `groups` to their catalog definitions and target the group with `@`:

```bash
hcom agent @wdt
hcom agent @wdt --dry-run
hcom agent @wdt --restart --clean
```

Flags are shared by every member, except `--as` and `--attach`, which are invalid for group launches. Members are processed in name order; already-running agents count as successful. A failure does not prevent subsequent members from launching, but the command returns exit code 1 after printing its summary. Group launches require separate terminal panes and reject `terminal: here`.

A targeted message starts a missing or stopped catalog agent before delivery:

```bash
hcom send @wdt_main --intent request -- "Review the current change"
```

This works for direct names, catalog tag groups, multiple recipients, and stopped catalog members in an existing thread. Unknown names fail without storing the message. Broadcasts address only currently deliverable instances and never start the whole catalog.

## Terminal placement

`terminal` selects the launch backend. `session` and `window` configure placement for tmux and Herdr.

hcom chooses the launch strategy in this order:

1. `terminal_command`: pass the raw command through `HCOM_TERMINAL`
2. explicit `terminal`: launch through that terminal preset
3. hcom's configured default terminal

For tmux, `session` selects or creates the tmux session and `window` selects the window. For Herdr, `session` selects or creates the space (called a workspace by the CLI), `window` selects or creates the tab, and each agent runs in a pane split inside that tab. When a project catalog supplies no Herdr session, hcom uses the name of the directory containing `.hcom`; unless `window` is configured explicitly, each agent gets a tab named after the agent. This placement also applies when Herdr comes from hcom's configured default, including nested launches and message autostart; the launching agent's workspace and tab are not inherited. If no Herdr server is running at launch time, hcom automatically starts `herdr server` headless in the background (enabled by default; toggle with `hcom config terminal.herdr_autostart true|false` or `HCOM_HERDR_AUTOSTART`). Other non-tmux terminal presets ignore `session` with a warning. Managed agents retain normal hcom lifecycle behavior, including closing their pane through `hcom kill`.

For Herdr-recognized tools, hcom identifies the outer PTY process to Herdr on Unix/macOS; on Windows, Herdr detects the tool in the descendant process tree. Herdr screen manifests and native integrations manage working, idle, blocked state and session-resume metadata. The hcom `pane.report_agent` fallback is reserved for tools Herdr does not recognize.

## Shell completions

Generate shell completions for agent names and groups:

```bash
# Bash
hcom agent completions bash > ~/.local/share/bash-completion/completions/hcom-agent

# Zsh
hcom agent completions zsh > ~/.zsh/completions/_hcom_agent

# Fish
hcom agent completions fish > ~/.config/fish/completions/hcom-agent.fish
```

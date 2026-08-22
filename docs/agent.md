# Named agents

`hcom agent` launches recurring agents from JSON catalogs and editable bundles. A catalog keeps
launch settings; `agents/<name>/AGENTS.md` keeps the agent's evolving instructions and can refer to
other files in the same bundle.

```bash
hcom agent wdt_main                 # launch, or report that it is already running
hcom agent @wdt                     # launch every member of the wdt catalog group
hcom agent wdt_main --as wdt_review # same config, independent instance named wdt_review
hcom agent wdt_main --cli codex     # override the configured CLI
hcom agent list                     # catalog entries, effective CLI/model, status, and source
hcom agent list @wdt                # show only members of the wdt catalog group
hcom agent list --all               # include agents hidden by recursive selective imports
hcom agent list --local             # only direct and imported agents from this project
hcom agent show wdt_main            # effective configuration and exact command
hcom agent attach wdt_main          # focus its managed terminal window
hcom agent edit                     # edit the global catalog
hcom agent edit --project           # edit the current project's catalog
```

Run `hcom agent --help` for all flags. `--dry-run` prints commands without launching them.

## Catalogs and precedence

hcom merges catalog layers in this order. Later values win:

1. built-in defaults
2. `defaults` in `~/.hcom/agents.json`
3. the named entry in `~/.hcom/agents.json`
4. catalogs listed by `HCOM_AGENT_CATALOGS`
5. `defaults`, named entries, and bundles in the nearest parent `.hcom`
6. command-line flags

Set `HCOM_AGENTS_FILE` to replace the global catalog path. `HCOM_AGENT_CATALOGS` is a
platform-specific path-separated list of additive catalogs; it does not replace the global one.

The project search walks parent directories until it finds the nearest `.hcom`, independently of
Git boundaries. The global `~/.hcom` is never a project scope. A project agent fully shadows a
same-named global/additive agent instead of inheriting its fields.

Scalar fields replace earlier values. `env`, `args`, and tool profiles merge. Relative `dir`
paths use `$HOME` in the global catalog, the directory containing `.hcom` in project
catalogs (including when imported), and the catalog directory in other imported/additive
catalogs. hcom expands `~` and environment variables.

Keep paths in project catalogs relative to the project root. Do not store machine-specific
absolute paths in a versioned `.hcom/agents.json`; use absolute or `~`-based paths only in
machine-local catalogs such as `~/.hcom/agents.json`.

Global catalog `defaults` apply throughout the non-project catalog group. Project defaults apply
only to project agents.

Unknown catalog fields are errors. This catches misspelled configuration instead of silently
ignoring it.

## Agent bundles

Global and project layouts are identical:

```text
~/.hcom/                         <project>/.hcom/
├── agents.json                 ├── agents.json
└── agents/                     └── agents/
    └── reviewer/                   └── reviewer/
        ├── AGENTS.md                   ├── AGENTS.md
        └── skills/                     └── skills/
            └── review/                     └── review/
                └── SKILL.md                    └── SKILL.md
```

`agents/<name>/AGENTS.md` defines an agent even without a matching JSON entry. A JSON-only agent
also remains valid. Discovered directory names must contain only lowercase letters, numbers, and
underscores.

The effective `agent_instructions` contains the fixed JSON `system_prompt`, then an `# Agent bundle
instructions` section with absolute bundle and `AGENTS.md` paths and its current contents, then an
`# Available agent skills` manifest. Empty sections are omitted. `prompt` remains the initial user
message. hcom rereads `AGENTS.md` and skills on every clean start and named-agent resume, so an
agent may improve its bundle for its next launch. Relative references resolve from the bundle or
skill directory named in the manifest.

If a bundle is outside the working directory, hcom grants only that directory through the CLI's
additional-workspace mechanism. Launch fails clearly when the CLI cannot make it writable at
startup.

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

The selected `tools.<cli>` profile replaces shared scalar values and appends its `args`.
Command-line flags override both. Top-level `model`, `reasoning`, and `args` remain useful for
agents that always use one CLI.

`reasoning` maps to `--effort` for Claude and Antigravity (`agy`), and to Codex's
`model_reasoning_effort`. Other CLIs reject the field at launch; use `tools.<cli>.args` when that
CLI has its own reasoning control. `--reasoning` overrides the catalog value.

`groups` is independent of `tag`: it does not change runtime display names, message routing, or
`hcom kill tag:...`. An agent may belong to multiple catalog groups. Group membership merges
additively across catalog layers and duplicate names are ignored. Group names use lowercase
letters, numbers, and underscores.

## Imports

A catalog may import every agent or a selected set from another catalog:

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

Omitting `agents` imports all entries; an empty list imports none. Relative `from` paths use the
importing catalog's directory. Imports are recursive, load before local entries, preserve the
source catalog's defaults and path base, and reject cycles or unknown selected agents.
Every imported or additive catalog discovers bundles in an `agents/` directory beside that
catalog file; bundle-only agents participate in selective imports normally.

For normal listing, direct launch, and message routing, an import's `agents` list remains a
visibility boundary. `hcom agent list --all` and `hcom agent @<group>` deliberately traverse every
recursively reachable import and consider all of its agents, including entries omitted by a
selective import. `--all` also works with the `--names` and `--json` listing formats. This lets a
global catalog act as a registry from which agents can be inspected or a project group can be
started without changing to that project's directory. hcom does not scan the filesystem for
unrelated project catalogs; they must be reachable through an import.

`hcom agent list --local` limits any listing format to the nearest project's direct and imported
agents, excluding global and additive catalogs. Combine it with `--all` to include project-imported
agents hidden by selective imports.

The table produced by `hcom agent list` shows each agent's effective CLI and model. An unset model is
shown as `-`; JSON output includes it as `model: null`. Per-CLI tool profiles are resolved before
the value is displayed.

## Private agent skills

Private skills live only under `<bundle>/skills/`. Each immediate child containing `SKILL.md` is
listed in the common lazy-loading manifest; hcom does not register it through a CLI-specific skill
system. Normal user, project, and plugin skills remain available.

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

YAML frontmatter supplies `name` and `description`. Missing or malformed values fall back to the
directory basename and first Markdown heading, then to `Agent-local skill; read SKILL.md for
details.` `hcom agent show` warns about malformed metadata, duplicate names, and skipped symlinks
that escape the bundle. `hcom agent list --json` exposes `{name, description, path}` entries.

`skills_dir` and `--skills-dir` have been removed. Move each old skill to
`agents/<name>/skills/<skill>/SKILL.md`; old configuration produces a targeted migration error.

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

Per-instance files are stored under `$HCOM_DIR/system-prompts/<tool>/<instance>/`. The fallback is
an approximation: it is subordinate to genuine system/developer messages, though agents are told
to treat it above ordinary task text. It is not emitted as a separate user task or meta-turn.

Use `hcom agent show <name> --cli <tool>` to inspect the exact effective command before launch.

## Starting, resuming, and messaging

The built-in start mode is clean. Set `"resume": true` in defaults or an agent entry to continue
its stopped session. `--resume` and `--clean` override the catalog; if both occur, the last one
wins. `--restart` first replaces a running instance, then applies the selected start mode.

An hcom-managed agent may invoke another AI CLI directly. The child inherits the
parent environment, but hcom rejects hooks whose actual CLI does not match the
tool bound to the inherited process identity. This keeps raw nested CLI sessions
from replacing the parent's session, transcript, or delivery binding.

An instance name is unique. Launching an already-running name reports its status and exits without
opening another instance. To run one catalog definition concurrently, assign each instance a
different name:

```bash
hcom agent wdt_main --as wdt_review
hcom agent wdt_main --as wdt_backend
hcom agent show wdt_main --as wdt_review # inspect the aliased command
```

`wdt_main` remains the catalog key used to resolve configuration. The `--as` value becomes the
runtime identity used by `hcom list`, messages, duplicate detection, `--restart`, `--attach`, and
resume history. It is also the default tmux window or Herdr tab name; an explicit catalog or CLI
`window` still wins. Address an aliased instance directly, for example
`hcom send @wdt_review -- "Review this"`. Catalog-driven message startup continues to use the
canonical catalog name and does not invent or restart aliases.

To launch several named agents together, add `groups` to their catalog definitions and target the
group with `@`:

```bash
hcom agent @wdt
hcom agent @wdt --dry-run
hcom agent @wdt --restart --clean
```

Flags are shared by every member, except `--as` and `--attach`, which are invalid for group
launches. Members are processed in name order; already-running agents count as successful. A
failure does not prevent later members from being attempted, but the command returns exit 1 after
printing its summary. Group launches require separate terminal panes and reject `terminal: here`.

A targeted message starts a missing or stopped catalog agent before delivery:

```bash
hcom send @wdt_main --intent request -- "Review the current change"
```

This works for direct names, catalog tag groups, multiple recipients, and stopped catalog members
in an existing thread. Unknown names fail without storing the message. Broadcasts address only
currently deliverable instances and never start the whole catalog.

## Terminal placement

`terminal` selects the launch backend. `session` and `window` configure placement for tmux and
Herdr.

hcom chooses the launch strategy in this order:

1. `terminal_command`: pass the raw command through `HCOM_TERMINAL`
2. explicit `terminal`: launch through that terminal preset
3. hcom's configured default terminal

For tmux, `session` selects or creates the tmux session and `window` selects the window. For Herdr,
`session` selects or creates the space (called a workspace by the CLI), `window` selects or creates
the tab, and each agent runs in a pane split inside that tab. When a project catalog supplies no
Herdr session, hcom uses the name of the directory containing `.hcom`; unless `window`
is configured explicitly, each agent gets a tab named after the agent. This placement also applies
when Herdr comes from hcom's configured default, including nested launches and message autostart;
the launching agent's workspace and tab are not inherited. Other non-tmux terminal
presets ignore `session` with a warning. Managed agents retain normal hcom lifecycle behavior,
including closing their pane through `hcom kill`.

For Herdr-known tools, hcom identifies the outer PTY process to Herdr on Unix/macOS; on Windows,
Herdr detects the tool in the descendant process tree. Herdr's screen manifests or native
integrations own working/idle/blocked state and session-resume metadata. The hcom
`pane.report_agent` fallback is reserved for tools Herdr does not recognize.

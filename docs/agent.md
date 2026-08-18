# Named agents

`hcom agent` launches recurring agents from JSON catalogs. A catalog keeps the CLI, working
directory, terminal placement, environment, tool-specific arguments, and private skills under one
stable name.

```bash
hcom agent wdt_main                 # launch, or report that it is already running
hcom agent wdt_main --as wdt_review # same config, independent instance named wdt_review
hcom agent wdt_main --cli codex     # override the configured CLI
hcom agent ls                       # catalog entries, live status, and source file
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
5. `defaults` and the named entry in the nearest `.hcom-agents.json`
6. command-line flags

Set `HCOM_AGENTS_FILE` to replace the global catalog path. `HCOM_AGENT_CATALOGS` is a
platform-specific path-separated list of additive catalogs; it does not replace the global one.

Scalar fields replace earlier values. `env`, `args`, and tool profiles merge. Relative `dir` and
`skills_dir` paths use `$HOME` as their base in the global catalog and the catalog's directory in
project, imported, and additive catalogs. hcom expands `~` and environment variables.

Global catalog `defaults` apply to every resolved catalog agent, even when the named agent is
defined only in an additive or project catalog. Defaults from later catalogs apply only when that
catalog defines the named agent.

Unknown catalog fields are errors. This catches misspelled configuration instead of silently
ignoring it.

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
      "skills_dir": ".hcom/agents/wdt_main/skills",
      "cli": "codex",
      "session": "wdt",
      "window": "review",
      "terminal": "wezterm-tab",
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
| `skills_dir` | Additional editable skills belonging only to this named agent |
| `cli` | CLI selected by default |
| `terminal` | hcom terminal preset, or `here` |
| `terminal_command` | Raw terminal command containing `{script}` |
| `session` | tmux session or Herdr workspace; ignored by other terminals |
| `window` | tmux window or Herdr tab; defaults to the instance name |
| `tag` | hcom group tag |
| `model` | Default model passed to the selected CLI |
| `prompt` | Initial user prompt |
| `system_prompt` | Additional system prompt |
| `pre` | Shell command run before the CLI |
| `resume` | Resume the previous session by default |
| `env` | Environment variables merged by key |
| `args` | Additional CLI arguments |
| `tools.<cli>` | Per-CLI `model`, `prompt`, `system_prompt`, and `args` overrides |

The selected `tools.<cli>` profile replaces shared scalar values and appends its `args`.
Command-line flags override both. Top-level `model` and `args` remain useful for agents that always
use one CLI.

## Imports

A catalog may import every agent or a selected set from another catalog:

```json
{
  "imports": [
    {
      "from": "~/work/wdt/ansible-wdt/.hcom-agents.json",
      "agents": ["wdt_main"]
    },
    { "from": "../shared/.hcom-agents.json" }
  ],
  "agents": {
    "local_override": { "dir": ".", "cli": "codex" }
  }
}
```

Omitting `agents` imports all entries; an empty list imports none. Relative `from` paths use the
importing catalog's directory. Imports are recursive, load before local entries, preserve the
source catalog's defaults and path base, and reject cycles or unknown selected agents.

## Private agent skills

`skills_dir` adds an editable skill collection for one named agent. hcom passes it only to the
selected CLI invocation. It does not change `HOME`, the CLI's own home directory, or the working
directory. Normal user, project, and plugin skills remain available.

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

Support depends on the selected CLI:

| CLI | Support | Invocation-local mechanism | Notes |
|---|---:|---|---|
| Claude | yes | `--plugin-dir <parent-of-skills_dir>` | `skills_dir` must be named `skills`. Claude loads it as the plugin's standard `skills/` directory; a manifest is optional. Skills are namespaced by the plugin directory name. |
| Codex | yes | `-c skills.config=[...]` | hcom passes each immediate child containing `SKILL.md`. Other children are ignored. |
| Kimi | yes | `--skills-dir <skills_dir>` | The collection stays in place. |
| Pi | yes | `--skill <skills_dir>` | The collection stays in place. |
| OpenCode | yes | `OPENCODE_CONFIG_CONTENT.skills` | hcom merges the path with existing inline configuration. |
| Kilo | yes | `KILO_CONFIG_CONTENT.skills.paths` | hcom merges the path with existing inline configuration. |
| Copilot | yes | `COPILOT_SKILLS_DIRS` | hcom appends the path to an existing value. |
| Antigravity | no | none | hcom rejects `skills_dir`. |
| Gemini | no | none | hcom rejects `skills_dir`. |
| OMP | no | none | hcom rejects `skills_dir`. |
| Cursor | no | none | hcom rejects `skills_dir`. |
| Hermes | no | none | hcom rejects `skills_dir`. |

The unsupported adapters fail explicitly. Filesystem permission flags such as `--add-dir` do not
count as skill discovery.

Use `hcom agent show <name> --cli <tool>` to inspect the exact effective command before launch.

## Starting, resuming, and messaging

The built-in start mode is clean. Set `"resume": true` in defaults or an agent entry to continue
its stopped session. `--resume` and `--clean` override the catalog; if both occur, the last one
wins. `--restart` first replaces a running instance, then applies the selected start mode.

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
Herdr session, hcom uses the name of the directory containing `.hcom-agents.json`; unless `window`
is configured explicitly, each agent gets a tab named after the agent. Other non-tmux terminal
presets ignore `session` with a warning. Managed agents retain normal hcom lifecycle behavior,
including closing their pane through `hcom kill`.

For Herdr-known tools, hcom identifies the outer PTY process to Herdr on Unix/macOS; on Windows,
Herdr detects the tool in the descendant process tree. Herdr's screen manifests or native
integrations own working/idle/blocked state and session-resume metadata. The hcom
`pane.report_agent` fallback is reserved for tools Herdr does not recognize.

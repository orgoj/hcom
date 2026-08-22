# Named agents (`hcom agent`)

Use `hcom agent` for recurring agents whose working directory, CLI, prompts, environment, and
terminal setup should live in versionable configuration instead of shell history. An editable
`agents/<name>/AGENTS.md` bundle supplies evolving instructions and defines the agent even without
a JSON entry. An instance name
is unique: launching a name that is already running reports its status and exits without creating
a duplicate. Use `--as` to launch one catalog definition concurrently under distinct instance
names.

`hcom agent list` answers "who can be addressed at all"; `hcom list` answers "who is running right
now". A catalog agent absent from `hcom list` is stopped, not unknown, and an exact name is never
resolved to a similar one.

## Catalogs and precedence

hcom reads these catalogs:

1. `~/.hcom/agents.json` for machine-wide agents. Override its path with `HCOM_AGENTS_FILE`.
2. Catalogs listed in `HCOM_AGENT_CATALOGS`, separated with the platform path separator, for
   additive client-specific views.
3. The nearest parent `.hcom/agents.json` and `.hcom/agents/<name>/` bundles, found independently
   of Git roots, for project agents that fully shadow same-named global agents.
4. Command-line flags, which have the highest precedence.

Global defaults apply to the non-project catalog group. Project agents resolve independently and
do not inherit global fields. Within each group, scalar fields replace earlier values while `env`,
`args`, `groups`, and matching `tools` profiles merge.

Relative `dir` values resolve from `$HOME` globally, from the directory containing
`.hcom` for project catalogs (including when imported), and from the catalog directory for other
additive/imported catalogs.

Keep paths in project catalogs relative to the project root. Do not store machine-specific
absolute paths in a versioned `.hcom/agents.json`; use absolute or `~`-based paths only in
machine-local catalogs such as `~/.hcom/agents.json`.

The fixed JSON `system_prompt` is followed by the current `AGENTS.md` and a manifest of immediate
`skills/*/SKILL.md` children. hcom rereads both on every clean launch/named resume, and referenced
files resolve relative to their bundle or skill directory. For an external bundle,
hcom grants only that directory when the CLI supports startup-time additional workspaces;
otherwise launch fails clearly.

## Imports and additive client catalogs

Every catalog may import all or selected agents from other catalogs. Definitions stay in their
owning project while an external client can compose a private address book:

```json
{
  "imports": [
    {
      "from": "~/work/wdt/ansible-wdt/.hcom/agents.json",
      "agents": ["wdt_main"]
    }
  ]
}
```

Point only that client's process at the overlay while retaining normal global agents:

```bash
export HCOM_AGENT_CATALOGS="$HOME/.hermes/profiles/main/hcom-agents.json"
hcom send @wdt_main --intent request -- "Review the deployment"
```

Multiple additive catalogs use the operating system's path separator (`:` on Unix, `;` on
Windows). Their precedence is global catalog, additive catalogs from left to right, project
catalog, then command-line flags. `HCOM_AGENTS_FILE` keeps its separate replacement semantics for
the global catalog.

Imports are recursive and load before the importing file's local definitions. Relative `from`
paths resolve against the importing file. Omitting `agents` imports all agents; an empty list
imports none. Imported agents retain their source catalog's defaults and relative directory base.
Each catalog discovers bundles in a sibling `agents/` directory, including bundle-only agents.
Missing files, requested names, and import cycles are errors.

Selective imports remain the visibility boundary for normal listing, direct launch, and message
routing. `hcom agent list --all` and `hcom agent @<group>` are explicit exceptions: they consider all
agents in every recursively reachable import, including agents omitted by an import's `agents`
list. The listing flag also works with `--names` and `--json`. This supports inspecting agents or
starting a project group from another directory while keeping its other agents out of the normal
external address book. hcom does not discover unrelated catalogs by scanning the filesystem.
`hcom agent list --local` limits listings to the nearest project's direct and recursively imported
agents; global and additive catalogs are excluded. Combine it with `--all` to include agents hidden
by the project's selective imports.

## Basic catalog

```json
{
  "defaults": {
    "cli": "claude",
    "terminal": "tmux-window",
    "resume": false
  },
  "agents": {
    "reviewer": {
      "description": "code review for ~/projects/app: correctness, tests, security",
      "dir": "~/projects/app",
      "tag": "review",
      "groups": ["reviewers", "app"],
      "prompt": "Review the current changes and report actionable findings.",
      "system_prompt": "You are a concise, rigorous code reviewer.",
      "env": {
        "RUST_BACKTRACE": "1"
      }
    }
  }
}
```

```bash
hcom agent reviewer
hcom agent @app
hcom agent reviewer --as review_api
hcom agent show reviewer       # effective configuration and exact launch command
hcom agent reviewer --dry-run  # render without launching
hcom agent list                # name and description only, unless output is a terminal
hcom agent list --for-agents   # same listing, forced
hcom agent list --for-humans   # full table: CLI, model, placement, directory, status
hcom agent list @app           # show only members of one catalog group
hcom agent list --all          # include agents hidden by recursive selective imports
hcom agent list --local        # only direct and imported agents from this project
hcom agent attach reviewer
hcom agent reviewer --restart
hcom agent reviewer --resume
hcom agent reviewer --clean
```

Use `hcom agent edit` for the global catalog or `hcom agent edit --project` for the project catalog.

The listing shows the effective CLI and its resolved model. An unset table value is `-`; JSON uses
`model: null`. Models from the selected CLI's tool profile are resolved before display.

`groups` is a list of catalog-only launch sets; it does not affect runtime display names, message
routing, or `hcom kill tag:...`. `hcom agent @<group>` processes members in name order, applies its
flags to every member, continues after individual failures, and returns exit 1 if any member
failed. `--as`, `--attach`, and `terminal: here` are invalid for group launches.

The positional name selects the catalog definition; `--as <name>` selects the runtime identity.
The alias is used for duplicate detection, routing, restart/attach behavior, resume history, and the
default tmux window or Herdr tab. An explicit `window` remains unchanged. Address the instance by
its alias, such as `hcom send @review_api -- "Review this"`. Targeted sends to catalog names only
auto-start the canonical catalog instance; aliases must be launched explicitly.

## Message-driven startup

A targeted message treats the effective catalog as an address book as well as a launcher. If a
resolved local recipient is missing or stopped but is defined in the catalog, hcom starts it before
writing the message:

```bash
hcom send @reviewer --intent request -- "Review the current diff"
hcom send @review- --intent inform -- "The API contract changed"
hcom send @reviewer @one_shot -- "Coordinate this task"
```

The launch uses the same merged agent definition as `hcom agent <name>`, including its configured
clean/resume start mode. The message event is addressed to the canonical catalog name, so delivery
continues through the normal unread/wake mechanism as the new process registers. A send fails
without writing its message if a requested name is neither deliverable nor defined in the catalog,
or if its catalog launch fails.

This behavior applies to direct names, catalog tag groups, and missing catalog members already
recorded in a `--thread`. It does not apply to broadcasts: a message without targets reaches current
deliverable instances only and never starts the whole catalog. Remote `name:DEVICE` targets are
resolved by relay state and do not launch a same-named local catalog entry.

## Clean start and resume

Operational rule: create and update catalog agents as clean-starting. “Persistent,” “recurring,”
or “catalog agent” refers to the persistent catalog definition, not session continuation. Add
`resume: true`, pass `--resume`, or run `hcom r` only when the user explicitly requests resuming a
previous tool session. Never infer resume from persistence, an existing stopped session, or another
agent's configuration.

`resume` is a scalar boolean accepted in `defaults` and individual agent entries:

- `false` starts a new tool session and is the built-in default.
- `true` runs `hcom r <name> --go` to continue the agent's previous stopped session.
- Omitting the field inherits the value from the preceding catalog layer.

Normal catalog precedence applies within a scope, so an agent entry can override its catalog's
default; a project agent shadows the global agent as a whole. Command-line `--resume` and `--clean` have the highest
precedence. If both occur, the last flag wins, which makes wrapper scripts able to append an
authoritative choice.

```json
{
  "defaults": { "resume": true },
  "agents": {
    "reviewer": {},
    "one_shot": { "resume": false }
  }
}
```

```bash
hcom agent reviewer              # resumes by catalog default
hcom agent reviewer --clean      # one clean launch
hcom agent one_shot --resume     # one resumed launch
hcom agent reviewer --restart --clean
hcom agent show reviewer --clean # shows start: clean and the clean command
```

Resume mode requires an existing stopped session for that name. The selected start mode also
applies when `--restart` first stops a running agent. Use `hcom agent show <name>` or `--dry-run` to
inspect the effective mode and command without launching it.

## Per-CLI tool profiles

An agent may preserve one identity and shared setup while being launched through different AI CLIs.
Keep shared fields at the agent level and put CLI-specific `model`, `prompt`, `system_prompt`, and
`args` in `tools.<cli>`:

```json
{
  "agents": {
    "reviewer": {
      "cli": "claude",
      "dir": "~/projects/app",
      "prompt": "Review the current changes.",
      "args": ["--common-tool-flag"],
      "tools": {
        "claude": {
          "model": "sonnet",
          "system_prompt": "Use the configured Claude reviewer agent.",
          "args": ["--agent", "security-reviewer"]
        },
        "codex": {
          "model": "gpt-5.4",
          "reasoning": "high",
          "system_prompt": "Prioritize correctness and cite file locations.",
          "args": ["--sandbox", "workspace-write"]
        }
      }
    }
  }
}
```

The effective CLI is the command-line `--cli` value, otherwise the agent's `cli`, otherwise
`claude`. hcom then applies configuration in this order:

1. Shared agent fields and top-level `args`.
2. The matching `tools[effective_cli]` profile.
3. Command-line flags and passthrough arguments.

Tool-profile scalar values override their shared equivalents. Tool-profile arguments append after
top-level arguments. Command-line `--model`, `--reasoning`, `--prompt`, and `--system-prompt`
override both; unknown command-line arguments append last and are forwarded to the selected tool.

```bash
hcom agent reviewer --cli claude
hcom agent reviewer --cli codex
hcom agent show reviewer --cli codex
hcom agent reviewer --cli codex --model gpt-5.4 --reasoning high --dry-run
```

Top-level `model`, `reasoning`, and `args` remain valid for simple single-CLI agents and existing
catalogs. `reasoning` maps to `--effort` for Claude and Antigravity (`agy`), and to
`model_reasoning_effort` for Codex. Other CLIs reject it at launch; use a tool-specific `args`
entry instead. Avoid putting a Claude-only flag in top-level `args` when the agent can be switched
to Codex or another CLI.

## Terminal selection

`terminal` selects how the agent is launched. `session` and `window` are supplementary placement
settings; they do not select a multiplexer by themselves. For tmux, they select the session and
window. For Herdr, they select the workspace and tab, including when Herdr is hcom's configured
default. Nested launches and targeted-message autostart use the catalog agent's placement rather
than the launching agent's inherited workspace and tab. Other non-multiplexer terminals ignore
`session` with a warning.

For tools Herdr recognizes, hcom lets Herdr's native process detection, screen manifests, and
installed integrations own classification, lifecycle state, and resume metadata. hcom uses
`pane.report_agent` only as a fallback for tools outside Herdr's agent catalog.

`pre` runs inside the prepared window before hcom, and `env` variables are supplied to the launch.

## Schema summary

Agent and `defaults` fields:

- `description` — one line on what the agent is for; the only field other agents see in
  `hcom agent list --for-agents`
- `cli`, `dir`, `terminal`, `terminal_command`, `session`, `window`, `tag`, `groups`, `model`, `reasoning`
- `prompt`, `system_prompt`, `pre`, and boolean `resume`
- `env` object and `args` array
- `tools` object keyed by CLI name

Each `tools.<cli>` profile accepts only `model`, `reasoning`, `prompt`, `system_prompt`, and `args`.
Unknown fields are rejected so catalog typos fail visibly.

Any unrecognized flag after `hcom agent <name>` is forwarded to `hcom <effective-cli>`. Use `--` to
forward everything that follows verbatim, including tokens that resemble `hcom agent` flags.

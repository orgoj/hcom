# Named agents (`hcom agent`)

Use `hcom agent` for recurring agents whose working directory, CLI, prompts, environment, and
terminal setup should live in versionable configuration instead of shell history. An editable
`agents/<name>/SOUL.md` bundle supplies evolving instructions and defines the agent even without
a JSON entry. An instance name
is unique: launching a name that is already running reports its status and exits without creating
a duplicate. Use `--as` to launch one catalog definition concurrently under distinct instance
names. Bundle `AGENTS.md` files are not read as a fallback.

`hcom agent list` answers "who can be addressed at all"; `hcom list` answers "who is running right
now". A catalog agent absent from `hcom list` is stopped, not unknown, and an exact name is never
resolved to a similar one. "Who can be addressed at all" is also directory-dependent: see
[Imports and additive client catalogs](#imports-and-additive-client-catalogs).

## Catalogs and precedence

hcom resolves effective values through this chain, from weakest to strongest, regardless of the
directory where the command runs:

1. built-in defaults (`cli` = `claude`)
2. `defaults` in `~/.hcom/agents.json` (replace its path with `HCOM_AGENTS_FILE`)
3. the matching catalog's `defaults`
4. the named agent entry
5. global `cli_overrides` maps the catalog CLI once (explicit `--cli` bypasses it)
6. the matching `tools.<effective-cli>` profile (exact key, else an alias: `agy` = `antigravity`)
7. command-line flags

Steps 3-4 repeat for every matching imported, additive, or project catalog in catalog order.
A catalog's `defaults` apply to every agent it defines and to every agent it brings in — by import,
or as the enclosing project of a nested `.hcom`. Imports are recursive and apply before the
importing file's local entries. Catalogs in `HCOM_AGENT_CATALOGS` apply left to right.

Every parent `.hcom` up from the launch directory is found independently of Git roots; all of them
apply, the outermost weakest and the nearest strongest, so an agent in a nested repository also
addresses the enclosing project's agents while its own stay private to it. A project agent ignores a
same-named global/additive entry, but global `defaults` remain its lowest catalog layer.
Consequently, the same project agent inherits the same global defaults whether hcom runs inside
the project or from an external catalog that imports it.

`env`, `inherit_env`, `args`, `groups`, and tool-profile definitions merge; later scalar values replace earlier
ones. In particular, a later `system_prompt` replaces rather than appends to the earlier text, and
an explicit empty string clears it. Setting `inherit_env: ["VAR1", "VAR2"]` in catalog defaults or an
agent entry copies those environment variables from the caller's environment if set, without hardcoding
machine-specific values in shared catalogs.

For a long shared prompt, set top-level `"system_prompt_file": "SYSTEM_PROMPT.md"`. Relative paths
resolve from the catalog's directory. The UTF-8 contents supply that catalog's default
`system_prompt`; inline `defaults.system_prompt` overrides the file, including `""` to clear it.
Missing, unreadable, or non-UTF-8 referenced files are load errors. Omitting the key preserves
inline-only behavior.

Relative `dir` values resolve from `$HOME` globally, from the directory containing
`.hcom` for project catalogs (including when imported), and from the catalog directory for other
additive/imported catalogs.

Keep paths in project catalogs relative to the project root. Do not store machine-specific
absolute paths in a versioned `.hcom/agents.json`; use absolute or `~`-based paths only in
machine-local catalogs such as `~/.hcom/agents.json`.

For temporary changes without editing a tracked catalog, use a sidecar `<stem>.local.json` beside
it (`agents.json` → `agents.local.json`, usually gitignored). It merges into that catalog as its
strongest part, may add or disable agents, keeps the catalog's scope, and may not use `imports`;
`cli_overrides` work only beside the global catalog. `hcom agent show <name>` lists it under
`sources`.

## Bundles and file-based memory

The catalog `system_prompt` is followed by the current `SOUL.md` and, for CLIs without native
bundle skill loading, a manifest of immediate `skills/*/SKILL.md` children. On Claude launches,
hcom creates `<bundle>/.claude/skills -> ../skills` when absent and passes `--add-dir <bundle>`;
Claude discovers those skills natively, so the prompt omits the manifest. hcom rereads both on
every clean launch and named resume. References in the manifest resolve relative to their bundle
or skill directory. For an external bundle,
hcom grants only that directory when the CLI supports startup-time additional workspaces;
otherwise launch fails clearly. Antigravity (`agy`) uses its repeatable `--add-dir` option.

A bundle may keep file-based memory next to `SOUL.md`:

```text
.hcom/agents/project_main/
├── SOUL.md                 # stable role, boundaries, and critical rules (injected by hcom)
├── NOTES.md                # current local work and decisions
└── MEMORY/
    └── release-checks.md   # longer lessons recalled for related tasks
```

`NOTES.md` and `MEMORY/` are ordinary files, not hcom features: `SOUL.md` or the catalog
`system_prompt` must tell the agent when to read and update them. A running session keeps the
prompt it started with; edits to `SOUL.md` take effect on the next clean start or named resume.
The generated `# Agent bundle instructions` section names the absolute bundle and `SOUL.md`
paths, so instructions can refer to bundle files relative to it.

Every launched agent receives `HCOM_INSTANCE_NAME`, its actual instance name, and
`HCOM_AGENT_NAME`, the catalog name it was started from. They differ under `--as` and for roaming
agents (`reviewer` runs as `reviewer_<project>_<hash>`); outside the catalog both carry the instance
name, so one rule keyed on `HCOM_AGENT_NAME` covers every agent. `hcom r` keeps both, a fork keeps
the parent's `HCOM_AGENT_NAME` under its new instance name, and nested child launches set their own.

Catalog-launched Antigravity receives `DIPPY_POLICY_CWD` with the canonical catalog agent
directory. Its hooks inherit this value regardless of `--dir`, tool-call `Cwd`, or additional bundle
workspaces. A named resume recomputes it; nested child launches do not inherit it.
Direct tracked `hcom r <name>` restores the stored scope; older snapshots without one do not.

## Local CLI replacement

When a CLI reaches its usage limit, add a top-level mapping to the global catalog
`~/.hcom/agents.json` (or the file selected by `HCOM_AGENTS_FILE`):

```json
"cli_overrides": {
  "codex": "claude"
}
```

This maps the resolved catalog CLI once, before selecting the target `tools.<cli>` profile.
It applies to project and global agents, group launches, and message autostart without changing
project catalogs. `agent show`, dry-run, and listings show the effective CLI and model.
Explicit `--cli` (or `--tool`) bypasses the mapping. Mappings do not chain: with both
`codex -> claude` and `claude -> gemini`, a Codex agent uses Claude. Tool names and aliases
are validated; `adhoc` is not a launchable target or source. Other catalogs cannot define a
nonempty `cli_overrides` map. Remove the mapping to restore the catalog's choices.

Shared `model`, `reasoning`, and `args` still apply. Keep CLI-specific values in `tools.codex`,
`tools.claude`, etc.; the source CLI's profile is not applied after replacement. Without a target
profile or shared settings, the target CLI uses its own defaults.

Running agents keep their current CLI. To replace one and carry over context, use:

```bash
hcom agent reviewer --restart --continue
```

The mapping changes catalog launches only. Direct `hcom r` and `hcom f` continue using the saved
tool; native sessions cannot be resumed in another CLI. Use `--continue` for a handoff summary
when changing CLI, including for an agent configured with `resume: true`.

## Imports and additive client catalogs

Every catalog may import all or selected agents from other catalogs. Definitions stay in their
owning project while an external client can compose a private address book:

```json
{
  "imports": [
    {
      "from": "~/work/ops/.hcom/agents.json",
      "agents": ["ops_main"]
    }
  ]
}
```

Point only that client's process at the overlay while retaining normal global agents:

```bash
export HCOM_AGENT_CATALOGS="$HOME/.hermes/profiles/main/hcom-agents.json"
hcom send @ops_main --intent request -- "Review the deployment"
```

Multiple additive catalogs are separated by `:`. Their precedence is global catalog, additive catalogs from left to right, project
catalog, then command-line flags. `HCOM_AGENTS_FILE` keeps its separate replacement semantics for
the global catalog.

Imports are recursive and load before the importing file's local definitions. Relative `from`
paths resolve against the importing file. Omitting `agents` imports all agents; an empty list
imports none. Imported agents retain their source catalog's defaults and relative directory base.

An import's `agents` list is a scope boundary, for `hcom send` as much as for `hcom agent`, and the
isolation is deliberate. An agent defined only in a project catalog is addressable from inside that
project, and from another directory only if a catalog in scope imports it: in the example above
`ops_main` is reachable from anywhere while the project's other agents stay reachable only from
inside `~/work/ops`. hcom never scans the filesystem for unrelated project catalogs. A name out of
scope is reported as unknown plus a note that scope depends on the directory; hcom does not tell a
caller out of scope which catalog defines that name, so do not read "unknown" from outside a
project as the entry having been removed, and do not build tooling that works around the boundary.
Each catalog discovers bundles in a sibling `agents/` directory, including bundle-only agents.
Missing files, requested names, and import cycles are errors.

Selective imports remain the visibility boundary for normal listing, direct launch, and message
routing. `hcom agent list --all` and `hcom agent @<group>` are explicit exceptions: they consider all
agents in every recursively reachable import, including agents omitted by an import's `agents`
list. The listing flag also works with `--names` and `--json`. This supports inspecting agents or
starting a project group from another directory while keeping its other agents out of the normal
external address book. hcom does not discover unrelated catalogs by scanning the filesystem.
`hcom agent list --local` limits listings to the direct and recursively imported agents of the
enclosing project catalogs; global and additive catalogs are excluded. Combine it with `--all` to
include agents hidden by the project's selective imports.

## Basic catalog

```json
{
  "defaults": {
    "cli": "claude",
    "terminal": "herdr",
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

`groups` is a list of catalog groups, independent of `tag`; it does not affect runtime display
names or `hcom kill tag:...`. `hcom agent @<group>` launches every agent in a catalog group,
`hcom send @<group> -- <text>` reaches the members in scope and starts stopped ones (a running or
catalog agent with that name wins), and `hcom kill @<group>` terminates all active agents in that
group. `hcom agent @<group>` processes members in name order, applies its
flags to every member, continues after individual failures, and returns exit 1 if any member
failed. `--as`, `--attach`, and `terminal: here` are invalid for group launches.

The positional name selects the catalog definition; `--as <name>` selects the runtime identity.
The alias is used for duplicate detection, routing, restart/attach behavior, resume history, and the
default Herdr tab. An explicit `window` remains unchanged. Address the instance by
its alias, such as `hcom send @review_api -- "Review this"`.

To autostart a catalog agent under an adhoc clone/alias name directly via send, use `--spawn-as <name>` (alias `--as <name>`):

```bash
hcom send @ops_main --spawn-as review_api -- "Review this"
# or with the alias:
hcom send @ops_main --as review_api -- "Review this"
```

If `review_api` is already running, send delivers directly without relaunching. If stopped or not yet running, it autostarts the catalog agent under that instance name. Targeted sends without `--spawn-as` only auto-start the canonical catalog instance.

## Roaming archetypes

`"roaming": true` defines a role that materializes once per project. Do not set `dir`, `session`,
or `window` on that definition. A targeted send to `@reviewer` uses the sender instance's recorded
directory (or the shell CWD for an external sender), selects the nearest Git root, and routes to
`<name>_<project>_<hash>`, such as `reviewer_weather_app_412`: `<project>` is the root's normalized
basename and `<hash>` a 3-digit number (100-999) from the SHA-256 of the root path, so roots with
the same basename get different instances. Without a Git root it falls back to the nearest
`.hcom/agents.json` root, then the starting directory. `hcom send --project-dir <DIR> @reviewer`
uses `<DIR>`'s project root instead (roaming targets only).

The send and `hcom agent reviewer` print a `Note:` line on stdout naming the instance; address it
by that name afterwards. Instances of the role in other projects are separate agents. `hcom list`
marks each instance `↻reviewer`; `hcom agent list --json` lists a roaming entry's running
`instances`.

The materialized name, not the archetype alias, is stored in mentions, thread membership, request
watches, and delivery metadata. The original message text remains unchanged. Direct
`hcom agent/show/attach reviewer` commands resolve the current project identically. Broadcasts do
not materialize roaming agents. If an instance with the materialized name already belongs to
another project root, the send fails safely: only sends addressing that roaming agent by name,
full name, or tag fail. Running agents route first; the catalog is read
only for unmatched names, tags, stopped thread members, or `--spawn-as`, and an unreadable catalog
only warns.

Catalog `env` reaches the materialized process during nested autostart. A read-only roaming role
can therefore select a separately managed Dippy policy with one variable:

```json
{
  "agents": {
    "reviewer": {
      "roaming": true,
      "env": { "DIPPY_CONFIG_ONLY": "~/.hcom/dippy/roaming-reviewer.dippy" }
    }
  }
}
```

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

A slow-starting TUI is not a launch failure. When the readiness wait ends before the agent reports
ready, `send` notes that the agent is still starting, writes the message anyway, and exits 0; the
agent receives it when its delivery loop starts. The readiness wait itself is per CLI, since TUI
startup times differ by an order of magnitude: `hcom <tool> --help` prints the tool's value.

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

## Self handoff

A catalog agent can replace itself with a clean session that starts from a prompt it writes:

```bash
hcom handoff "Continue with step 3 of PLAN.md; steps 1-2 are committed."
hcom handoff --cli claude < handoff.md   # prompt from stdin, restart on another CLI
hcom handoff --dry-run "..."             # print the relaunch, stop nothing
```

hcom checks the relaunch first, then runs `hcom agent <HCOM_AGENT_NAME> --as <HCOM_INSTANCE_NAME>
--restart --clean --hcom-prompt <prompt>` from the agent's recorded directory, detached from the
agent it kills. The catalog name is the one recorded at launch, or the agent's own `HCOM_AGENT_NAME`
when none is recorded. The successor is the same catalog agent under the same instance name, so `--as`
aliases and roaming instances keep their names. The prompt is its initial prompt. Run it as the last
action of a turn; its output is logged to `.tmp/logs/handoff-<name>.log` in the hcom data directory
(`~/.hcom/.tmp/logs/` by default). Each validated handoff logs a life event with action `handoff`
(`hcom events --action handoff`).
Only agents started by `hcom agent` or send autostart (or resumed or forked from one) can hand off;
others get an error and keep running. `--restart` stops the old agent through its tracked process,
so an agent without one keeps running and the log shows the failed relaunch. With `auto_approve`,
`hcom handoff` is approved like the other safe hcom commands.

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
          "system_prompt": "Prioritize correctness and cite file locations."
        }
      }
    }
  }
}
```

The effective CLI is the command-line `--cli` value, otherwise the global `cli_overrides`
replacement of the agent's `cli` (default `claude`). hcom then applies configuration in this order:

1. Shared agent fields and top-level `args`.
2. The matching `tools[effective_cli]` profile.
3. Command-line flags and passthrough arguments.

Tool-profile scalar values override their shared equivalents. Tool-profile arguments append after
top-level arguments. Command-line `--model`, `--reasoning`, `--hcom-prompt`, and
`--hcom-system-prompt` override both; unknown command-line arguments append last and are forwarded to the selected tool.

```bash
hcom agent reviewer --cli claude
hcom agent reviewer --cli codex
hcom agent show reviewer --cli codex
hcom agent reviewer --cli codex --model gpt-5.4 --reasoning high --dry-run
hcom agent reviewer --cli claude --continue # fresh session with handoff summary from previous session
hcom agent reviewer --continue --last 3     # include last 3 exchanges in handoff prompt
```

Top-level `model`, `reasoning`, and `args` remain valid for simple single-CLI agents and existing
catalogs. `reasoning` maps to `--effort` for Claude and Antigravity (`agy`), and to
`model_reasoning_effort` for Codex. Other CLIs reject it at launch; use a tool-specific `args`
entry instead. Avoid putting a Claude-only flag in top-level `args` when the agent can be switched
to Codex or another CLI.

## Terminal selection

`terminal` selects how the agent is launched. `session` and `window` are supplementary Herdr
placement settings; they do not select Herdr by themselves. They select the workspace and tab,
including when Herdr is hcom's configured default. Without `session`, the workspace is named after
the agent's own `dir` (a roaming agent: its project root; no `dir`: the caller's project); the tab
defaults to the instance name. `hcom r` returns an agent to its recorded workspace and tab, and
`hcom f` opens a tab named after the fork. Nested launches and targeted-message autostart
use the catalog agent's placement rather than the launching agent's inherited workspace and tab.
Other terminals ignore `session` with a warning. A catalog agent with a terminal preset always
opens its own pane or window, even from a human shell; `terminal: here` runs it in place.

`terminal: orca` launches an unfocused tab in the local Orca runtime, bound to
the registered folder workspace matching the agent's canonical directory.
Orca is only the terminal UI; hcom retains messaging, identity, hooks, and
lifecycle authority. Remote Orca selectors are not supported, and the local
runtime must advertise `terminal.create-interactive-agent.v1` and
`terminal.create-folder-workspace.v1`. Orca automatically registers an
unrecognized local agent directory as a folder workspace without requiring
Git or creating a worktree.

For tools Herdr recognizes, hcom lets Herdr's native process detection, screen manifests, and
installed integrations own classification, lifecycle state, and resume metadata. hcom uses
`pane.report_agent` only as a fallback for tools outside Herdr's agent catalog.

`env` variables are supplied to the launch.

## Schema summary

Top-level `system_prompt_file` optionally references a UTF-8 file relative to the catalog.

Agent and `defaults` fields:

- `description`: one line on what the agent is for; the only field other agents see in
  `hcom agent list --for-agents`
- `cli`, `dir`, boolean `roaming`, `terminal`, `terminal_command`, `session`, `window`, `tag`, `groups`, `model`, `reasoning`
- `prompt`, `system_prompt`, boolean `resume`, and boolean `continue`
- `env` object, `inherit_env` and `args` arrays
- `idle_kill` (seconds; `0` or `false` disables) and boolean `disabled`
- `tools` object keyed by CLI name

Top level: `version`, `defaults`, `agents`, `imports`, `system_prompt_file`, and `cli_overrides`
(global catalog only).

`"disabled": true` makes hcom treat the agent as undefined (no launch, autostart, group
membership, or listing; `hcom agent list --all` shows it marked `disabled`); a later layer's
`false` re-enables it. A running instance stays messageable and killable.

`[[status_column]]` tables in `~/.hcom/config.toml` (`name`, `regex`, `reset = "keep" | "turn"`)
take values from each agent's last reply when it goes idle; `hcom agent list --status` shows them.
See `docs/agent.md` in the repository for details.

Each `tools.<cli>` profile accepts only `model`, `reasoning`, `prompt`, `system_prompt`, and `args`.
Unknown fields are rejected so catalog typos fail visibly.

Any unrecognized flag after `hcom agent <name>` is forwarded to `hcom <effective-cli>`. Use `--` to
forward everything that follows verbatim, including tokens that resemble `hcom agent` flags.

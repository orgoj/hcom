# Named agents (`hcom agent`)

Use `hcom agent` for recurring agents whose working directory, CLI, prompts, environment, and
terminal setup should live in versionable configuration instead of shell history. A named agent is
unique: launching a name that is already running reports its status and exits without creating a
duplicate.

## Catalogs and precedence

hcom reads these catalogs:

1. `~/.hcom/agents.json` for machine-wide agents. Override its path with `HCOM_AGENTS_FILE`.
2. The nearest `.hcom-agents.json` found from the working directory upward to the Git root, for
   project-specific additions and overrides.
3. Command-line flags, which have the highest precedence.

Within each catalog, `defaults` apply only to agents defined by that same file. An empty project
entry such as `"reviewer": {}` opts a global agent into the project defaults. Scalar fields replace
earlier values; `env`, `args`, and matching `tools` profiles merge. Arguments append in layer order.

Relative `dir` values resolve from `$HOME` in the global catalog and from the catalog directory in a
project catalog. `~` and environment variables are expanded.

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
      "dir": "~/projects/app",
      "tag": "review",
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
hcom agent show reviewer       # effective configuration and exact launch command
hcom agent reviewer --dry-run  # render without launching
hcom agent ls
hcom agent attach reviewer
hcom agent reviewer --restart
hcom agent reviewer --resume
hcom agent reviewer --clean
```

Use `hcom agent edit` for the global catalog or `hcom agent edit --project` for the project catalog.

## Clean start and resume

`resume` is a scalar boolean accepted in `defaults` and individual agent entries:

- `false` starts a new tool session and is the built-in default.
- `true` runs `hcom r <name> --go` to continue the agent's previous stopped session.
- Omitting the field inherits the value from the preceding catalog layer.

Normal catalog precedence applies, so an agent entry can override its catalog's default and a
project entry can override the global value. Command-line `--resume` and `--clean` have the highest
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
top-level arguments. Command-line `--model`, `--prompt`, and `--system-prompt` override both; unknown
command-line arguments append last and are forwarded to the selected tool.

```bash
hcom agent reviewer --cli claude
hcom agent reviewer --cli codex
hcom agent show reviewer --cli codex
hcom agent reviewer --cli codex --model gpt-5.4 --dry-run
```

Top-level `model` and `args` remain valid for simple single-CLI agents and existing catalogs. Avoid
putting a Claude-only flag in top-level `args` when the agent can be switched to Codex or another
CLI.

## Terminal selection

Terminal behavior is selected in this order:

1. `terminal_command`: raw command containing `{script}`, passed as `HCOM_TERMINAL`.
2. `session`: create or reuse a tmux session and launch in its `window` (defaults to agent name).
3. `terminal`: pass the named terminal preset to hcom.
4. hcom's configured default terminal.

`pre` runs inside the prepared window before hcom, and `env` variables are supplied to the launch.
Without tmux, `session` is ignored with a warning and terminal selection falls back normally.

## Schema summary

Agent and `defaults` fields:

- `cli`, `dir`, `terminal`, `terminal_command`, `session`, `window`, `tag`, `model`
- `prompt`, `system_prompt`, `pre`, and boolean `resume`
- `env` object and `args` array
- `tools` object keyed by CLI name

Each `tools.<cli>` profile accepts only `model`, `prompt`, `system_prompt`, and `args`. Unknown fields
are rejected so catalog typos fail visibly.

Any unrecognized flag after `hcom agent <name>` is forwarded to `hcom <effective-cli>`. Use `--` to
forward everything that follows verbatim, including tokens that resemble `hcom agent` flags.

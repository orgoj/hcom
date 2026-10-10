<div align="center">

# `hcom`

*Hook your coding agents together*

[![CI](https://github.com/orgoj/hcom/actions/workflows/ci.yml/badge.svg)](https://github.com/orgoj/hcom/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/orgoj/hcom)](https://github.com/orgoj/hcom/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/orgoj/hcom/blob/orgoj/LICENSE)

</div>

**CLI tool that agents use to message, watch, and spawn each other across terminals.**

This repository is a fork of [`aannoo/hcom`](https://github.com/aannoo/hcom). Its primary addition is the [`agent` subcommand](docs/agent.md), providing persistent named agent catalogs, directory bundles, private agent skills, and multi-agent group launches.

Start an agent with `hcom` in front, then prompt normally.

Use it to coordinate multi-agent pipelines, run different AI CLIs as each other's subagents, or just to avoid copy-pasting.

Works with: `claude`, `codex`, `opencode`, `copilot`, `qoder`, `grok`, `pi`, `omp`, `agy`, `cursor`, `kimi`, `kilo`, `gemini`, `hermes`

https://github.com/user-attachments/assets/1ce23ed9-f529-4be0-8124-816aa4c2fd43

## Install

```bash
# macOS, Linux, Android (Termux), and WSL
curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh
```

```powershell
# Windows (native, PowerShell)
irm https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.ps1 | iex
```

```bash
# Update any existing install
hcom update
```

## Quickstart

<table>
<tr>
<td width="50%" valign="top">

**Terminal 1:**

```bash
hcom claude
```

</td>
<td width="50%" valign="top">

**Terminal 2:**

```bash
hcom codex
```

</td>
</tr>
</table>

<b>Prompt:</b>

<details>
<summary><code>ask the other agent their favorite cake</code></summary>

- `review what claude did and send it fixes`

- `spawn 3x opencode, split work, collect results`

- `fork yourself to investigate the bug and report back`

- `when codex goes idle, send it the next task`

</details>

**Open the TUI dashboard:**

```bash
hcom
```

## What agents can do

- **Message** each other in real time: mid-turn or wake immediately when idle

- **Observe** each other: status, transcripts, file edits, live terminal screens, command history.

- **Subscribe** and notify on status changes, file edits, collisions, specific events. React automatically.

- **Spawn**, **fork**, **resume**, **kill** in any terminal emulator or headless.

## How it works

Hooks record activity to a local SQLite database and deliver messages from it.

```text
agent → hooks → db → hooks → other agent
```

Hooks activate only when an agent is launched with `hcom` in front. Normal usage is unaffected.

For hcom-requested work, agents send one reply only after the work is complete, then leave a
short terminal summary instead of a blank terminal or a duplicate full report.

<details>
<summary>Any other AI tool without hooks can join by running <code>hcom start</code></summary>

In any CLI tool, prompt:

```text
> run this command: `hcom start`
```

Keep it listening for messages:

```text
> stay connected to hcom
```

</details>

<details>
<summary>Any process can wake agents with <code>hcom send</code></summary>

Send messages from any process/script:

```bash
hcom send -b @luna -- "wake up and do this task"
```

Chain with `hcom events`:

```bash
hcom events --idle luna --wait 600 && hcom send -b @nova -- "luna is done, review it"
```

</details>

## Terminal

Every agent runs in a real terminal you can see, scroll, and interrupt. Any emulator works for spawning. **kitty**, **wezterm**, **tmux**, **zellij**, **waveterm**, **cmux**, **herdr**, and **Orca** also support closing panes from `hcom kill`.

With the Herdr preset, hcom supplies Herdr's native agent identity hint on Unix/macOS and uses
native descendant-process detection on Windows. Herdr remains responsible for agent state through
its screen manifests or installed integrations; hcom only supplies lifecycle reports for tools
Herdr does not recognize.

With `terminal = "orca"`, hcom opens one unfocused terminal tab in the local
Orca runtime, using the registered Orca workspace that matches the agent's
canonical working directory. Start the Orca desktop app or local `orca serve`
first. Remote Orca environments are intentionally rejected; normal hcom hooks,
messages, identity, and lifecycle remain authoritative. The Orca runtime must
advertise `terminal.create-interactive-agent.v1` and
`terminal.create-folder-workspace.v1` (available in the locally verified Orca
CLI 1.4.197). Unregistered local directories are added automatically as folder
workspaces; Git repositories and manual Orca registration are not required.

To configure a custom terminal open/close setup, tell an agent to run:

```bash
hcom config terminal --info
```

## Cross-device

Connect agents across machines via MQTT relay.

```bash
hcom relay new               # get token
hcom relay connect <token>   # on each device
```

```bash
hcom relay status   # check connection
hcom relay off|on   # toggle
```

> Treat the token like an API/SSH key. See [SECURITY.md](SECURITY.md)

## Troubleshoot

```bash
hcom status                  # diagnostics
hcom list                    # reconcile and show live agents
hcom list --stopped --all    # inspect stopped/stale history
hcom kill <name>             # remove one live managed instance
hcom reset all               # last resort: archive database, hooks, and config
```

For `Instance '<name>' already exists` after a reboot, run `hcom list` and retry.
For newly launched agents, listing compares the recorded process incarnation—not
just its reusable numeric PID—and immediately reconciles a process lost to exit or
reboot. Older records retain heartbeat-based stale cleanup. Current releases also
reconcile stale launch records automatically when reusing an explicit name.

Running another AI CLI directly from an hcom-managed agent is safe: foreign child
hooks cannot reuse the parent's inherited process identity. If an older release
already let a child overwrite a Claude instance's session metadata, the next
Claude hook restores the original identity when both transcript paths identify
the cross-tool mismatch.

## Uninstall

Safely remove all hcom hooks:

```bash
hcom hooks remove
```

Then remove binary:

```bash
rm "$(which hcom)"
```

---

## Reference

<details>
<summary><strong>Tools</strong></summary>

### Supported tools

| Tool | Message delivery | Connect |
|---|---|---|
| Claude Code | automatic | `hcom claude` |
| Gemini CLI | automatic | `hcom gemini` |
| Codex CLI | automatic | `hcom codex` |
| Antigravity CLI | automatic | `hcom agy` |
| OpenCode | automatic | `hcom opencode` |
| Kilo Code | automatic | `hcom kilo` |
| Pi | automatic | `hcom pi` |
| Oh My Pi | automatic | `hcom omp` |
| Cursor CLI | automatic | `hcom cursor-agent` |
| Kimi | automatic | `hcom kimi` |
| Copilot CLI | automatic | `hcom copilot` |
| Qoder CLI | automatic | `hcom qoder` |
| Grok Build | automatic | `hcom grok` |
| Hermes | automatic | `hcom hermes` |
| Anything else | manual via `hcom listen` | `hcom start` (run inside tool) |

```bash
hcom r <session_id>   # Resume a session started outside hcom
hcom f <session_id>   # Fork a session in hcom
```

#### Codex CLI configuration

When using OpenAI Codex CLI (especially with GPT-6 / Astra models), decorative TUI particle animations ("whimsy" / sparkles) can interfere with terminal screen scrapers and prompt readiness detection. Disable them in `~/.codex/config.toml`:

```toml
[tui]
whimsy = false
animations = false
```

#### Claude Code headless and subagents

Detached background processes in print mode stay alive. Manage through the TUI.

```bash
hcom claude -p 'say hi in hcom'   # print mode (separate Agent SDK credits)
hcom claude --headless            # Run normal claude in background pty (works for any tool)
```

For subagents, run `hcom claude`, then prompt:

> run 2x task tool and get them to talk to each other in hcom

</details>

<details>
<summary><strong>CLI</strong></summary>

### CLI commands

What you might type from a shell. Agents run their own commands that they learn from the hcom CLI primer (~700 tokens) at launch. `hcom <command> --help` for full flags.

#### Spawn

```bash
hcom [N] claude|gemini|codex|opencode|kilo|pi|omp|antigravity|cursor|kimi|copilot|qoder|grok|hermes   # launch N agents
hcom r <name|session_id>                # resume agent
hcom f <name|session_id>                # fork session
hcom kill <name|@group|tag:T|all>       # kill + close terminal pane
```

hcom launch flags:

| Flag | Purpose |
|---|---|
| `--as <name>` | Explicit agent name (single-agent launch only) |
| `--tag <name>` | Group label — agents can be addressed as `@tag` |
| `--terminal <preset>` | Where windows open: `default` (auto-detect), `kitty`, `wezterm`, `tmux`, `cmux`, `iterm`, etc… |
| `--dir <path>` | Directory where the agent launches |
| `--headless` | Run in background pty with no terminal window |
| `--device <name>` | Spawn on a remote device (via relay) |
| `--hcom-prompt <text>` | Initial user prompt |
| `--hcom-system-prompt <text>` | Invocation-local agent instructions (native channel or marked fallback) |
| `--dry-run` | Print the launch preview and run nothing (wins over `--go`) |

Anything else is forwarded verbatim to the tool: `--model sonnet`, `--yolo`, etc. A flag hcom does
not know is never an error here — it lands in the tool's argv, so a typo in an hcom flag surfaces as
the tool's own "unknown option". `--` ends hcom's flags explicitly, and `--dry-run` shows the
resulting command without launching anything.

### Named agents

To temporarily replace a CLI across projects without editing their catalogs, add
`"cli_overrides": {"codex": "claude"}` at the top level of `~/.hcom/agents.json`.
This applies once to catalog launches and send autostart, before selecting `tools.claude`;
explicit `--cli` bypasses it. Running agents keep their CLI. Use
`hcom agent <name> --restart --continue` to replace one with a handoff summary.
Keep CLI-specific models and arguments in `tools.<cli>`; shared fields still apply.
Remove the mapping to restore project preferences. See [local CLI replacement](docs/agent.md#local-cli-replacement).

`hcom agent` launches recurring agents from JSON settings and editable bundles. A bundle at
`~/.hcom/agents/<name>/SOUL.md` or an enclosing project `.hcom/agents/<name>/SOUL.md` defines an
agent even without a JSON entry. Its contents follow the catalog `system_prompt`; immediate
`skills/*/SKILL.md` children are advertised through a lazy-loading manifest for CLIs without
native bundle skill loading. Claude instead gets `<bundle>/.claude/skills` linked to `../skills`
when absent and receives the bundle through `--add-dir`, so its skills load natively without a
duplicate prompt manifest. Both instructions and skills are reread on clean start and named resume.
Bundle `AGENTS.md` files are not read as a fallback. For
Antigravity, an external bundle is made writable with
`agy --add-dir`. An instance name is unique: launching one that already runs prints its status and
exits. Use `--as` to run the same definition concurrently.

Catalog-launched Antigravity receives `DIPPY_POLICY_CWD` set to the canonical catalog agent
directory. Its hooks inherit this stable policy scope even when `--dir` overrides the launch
directory, a tool uses another `Cwd`, or `--add-dir` adds a bundle workspace.
A named resume through `hcom agent <name> --resume` derives the value again from the catalog;
`hcom r <name>` restores it from the stopped instance snapshot. Older snapshots without a
recorded scope do not receive the variable.

```bash
hcom agent wdt_main                 # launch (or report that it already runs)
hcom agent @wdt                     # launch every member of a catalog group
hcom agent wdt_main --as wdt_review # same config, independent instance named wdt_review
hcom agent wdt_main --continue      # clean session with handoff summary from previous session
hcom agent wdt_main --cli claude --continue # switch tool (e.g. from Codex) with previous context
hcom agent wdt_main --cli codex     # unknown flags are forwarded to `hcom <cli>`
hcom agent list                     # catalog + effective CLI/model + live status + source
hcom agent list --for-agents        # only names and catalog "description" entries
hcom agent list @wdt                # show only members of one catalog group
hcom agent list --all               # include agents hidden by recursive selective imports
hcom agent list --local             # only direct and imported agents from this project
hcom agent show wdt_main            # effective model/reasoning and the exact command
hcom agent edit                     # open the catalog in $EDITOR (creates a starter file)
```

Targeted messages start missing or stopped catalog agents before delivery. The send waits briefly
for that first event to be acknowledged; if startup is still settling, it succeeds with
`Queued; delivery pending` and keeps the event durable for later delivery. To start a catalog
agent under a custom instance name (or message an existing clone), use `--spawn-as <name>` (alias `--as <name>`).
Catalog precedence, imports, tool profiles, terminal placement, resume behavior, instruction transport, and bundle skills are
documented in [Named agents](docs/agent.md).

A catalog entry with `"roaming": true` is a project-local archetype. It omits `dir`, `session`,
and `window`; `hcom send @reviewer` resolves the sender's nearest Git root (then the nearest
`.hcom/agents.json`, then the sender directory), routes to `reviewer_<project>`, and starts that
instance in the resolved root when needed. Different projects therefore get separate sessions.
Catalog `env` is preserved during autostart, including an isolated policy such as
`"DIPPY_CONFIG_ONLY": "/path/to/reviewer.dippy"`. Broadcasts never materialize roaming agents.

The precedence chain is built-in defaults, global catalog `defaults`, each matching catalog's
`defaults` and named entry, global `cli_overrides`, the matching `tools.<cli>` profile, then
command-line flags. It is the same inside and outside a project. Later scalar values replace earlier ones: a project
`system_prompt` replaces the global text rather than appending to it, and `""` clears it. Recursive
imports apply before the importing catalog's local entries, and a catalog's `defaults` also cover
the agents it brings in, whether by import or as the enclosing project of a nested `.hcom`. An agent defined only in a project
catalog is addressable from inside that project, and from elsewhere only where a catalog in scope
imports it; a project's other agents stay private to it.

For a long shared prompt, set top-level `"system_prompt_file": "SYSTEM_PROMPT.md"`. The path is
resolved relative to that catalog file and its UTF-8 contents act as the catalog's default
`system_prompt`. An inline `defaults.system_prompt` in the same catalog overrides the file,
including `""` to clear it. Missing, unreadable, or non-UTF-8 referenced files are load errors;
omitting `system_prompt_file` preserves the existing inline-only behavior.

Catalog `session`/`window` placement is honored when Herdr is the configured default, including
nested launches and targeted-message autostart; a parent agent's Herdr location is not inherited.

#### Other commands

```bash
hcom                                # TUI dashboard
hcom send -b @luna -- hey           # one-off message to an agent
hcom list                           # show all active agents
hcom term [name]                    # view/inject into an agent's PTY screen
hcom agent <name>                   # launch a named agent from the catalog
hcom events --wait <filters>         # Block until match for scripting
hcom completions [shell]            # generate shell completions (bash, zsh, fish)
hcom update                         # update hcom version
```

`hcom run docs --cli` for all commands.

</details>

<details>
<summary><strong>Config</strong></summary>

### Configuration

Config lives in `~/.hcom/config.toml`. Precedence: defaults < `config.toml` < env vars.

```bash
hcom config                           # show all values with sources
hcom config <key>                     # get
hcom config <key> <value>             # set
hcom config <key> --info              # detailed help for a key
hcom config -i <name> <key> <value>   # per-agent override at runtime
```

#### Keys

| Key | Purpose |
|---|---|
| `tag` | Group label — launched agents become `tag-name` |
| `hints` | Text appended to every message the agent receives |
| `notes` | Text appended to bootstrap (one-time, at launch) |
| `auto_approve` | Auto-approve safe hcom commands (send/list/events/…) |
| `auto_subscribe` | Event subscription presets: `collision`, `created`, `stopped`, `blocked` |
| `name_export` | Export instance name to a custom env var |
| `title_mode` | Terminal/tab title behavior: `combined` (default), `label`, or `off` |
| `terminal` | Where new agent windows open (`hcom config terminal --info`) |
| `timeout` | Idle timeout for headless Claude (seconds) |
| `subagent_timeout` | Keep-alive for Claude subagents (seconds) |
| `claude_args` / `gemini_args` / `codex_args` / `opencode_args` / `kilo_args` / `pi_args` / `omp_args` / `cursor_args` / `kimi_args` / `copilot_args` / `qoder_args` / `grok_args` | Default args passed to the tool |

#### Scope

```bash
hcom config tag mycrew                        # global
hcom config -i luna hints "respond in JSON"   # per-agent
HCOM_TAG=dev hcom 3 claude                    # per-launch env
```

#### Per-project isolation

```bash
export HCOM_DIR="$PWD/.hcom"   # isolate hcom state (db, logs) to this folder
rm -rf "$HCOM_DIR"             # clean up
```

Run `hcom config <key> --info` or `hcom run docs --config` for the full per-key reference.

Edit `~/.hcom/env` to set external env vars passed to every launched agent.
If this file exists before the first hcom run, creating `config.toml` preserves it.
On Unix, an open TUI reconnects to the new database after `hcom reset`.

</details>

<details>
<summary><strong>Workflow Scripts</strong></summary>

### Multi-agent workflows

Bundled and user scripts (`~/.hcom/scripts/`) for multi-agent patterns:

```bash
hcom run                  # list available scripts
hcom run debate "topic"   # run one
hcom run docs             # tell agent to run this to create any new workflow
```

#### Included scripts

Tell agent to run them:

- **`hcom run confess`** — An agent (or background clone) writes an honesty self-eval. A spawned calibrator reads the target's transcript independently. A judge compares both reports and sends back a verdict via hcom message.

- **`hcom run debate`** — A judge spawns and sets up a debate with existing agents. It coordinates rounds in a shared thread where all agents see each other's arguments, with shared context of workspace files and transcripts.

- **`hcom run fatcow`** — headless agent reads every file in a path, subscribes to file edit events to stay current, and answers other agents on demand.

- **`hcom run onidle`** — waits for an agent to go idle, then types text into another agent (`hcom run onidle luna nova 'luna is done, review it'`) or launches a new one with it as the prompt (`hcom run onidle luna codex 'review what luna just did'`).

Custom scripts: drop `*.sh` or `*.py` into `~/.hcom/scripts/` — auto-discovered, override bundled scripts of the same name. Ask an agent to author one; `hcom run docs --scripts` is the authoring guide.

</details>

## Contributing

Issues and PRs welcome. Build from source and dev setup: [CONTRIBUTING.md](CONTRIBUTING.md)

## License

[MIT](LICENSE)

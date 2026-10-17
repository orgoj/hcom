<div align="center">

# `hcom`

*Hook your coding agents together*

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

</div>

**CLI tool that agents use to message, watch, and spawn each other across terminals.**

Start an agent with `hcom` in front, then prompt normally.

Use it to coordinate multi-agent pipelines, run different AI CLIs as each other's subagents, or just to avoid copy-pasting.

Works with: `claude`, `codex`, `opencode`, `copilot`, `qoder`, `grok`, `hermes`, `pi`, `omp`, `agy`, `cursor`, `kimi`, `kilo`, `gemini`

This is the [orgoj fork](https://github.com/orgoj/hcom) of [aannoo/hcom](https://github.com/aannoo/hcom). Upstream behavior runs unchanged; the fork adds named agents from a JSON catalog (`hcom agent`, under Reference below), catalog groups and roaming agents, send autostart, idle autokill, `hcom handoff`, and a desktop window for the catalog ([contrib/hcom-gui](contrib/README.md)).

https://github.com/user-attachments/assets/1ce23ed9-f529-4be0-8124-816aa4c2fd43

## Install

The fork builds from source and needs Rust 1.89 or newer. The `uv` and `brew` packages install upstream hcom, without the fork's features.

```bash
cargo install --git https://github.com/orgoj/hcom --locked
```

To follow the repository instead, build it and link the binary:

```bash
git clone https://github.com/orgoj/hcom.git
cd hcom
cargo build --release
ln -sf "$PWD/target/release/hcom" ~/.cargo/bin/hcom
```

Update with `git pull && cargo build --release`.

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

- Message each other in real time, mid-turn or waking an idle agent at once.
- Read each other's status, transcripts, file edits, terminal screens and command history.
- Subscribe to status changes, file edits, collisions or any event query, and react when one fires.
- Spawn, fork, resume and kill agents in any terminal emulator or headless.
- Start named agents from a catalog, by name, by group, or by sending them a message.

## How it works

Hooks record activity to a local SQLite database and deliver messages from it.

```text
agent → hooks → db → hooks → other agent
```

Hooks activate only when an agent is launched with `hcom` in front. Normal usage is unaffected.

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

Every agent runs in a real terminal you can see, scroll, and interrupt. Any emulator works for spawning. kitty, wezterm, tmux, zellij, waveterm, cmux, herdr and orca also close the agent's pane on `hcom kill`. `--terminal here` runs the agent in the current terminal.

With **herdr**, each catalog agent is placed in its configured tab (default: a tab named after the agent; `hcom agent --help`). `hcom r` returns an agent to its recorded workspace and tab, `hcom f` opens a tab named after the fork, and no launch inherits the caller's tab.

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
hcom status   # diagnostics
```

```bash
hcom reset all   # clear and archive: database + hooks + config
```

## Uninstall

Safely remove all hcom hooks:

```bash
hcom hooks remove
```

Then remove the binary:

```bash
cargo uninstall hcom
# or, for a linked build: rm "$(which hcom)"
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
| Hermes Agent | automatic | `hcom hermes` |
| Anything else | manual via `hcom listen` | `hcom start` (run inside tool) |

```bash
hcom r <session_id>   # Resume a session started outside hcom
hcom f <session_id>   # Fork a session in hcom
```

#### Claude Code headless and subagents

Detached background processes in print mode stay alive. Manage through the TUI.

```bash
hcom claude -p 'say hi in hcom'   # print mode (separate Agent SDK credits)
hcom claude --headless            # Run normal claude in background pty (works for any tool)
```

For subagents, run `hcom claude`, then prompt:

> run 2x task tool and get them to talk to each other in hcom

#### Hermes chats

A Hermes gateway chat (Telegram, Discord, etc) can join hcom:

```bash
hcom hooks add hermes   # plugin into the active profile (HERMES_HOME picks another); a running gateway loads it
```

In the chat: `/hcom start` connects, `/hcom stop` disconnects.

`hcom hooks remove hermes` disconnects and removes its plugin.

</details>

<details>
<summary><strong>CLI</strong></summary>

### CLI commands

What you might type from a shell. Agents run their own commands that they learn from the hcom CLI primer (~700 tokens) at launch. `hcom <command> --help` for full flags.

#### Spawn

```bash
hcom [N] claude|gemini|codex|agy|opencode|kilo|pi|omp|cursor-agent|kimi|copilot|qoder|grok|hermes   # launch N agents
hcom r <name|session_id>     # resume agent
hcom f <name|session_id>     # fork session
hcom kill <name|@group|tag:T|tool:CLI|SELF|all>   # kill + close terminal pane (SELF = the calling agent)
```

hcom launch flags:

| Flag | Purpose |
|---|---|
| `--tag <name>` | Group label; agents become `tag-name` and answer to `@tag-` |
| `--terminal <preset>` | Where windows open: `default` (auto-detect), `here`, `kitty`, `wezterm`, `tmux`, `herdr`, `orca`, ... |
| `--dir <path>` | Directory where the agent launches |
| `--headless` | Run in a background pty with no terminal window |
| `--as <name>` | Explicit instance name (single agent only) |
| `--device <name>` | Spawn on a remote device (via relay) |
| `--hcom-prompt <text>` | Initial user prompt |
| `--hcom-system-prompt <text>` | Agent instructions for this launch only |
| `--dry-run` | Print the launch, run nothing |

Anything else is forwarded to the tool: `--model sonnet`, `--yolo`, etc.

#### Other commands

```bash
hcom                           # TUI dashboard
hcom send -b @luna -- hey      # one-off message to an agent (-b = from bigboss, the human)
hcom send -b @review -- hey    # catalog group: its in-scope members, started if stopped
hcom list                      # active agents; list <name> for details, --stopped for stopped ones
hcom listen 60                 # block until a message arrives
hcom events --wait <filters>   # block until a matching event, for scripting
hcom events sub --idle luna    # get an hcom message when luna goes idle
hcom transcript luna           # read luna's conversation; transcript search 'regex'
hcom bundle prepare            # context package for a handoff
hcom term [name]               # view/inject into an agent's PTY screen
hcom agent <name>              # launch a catalog agent (see Agent catalog)
hcom handoff <prompt>          # a catalog agent restarts itself in a clean session
hcom stop [name]               # disconnect without killing
hcom archive                   # query past sessions
hcom hooks [add|remove]        # inspect or change tool hooks
hcom completions bash          # shell completion (bash|zsh|fish)
hcom update                    # install the latest orgoj/hcom release
```

`hcom run docs --cli` for all commands.

</details>

<details>
<summary><strong>Agent catalog</strong></summary>

### Agent catalog

`hcom agent` launches named agents from a JSON catalog. Each entry fixes the CLI, directory, model, terminal placement and prompts, so `hcom agent docs` starts the same agent every time, and `hcom send @docs` starts it if it is not running.

```json
{
  "defaults": { "cli": "claude", "terminal": "herdr", "idle_kill": 3600 },
  "agents": {
    "docs": {
      "description": "Keeps README and help text in sync with the code",
      "dir": "~/projects/hcom",
      "session": "hcom",
      "model": "sonnet",
      "resume": true,
      "groups": ["review"]
    },
    "reviewer": {
      "description": "Reviews the sender's diff and replies with findings",
      "roaming": true,
      "cli": "codex",
      "groups": ["review"],
      "tools": { "codex": { "model": "gpt-5", "reasoning": "high" } }
    },
    "old_helper": { "dir": "~/projects/old", "disabled": true }
  }
}
```

#### Commands

```bash
hcom agent docs                  # launch; reports the status if docs already runs
hcom agent docs --resume         # continue the previous session (--continue: clean session + handoff summary)
hcom agent docs --as docs2       # same definition, second independent instance
hcom agent docs --restart        # kill the running instance, wait, launch again
hcom agent @review               # launch every member of a catalog group
hcom agent show docs             # effective config and the exact command
hcom agent docs --dry-run        # print the commands, run nothing
hcom agent list                  # table in a terminal, name + description elsewhere
hcom agent list --all            # also agents hidden by selective imports, and disabled ones
hcom agent list --local          # project agents only
hcom agent list @review --names  # one group, names only
hcom agent attach docs           # focus the running agent's Herdr pane
hcom agent edit [--project]      # open the global (or project) catalog in $VISUAL/$EDITOR
hcom agent completions bash      # completion for agent and group names (bash|zsh|fish)
```

`show` and `attach` accept the same flags as a launch (`--as`, `--cli`, `--catalog`, ...), so `hcom agent show docs --as docs2` previews that variant. `list` also takes `--json`, `--groups`, `--for-agents` and `--for-humans`.

Launch flags override the catalog for one run: `--cli` (alias `--tool`), `--dir`, `--model`, `--reasoning`, `--tag`, `--terminal`, `--terminal-command`, `--session`, `--window`, `--hcom-prompt`, `--hcom-system-prompt`, `--env K=V`, `--inherit-env VAR`, `--attach`, `--clean`, `--last N`. `--catalog <file>` replaces the project catalog and `--no-project` ignores it. hcom forwards every other flag to the tool. `hcom agent --help` has the full reference.

#### Agent fields

| Field | Meaning |
|---|---|
| `description` | One line other agents read in `hcom agent list` |
| `cli` | Tool to run (default `claude`) |
| `dir` | Working directory (`~` and `$VAR` expand) |
| `model`, `reasoning` | Model and reasoning effort (`--effort` for Claude/Antigravity, `model_reasoning_effort` for Codex) |
| `prompt`, `system_prompt` | Initial user prompt and agent instructions; `system_prompt` replaces earlier layers, `""` clears it |
| `terminal`, `terminal_command` | hcom terminal preset, or a raw command with `{script}` |
| `session`, `window` | Herdr workspace and tab (tab defaults to the agent name) |
| `tag` | Runtime messaging tag, as with `hcom --tag` |
| `groups` | Catalog groups for `hcom agent @group`, `hcom send @group` and `hcom kill @group` |
| `env`, `inherit_env`, `args` | Extra environment, variables copied from the caller, extra tool arguments |
| `resume`, `continue` | Default start mode; `--resume`, `--continue` and `--clean` override it |
| `idle_kill` | Seconds of idle listening before hcom kills the instance (`0` or `false` off) |
| `roaming` | Per-project instances, see below |
| `disabled` | `true` hides the entry, see below |
| `tools` | Per-CLI profile: `model`, `reasoning`, `prompt`, `system_prompt`, `args` for the effective CLI |

Any field also works in a catalog's `defaults`. Scalars from a later layer replace earlier ones; `env`, `args`, `inherit_env` and `groups` accumulate.

#### Where catalogs live

hcom reads these catalogs in order, each later one stronger:

1. the global catalog `~/.hcom/agents.json` (path override: `HCOM_AGENTS_FILE`)
2. extra catalogs listed in `HCOM_AGENT_CATALOGS`, left to right
3. every `.hcom/agents.json` from the launch directory upward, outermost first; no Git required

Each catalog applies its `defaults`, then the entry. Global `cli_overrides` follow, then the `tools.<cli>` profile of the resulting CLI, and command-line flags win over everything. A project agent ignores a same-named global entry but still inherits global `defaults`. Relative `dir` values resolve against `$HOME` in the global catalog, the project root in a project catalog, and the file's own directory elsewhere.

A catalog can import agents from other files. Paths resolve against the importing file:

```json
{
  "imports": [{ "from": "../shared/agents.json", "agents": ["builder"] }],
  "agents": { "tester": { "dir": ".", "cli": "codex" } }
}
```

Without `agents`, an import brings in every agent of that file; with a list, the others stay private to it. A project agent is reachable by `hcom agent` and `hcom send` inside its project, and elsewhere only where a catalog in scope imports it.

#### Groups

`"groups": ["review"]` puts an agent in a catalog group, separate from the messaging `tag`. `hcom agent @review` launches every member, including agents a selective import keeps private. `hcom send @review -- text` reaches the members in scope and starts stopped ones. A running agent or catalog agent named `review` takes precedence over the group.

#### Roaming agents

`"roaming": true` defines a project-local archetype with no `dir`, `session` or `window`. A targeted `hcom send @reviewer` from inside a Git project addresses `reviewer_<project>` and starts it in that project's root if it is missing. Broadcasts never start roaming agents.

#### Disabled agents

`"disabled": true` switches an agent off without deleting its entry. hcom then treats the name as undefined: no launch, resume, send autostart, roaming instance, group membership, `show`, listing or completion. The field merges like any other, so `"disabled": true` in `defaults` with `"disabled": false` on one entry keeps only that entry enabled. `hcom agent list --all` still shows disabled agents, marked `disabled`. A running instance stays an ordinary hcom agent: it receives messages and can be killed by name.

#### Bundles and instructions

An `agents/<name>/SOUL.md` next to a catalog defines that agent too and is appended to its instructions after `system_prompt`. Skills in `agents/<name>/skills/*/SKILL.md` belong to it; Claude loads them natively through `--add-dir`. A top-level `"system_prompt_file"` sets a catalog-wide default from a file.

#### Swapping a CLI locally

Only the global catalog accepts `"cli_overrides": {"codex": "claude"}`. It maps a catalog's CLI choice for named launches, groups and send autostart, without editing shared catalogs. `--cli` bypasses it, and running instances keep their tool. `hcom agent docs --restart --continue` moves a running agent to the new CLI with a handoff summary.

#### Environment and handoff

Every launched agent gets `HCOM_INSTANCE_NAME` (its instance name) and `HCOM_AGENT_NAME` (the catalog entry it came from; they differ under `--as` and for roaming agents). An agent started by `hcom agent` or send autostart can restart itself in a clean session with `hcom handoff <prompt>`, keeping its name and catalog entry.

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
| `tag` | Group label; launched agents become `tag-name` |
| `hints` | Text appended to every message the agent receives |
| `notes` | Text appended to bootstrap (one-time, at launch) |
| `auto_approve` | Auto-approve safe hcom commands (send/list/events/…) |
| `auto_subscribe` | Event subscription presets: `collision`, `created`, `stopped`, `blocked` |
| `name_export` | Export instance name to a custom env var |
| `title_mode` | Terminal/tab title behavior: `combined` (default), `label`, or `off` |
| `terminal` | Where new agent windows open (`hcom config terminal --info`) |
| `timeout` | Idle timeout for headless Claude (seconds) |
| `subagent_timeout` | Keep-alive for Claude subagents (seconds) |
| `claude_args` / `gemini_args` / `codex_args` / `opencode_args` / `kilo_args` / `pi_args` / `omp_args` / `cursor_args` / `kimi_args` / `copilot_args` / `qoder_args` / `grok_args` / `hermes_args` | Default args passed to the tool |
| `claude_delivery` / `copilot_delivery` | Idle delivery: `inject` (type into the PTY, default) or `native` |
| `gemini_system_prompt` / `codex_system_prompt` | Default system prompt for that tool |
| `auto_trust_workspace` | Trust the launch directory, skipping the tool's folder-trust prompt |
| `herdr_autostart` | Start the Herdr server when it is not running |
| `continue_last` | Recent exchanges carried into a `--continue` handoff summary |

#### Scope

```bash
hcom config tag mycrew                        # global
hcom config -i luna hints "respond in JSON"   # per-agent
HCOM_TAG=dev hcom 3 claude                    # per-launch env
```

#### Idle autokill

```bash
hcom config -i luna idle_kill 600   # kill luna after 10 min idle; 0 clears
```

A running local agent (not a remote one or a subagent) with `idle_kill` set is killed like `hcom kill` once it has been listening that long with nothing unread and no own `intent=request` message still waiting for a reply. A catalog entry or catalog `defaults` can set `"idle_kill": <seconds>` (`0` or `false` disables) for every launch; roaming agents restart on the next send, others resume with `hcom r`. The stopped event records `idle_kill` as the initiator. Enforcement runs inside hcom's own loops and commands, so it needs at least one hcom-managed agent or command running.

#### Per-project isolation

```bash
export HCOM_DIR="$PWD/.hcom"   # isolate hcom state (db, logs) to this folder
rm -rf "$HCOM_DIR"             # clean up
```

Run `hcom config <key> --info` or `hcom run docs --config` for the full per-key reference.

Edit `~/.hcom/env` to set external env vars passed to every launched agent.

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

- `hcom run confess`: an agent (or a background clone) writes an honesty self-evaluation, a spawned calibrator reads the target's transcript on its own, and a judge compares both reports and sends the verdict as an hcom message.

- `hcom run debate`: a judge sets up a debate between existing agents and runs the rounds in a shared thread, where every agent sees the others' arguments, workspace files and transcripts.

- `hcom run fatcow`: a headless agent reads every file in a path, follows file edit events to stay current, and answers other agents' questions about it.

- `hcom run onidle`: waits for an agent to go idle, then types text into another agent (`hcom run onidle luna nova 'luna is done, review it'`) or launches a new one with the text as its prompt (`hcom run onidle luna codex 'review what luna just did'`).

Custom scripts: put `*.sh` or `*.py` files in `~/.hcom/scripts/`. hcom finds them on its own, and they override bundled scripts of the same name. `hcom run docs --scripts` is the authoring guide; an agent can write one from it.

</details>

<details>
<summary><strong>Desktop GUI</strong></summary>

### hcom-gui

`contrib/hcom-gui` is a Tk window that lists every catalog agent next to every live instance. It starts, kills, attaches and messages agents through the `hcom` CLI, and shows status, context use, Git state and terminal placement per row. It needs Python 3 with tkinter.

```bash
ln -s "$PWD/contrib/hcom-gui" ~/.local/bin/hcom-gui
hcom-gui
```

Buttons and columns are configured in `~/.hcom/gui.json`; [contrib/README.md](contrib/README.md) documents the options and keyboard shortcuts.

</details>

## Contributing

Issues and PRs welcome. Build from source and dev setup: [CONTRIBUTING.md](CONTRIBUTING.md)

## License

[MIT](LICENSE)

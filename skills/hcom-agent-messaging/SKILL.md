---
name: hcom-agent-messaging
description: >
  Multi-agent communication for AI coding tools. Agents message, watch,
  and spawn each other across terminals. Use when setting up hcom,
  troubleshooting delivery, or writing multi-agent scripts.
---

# hcom — multi-agent communication for AI coding tools

AI agents running in separate terminals are isolated. hcom connects them via hooks and a shared database so they can message, watch, and spawn each other in real-time.

```bash
curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh
hcom claude       # or: hcom gemini, hcom codex, hcom opencode, hcom kilo, hcom pi, hcom omp, hcom agy, hcom cursor-agent, hcom kimi, hcom copilot
hcom              # TUI dashboard
```

---

## what humans can do

tell any agent:

> send a message to claude

> when codex goes idle send it the next task

> watch gemini's file edits, review each and send feedback if any bugs

> fork yourself to investigate the bug and report back

> find which agent worked on terminal_id code, resume them and ask why it sucks

---

## what agents can do

**Message** each other in real-time, bundle context for handoffs.

**Observe** each other: transcripts, file edits, terminal screens, command history.

**Subscribe** to each other: notify on status changes, file edits, specific events. React automatically.

**Spawn**, **fork**, **resume**, **kill** each other, in any terminal emulator.

run `hcom --help` for full command syntax and flags.

---

## spawning named agents with terminal access

Use `--as` for one intentional, stable agent name. It is valid only when launching one agent:

```bash
hcom codex --as audit_api --dir /path/to/repo --terminal tmux-window --hcom-prompt "Inspect authentication and report back" --go
```

The `tmux-window` preset creates a window named `hcom-audit_api` in the launching agent's current tmux session. Switch to it with normal tmux window navigation.

Use `--terminal tmux` when the child needs its own detached session. A human can access that session later:

```bash
tmux attach -t hcom-audit_api
```

Use `--terminal tmux-split` only when the child should split the launching agent's current window. Always pass `--dir` when the child must work in another repository. Use `hcom send @audit_api -- "..."` for follow-up work and `hcom kill audit_api` to stop the agent and close its managed tmux pane.

For agents defined in the effective `hcom agent` JSON catalog, a targeted send is also the normal
launch operation: `hcom send @audit_api --intent request -- "..."` starts `audit_api` when it is
missing or stopped, then delivers the message. Do not add `hcom list`/`hcom agent` preflight logic.
Broadcasts do not auto-start catalog agents. See `references/named-agents.md` for routing details.

For multiple agents, omit `--as` and capture the generated names from launch output; one explicit name cannot be assigned to a multi-agent launch.

---

## tool support

| tool | delivery | connect |
|------|----------|---------|
| claude code (incl. subagents) | automatic | `hcom claude` |
| gemini cli (>= 0.26.0) | automatic | `hcom gemini` |
| codex | automatic | `hcom codex` |
| opencode | automatic | `hcom opencode` |
| kilo code | automatic | `hcom kilo` |
| antigravity | automatic | `hcom agy` |
| cursor | automatic | `hcom cursor-agent` |
| any other ai tool | manual via `hcom listen` | `hcom start` (run inside tool) |

session binding (hcom transcript, hcom r/f by session id) happens on first message or first prompt for all hcom-launched tools.

---

## setup

if the user invokes this skill without arguments:

1. run `hcom status` — if "command not found", install first:
   ```bash
   curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh
   ```
2. run `hcom hooks add` to install hooks for all detected tools
3. restart the AI tool for hooks to activate

| status output | meaning | action |
|---------------|---------|--------|
| command not found | not installed | install with the curl installer above (or the PowerShell installer on Windows) |
| `[~] claude` | tool exists, hooks not installed | `hcom hooks add` then restart |
| `[✓] claude` | hooks installed | ready |
| `[✗] claude` | tool not found | install the AI tool first |

---

## troubleshooting

### "hcom not working"

```bash
hcom status          # check installation
hcom hooks status    # check hooks specifically
hcom relay status    # check cross-device relay
```

hooks missing? `hcom hooks add` then restart tool.

still broken?
```bash
hcom reset all && hcom hooks add
# close all ai tool windows
hcom claude          # fresh start
```

### "messages not arriving"

| symptom | diagnosis | fix |
|---------|-----------|-----|
| catalog agent not in `hcom list` | agent stopped or never launched | target it directly; `hcom send` starts it on demand |
| message sent but not delivered | check `hcom events --last 5` | verify @mention matches agent name/tag |
| message reaches more than one agent | duplicate base name across tags | target the full `@tag-name` to hit exactly one |
| messages leaking between workflows | no thread isolation | always use `--thread` |

### "Instance '<name>' already exists"

Preserve evidence before running `hcom list`, because listing may reconcile stale rows:

```bash
sqlite3 ~/.hcom/hcom.db \
  "SELECT name, status, status_time, status_context, pid, session_id
   FROM instances WHERE name = '<name>'"
hcom events --agent <name> --last 20
```

Then check whether the process or terminal still exists. If the instance is dead, run
`hcom list` to trigger normal stale reconciliation and retry the launch. Use
`hcom kill <name>` only for a genuinely live managed instance. Do not use
`hcom reset all` for a single-name collision.

### intent system

agents follow these rules from their bootstrap:
- `--intent request` -> agent always responds
- `--intent inform` -> agent responds only if useful
- `--intent ack` -> agent does not respond

Choose replies from the received intent, not from conversational politeness:

- Do not send receipt acknowledgements, work-started notices, progress updates,
  status messages, or conversational filler. For a request, work silently and
  send one completed result as `--intent inform --reply-to <request-id>`.
- If blocked on missing information, send one concrete question as
  `--intent request --reply-to <request-id>`, then continue after the answer.
- Do not reply to `inform` unless it requires a concrete substantive response.
- Reserve `ack` for an explicit protocol that specifically requires a receipt;
  ordinary agent tasks never require it.
- Treat a message as delivered only when `hcom send` exits successfully and
  prints `Sent to:`. On any CLI error, the message was not delivered; correct
  the command and retry before proceeding or reporting success.

### Hermes / shell delivery guardrails

- Pass ordinary message text directly as the argument after `--`, using normal
  shell quoting. Keep messaging to one `hcom send` command; do not add encoding
  commands or use `--base64`/`--file` unless the payload itself specifically
  requires that transport.
- `hcom` can append unread messages to the stdout of ordinary identity-bound
  commands; `listen` is therefore not the only receive path. Inspect every
  hcom command result for delivered messages before issuing another command.
- Use foreground `hcom listen` only when the user explicitly wants continuous
  waiting. After a message arrives, process and report it; do not repeatedly
  block the parent agent if the result already answers the active task.
- A CLI-output delivery is not native Hermes gateway injection: it becomes
  visible only while an hcom command runs. Do not claim automatic inbound
  delivery without a gateway bridge or another real integration.

### sandbox / permission issues

```bash
export HCOM_DIR="$PWD/.hcom"     # project-local mode
hcom hooks add                   # installs to project dir
```

---

## workflow scripting

place scripts in `~/.hcom/scripts/` as `.sh` or `.py`. run with `hcom run <name> "task"`. see `references/script-template.md` for the full annotated template, or run `hcom run docs --scripts` inside an agent.

### key rules

- **never use `sleep`** — use `hcom events --wait` or `hcom listen`
- **never hardcode generated agent names** — parse them from `grep '^Names: '` in launch output; `--as` is only for intentional single-agent names
- **always use `--thread`** — without it, messages leak across workflows
- **always use `trap cleanup ERR INT TERM`** — orphan headless agents run indefinitely
- **always use `hcom kill` for cleanup** (not `stop`) — kill also closes the terminal pane
- **always forward `--name`** — hcom injects it, scripts must propagate it
- **always use `--go`** on launch commands — without it, scripts hang on confirmation prompt (`hcom kill` never prompts, so `--go` is optional there)

### agent topologies

| topology | agents | pattern |
|----------|--------|---------|
| worker-reviewer | 2 | worker sends result, reviewer reads transcript, sends APPROVED/FIX |
| pipeline | N sequential | each stage reads previous via `hcom transcript`, signals via thread |
| ensemble | N+1 (judge) | N agents answer independently, judge reads all via `hcom events --sql` |
| hub-spoke | 1+N | coordinator broadcasts to `@tag-`, workers report back |
| reactive | N | `hcom events sub` triggers agent actions on file edits/status changes |

---

## files

| what | location |
|------|----------|
| database | `~/.hcom/hcom.db` |
| config | `~/.hcom/config.toml` |
| logs | `~/.hcom/.tmp/logs/` |
| user scripts | `~/.hcom/scripts/` |

with `HCOM_DIR` set, uses that path instead of `~/.hcom`.

---

## reference files

| file | when to read |
|------|-------------|
| `references/named-agents.md` | defining or launching recurring agents with `hcom agent`, JSON catalogs, start-mode overrides, and per-CLI `tools` profiles |
| `references/patterns.md` | writing multi-agent scripts — 6 tested patterns with full code and real event JSON |
| `references/cross-tool.md` | claude + codex + gemini + opencode + kilo + pi + omp + antigravity + cursor + kimi + copilot collaboration details and per-tool quirks |
| `references/gotchas.md` | debugging scripts — timing, message delivery, intent system, cleanup |
| `references/script-template.md` | writing a new script from scratch — full template with commentary |
| `references/scripts/` | 6 tested, working example scripts |

---

## more info

```bash
hcom --help              # all commands
hcom <command> --help    # command details
```

github: https://github.com/orgoj/hcom

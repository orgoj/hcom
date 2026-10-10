---
name: hcom-agent-messaging
description: >
  Multi-agent communication for AI coding tools. Agents message, watch,
  and spawn each other across terminals. Use when setting up hcom or
  troubleshooting.
---

# hcom — multi-agent communication for AI coding tools

AI agents running in separate terminals are isolated. hcom connects them via hooks and a shared database so they can message, watch, and spawn each other in real-time.

Start an agent with `hcom` in front, then prompt normally.

Use hcom to:

- coordinate multi-agent pipelines
- run different AI CLIs as each other's subagents
- avoid copy-pasting

```bash
curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh
hcom claude       # or: hcom gemini, hcom codex, hcom opencode, hcom kilo, hcom pi, hcom omp, hcom antigravity (agy binary), hcom cursor (cursor-agent binary), hcom kimi, hcom copilot, hcom qoder, hcom grok, hcom hermes
hcom              # TUI dashboard
```

Quickstart:

```bash
# terminal 1
hcom claude

# terminal 2
hcom codex
```

Prompt normally — e.g. `review what claude did and send it fixes`

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

**Message** each other in real time: mid-turn or wake immediately when idle

**Observe** each other: status, transcripts, file edits, live terminal screens, command history.

**Subscribe** and notify on status changes, file edits, collisions, specific events. React automatically.

**Spawn**, **fork**, **resume**, **kill** in any terminal emulator or headless.

run `hcom --help` for full command syntax and flags.

---

Works with Claude Code, Gemini CLI, Codex, OpenCode, Kilo Code, Pi, Oh My Pi, Antigravity, Cursor, Kimi, Copilot, Qoder CLI, Grok Build, Hermes, and other tools

---

## spawning named agents with terminal access

### Two registries, two commands

| what you want to know | command | analogy |
|---|---|---|
| who is running right now | `hcom list` | `ps` |
| who can be addressed at all | `hcom agent list` | an address book |

`hcom agent list` prints one `<name>  <description>` line per agent configured in the effective
`hcom agent` JSON catalog. `hcom list` prints only live instances; a configured agent missing from
it is stopped, not unknown.

**A name the user gives is exact.** `bin` means the agent named `bin`, never `project1_bin` or any
other similar entry that happens to be running. Resolve a name by exact match in `hcom list`, then
by exact match in `hcom agent list`. If neither matches, ask the user — do not pick a neighbour.

- Send the task directly with `hcom send @<name> --intent request -- "..."`. Targeted send resolves
  the catalog and automatically starts a missing or stopped configured agent, so no `hcom list` or
  `hcom agent` preflight is needed.
- To start a catalog agent under an adhoc clone/alias name (e.g. to solve an independent issue in clean
  context), use `hcom send @<catalog_name> --spawn-as <clone_name> -- "..."` (alias `--as`). If the clone
  is already running, send delivers directly without relaunching.
- Catalogs are scoped on purpose: an agent defined only in a project `.hcom/agents.json` is
  addressable from inside that project, and from elsewhere only where a catalog in scope imports
  it. A project's other agents are private to it. From outside, an out-of-scope name is
  indistinguishable from a name that does not exist, and that is intended — report it as
  unreachable from here rather than guessing at what a project might be hiding.
- A catalog entry with `roaming: true` is an archetype. Send to its catalog name normally; hcom
  resolves the sender's project and routes to `<name>_<project>`, starting that instance in the
  project root when missing. Do not invent the materialized name yourself or pre-launch it.
- An Antigravity catalog agent whose bundle is outside its working directory receives that bundle
  through `agy --add-dir`; do not move or duplicate the bundle into the project.
- If the user distinguishes an "hcom agent" from an "hcom instance", preserve that distinction
  literally.
- Treat every new or updated catalog agent as clean-starting. Do not add `"resume": true`, pass
  `--resume`, or run `hcom r <name>` unless the user explicitly asks to resume that agent's
  previous tool session. “Persistent,” “recurring,” or “catalog agent” means the definition
  persists, not the tool session. Existing catalog `resume: true` is not permission to add it to
  another agent. `hcom r` resumes a stopped session; it is not how you address or launch an agent.
- The global catalog's top-level `cli_overrides` can replace a configured CLI for named launches,
  groups, and send autostart without editing project catalogs. Explicit `--cli` bypasses it.
  Running agents and direct `hcom r/f` retain their tool; use `--restart --continue` for a CLI
  switch with handoff. Keep CLI-specific settings in `tools.<cli>`; shared fields still apply.
- When the user asks to launch a catalog group, use `hcom agent @<group>`. Catalog `groups` are
  launch-only and are independent of the runtime `tag` used for message routing.

To run the same catalog definition concurrently, give each runtime instance a distinct name:

```bash
hcom agent reviewer --as review_api
hcom agent reviewer --as review_backend
```

The positional name selects catalog configuration; `--as` selects the runtime identity used for
messaging and lifecycle operations. Aliases are not catalog entries and targeted sends do not
auto-start them. Catalog layers resolve identically inside and outside a project; see
`references/named-agents.md` for the complete precedence and scalar replacement rules.

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
For catalog-launched Antigravity agents, hcom passes the canonical catalog directory as
`DIPPY_POLICY_CWD` to the CLI and its hooks; extra `--add-dir` workspaces do not change it.
Direct tracked `hcom r <name>` restores the scope from the stopped snapshot.
Broadcasts do not auto-start catalog agents. Send briefly waits for an autostarted target to
acknowledge the initial event. If the agent is still starting when that check expires, output says
`Queued; delivery pending`; the durable message is delivered later and send still succeeds, so do
not repeat it. See `references/named-agents.md` for routing details.

Editable bundle instructions live only in `agents/<name>/SOUL.md`; bundle `AGENTS.md` files are
not read as a fallback.

For multiple agents, omit `--as` and capture the generated names from launch output; one explicit name cannot be assigned to a multi-agent launch.

`/clear` (and its aliases) ends the hcom session and starts a new one in the same terminal. An
hcom-launched agent takes its own name back on that new session; a session hcom did not launch, or
one whose name is already held by another running instance, gets a fresh generated name — check
`hcom list` after clearing if a name matters.

---

## setup

When this skill is invoked, first run:

```bash
hcom status
```

If hcom status works: run `hcom list`

If hcom list shows "Your name: <name>" where `<name>` is not "(not participating)": congratulations!

If running `hcom status` returns "command not found", install first:
```bash
curl -fsSL https://github.com/orgoj/hcom/releases/latest/download/hcom-installer.sh | sh
```

On Windows, use the PowerShell installer from the same release.

If hcom status shows a list of CLI tools and you are not any of them, run `hcom start` to connect to hcom.

If you've just installed hcom now or if hcom list shows "Your name: (not participating)" and you are a tool in the hcom status list of CLI tools:

Relaunch into hcom properly for automatic message delivery and full hcom functionality. User should exit and run `hcom <tool>`.
Or if you know your sessionID/ses_/thread_name: user can exit and run `hcom r <session-id>` to resume you inside hcom. See `hcom r --help`.

---

## troubleshooting

### "hcom not working"

```bash
hcom status          # check installation
hcom hooks status    # check hooks specifically
hcom relay status    # check cross-device relay
```

Raw nested AI CLIs may inherit `HCOM_PROCESS_ID`; hcom ignores their hooks when
the child CLI differs from the tool bound to that process. Current releases also
repair older cross-tool corruption on the next unambiguous Claude hook.

still broken?
```bash
hcom reset all # backup config/db + reset it
hcom claude          # fresh start
```

still broken after that?
```bash
git clone https://github.com/orgoj/hcom.git
cd hcom
```
Read code and figure out what is going on.

| symptom | diagnosis | fix |
|---------|-----------|-----|
| catalog agent not in `hcom list` | agent stopped or never launched | target it directly; `hcom send` starts it on demand |
| known agent reported as having no active agent | its catalog is not in scope from this directory | work from inside that project; exporting it elsewhere is the project owner's decision |
| message sent but not delivered | check `hcom events --last 5` | verify @mention matches agent name/tag |
| message reaches more than one agent | duplicate base name across tags | target the full `@tag-name` to hit exactly one |
| messages leaking between workflows | no thread isolation | always use `--thread` |
| Codex prompt never reported empty | `hcom term <name>` shows animated glyphs on an empty prompt | set `[tui] whimsy = false` and `animations = false` in `~/.codex/config.toml` |

### "Instance `<name>` already exists"

Preserve evidence before running `hcom list`, because listing may reconcile stale rows:

```bash
sqlite3 ~/.hcom/hcom.db \
  "SELECT name, status, status_time, status_context, pid, session_id
   FROM instances WHERE name = '<name>'"
hcom events --agent <name> --last 20
```

Then check whether the process or terminal still exists. If the instance is dead, run
`hcom list` to trigger normal stale reconciliation; new records use a persisted
process-incarnation identity so PID reuse cannot keep a dead row alive. Retry the launch. Use
`hcom kill <name>` only for a genuinely live managed instance. Do not use
`hcom reset all` for a single-name collision.

Do not treat `status = inactive` as proof that the agent process is dead.
Heartbeat or status timeouts can make a still-running agent appear inactive.
Check the recorded PID with the platform process-liveness check. A live PID
must continue to hold the name. A dead PID may be replaced. If no PID is
available, replace the row only when lifecycle context explicitly proves
termination (`exit:*`) or launch failure (`launch_failed`); otherwise fail
conservatively instead of risking a duplicate agent.

### Message composition and verification patterns

- **Task/Fix:** State the functional goal and constraints, then specify explicit
  verification criteria such as a test command, expected exit code, or local commit hash.
- **Report/Done:** Give the absolute path to the report or diff, a 1-2 sentence summary
  of findings or metrics, and verification proof such as a passing test, diff check,
  or commit hash.
- **Review:** Give the absolute path to the proposal or diff, the core invariants to
  preserve (for example, "without reindexing" or "preserve wire format"), and explicit
  questions for the reviewer.
- **Decision:** Lead with the verdict (for example, "GO with changes:" or "STOP:"),
  followed by numbered mechanical points and a required edge-case test scenario.

**Verifiability (Demonstrability):** Never report completion with a vague assertion
such as "done," "fixed," or "should work." Provide reproducible evidence: a command
output summary, passing test suite, or commit hash.

### intent system

agents follow these rules from their bootstrap:
- `--intent request` -> agent always responds
- `--intent inform` -> agent responds only if useful
- `--intent ack` -> agent does not respond

Choose replies from the received intent, not from conversational politeness:

- Do not send receipt acknowledgements, work-started notices, progress updates,
  status messages, or conversational filler. For a request, work silently and
  wait until the work and verification are complete before sending exactly one
  completed result as `--intent inform --reply-to <request-id>`.
- After that final `hcom send` succeeds, end the turn with a brief terminal-visible
  summary of 1-3 sentences. Say that the result was sent via hcom without repeating
  the full report. Never leave the terminal blank or print a long duplicate conclusion.
- If blocked on missing information, send one concrete question as
  `--intent request --reply-to <request-id>`, then end with the same brief terminal
  summary and continue after the answer.
- Do not reply to `inform` unless it requires a concrete substantive response.
- Reserve `ack` for an explicit protocol that specifically requires a receipt;
  ordinary agent tasks never require it.
- A task you delegated stays with the delegate until they report it. Do not inspect,
  verify or audit the result yourself — reporting on their own work is their job, and a
  second pass duplicates the report that is coming.
- An `[hcom-events]` notice that a target `is idle and has not replied ... yet` is a turn
  boundary, not a refusal. The request stands: keep waiting instead of taking the delegated
  work back. Only `stopped without responding` means no reply is coming.
- For an already-running target, `Sent to:` proves routing and durable enqueue only. For an
  autostarted target, it additionally proves cursor acknowledgement of that initial event during
  the bounded post-start check. Neither proves that the target finished processing or replied.
  `Queued; delivery pending:` means the autostart check expired while the durable event remained
  unread. Report "sent/queued" at this point. Claim end-to-end delivery only
  after a target response or integration-specific completion evidence. For a
  manual-ack bridge, require its completion marker and the listener returning
  to `listening`; recipient cursor or `delivered_to` alone is insufficient. On
  any CLI error, no event was enqueued; correct the command and retry.

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

Use project-local hcom state when the normal hcom directory is unavailable:

```bash
HCOM_DIR=$PWD/.hcom hcom <tool>
```

## files

| what | location |
|------|----------|
| database | `~/.hcom/hcom.db` |
| config | `~/.hcom/config.toml` |
| env | `~/.hcom/env` (preserved if created before first run) |
| logs | `~/.hcom/.tmp/logs/` |
| user scripts | `~/.hcom/scripts/` |

| reference | when to read |
|------|-------------|
| `references/named-agents.md` | defining or launching recurring agents with `hcom agent`, JSON catalogs, start-mode overrides, and per-CLI `tools` profiles |

---

## more info

```bash
hcom --help
hcom <command> --help
hcom run docs --scripts   # script authoring info
```

Github: https://github.com/orgoj/hcom

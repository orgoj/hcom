---
name: hcom-agent-messaging
description: >
  Multi-agent communication for AI coding tools. Agents message, watch,
  and spawn each other across terminals. Use when sending or replying to
  hcom messages, setting up hcom, or troubleshooting.
---

# hcom — multi-agent communication for AI coding tools

## send or reply (do this first, no other commands needed)

Your name and the incoming message's sender and event id are in the message header.

```bash
# reply to message <id> from <sender>
hcom send @<sender> --intent ack --reply-to <id> --name <your-name> -- 'reply text'
# new message that expects an answer
hcom send @<name> --intent request --name <your-name> -- 'message text'
# text with backticks, code or markdown
hcom send @<name> --intent inform --name <your-name> --file /abs/path/msg.md
```

All flags go before `--`; everything after `--` is the message. `@<group>` reaches every catalog agent of that group in your scope and starts stopped ones. Intents: `request` (want reply), `inform` (no reply needed), `ack` (answer, requires `--reply-to`). No need to run `hcom status`, `hcom list` or `--help` before sending; use them only if the send fails.

When the user tells you who to message (an agent or group, with or without `@`), send the user's request to that name right away. Do not research the content first (web, code, transcripts) and do not verify the name with `hcom list` or the catalog; the recipient does the analysis. Look the name up only if the send fails.

AI agents running in separate terminals are isolated. hcom connects them via hooks and a shared database so they can message, watch, and spawn each other in real-time.

Start an agent with `hcom` in front, then prompt normally.

Use hcom to:

- coordinate multi-agent pipelines
- run different AI CLIs as each other's subagents
- avoid copy-pasting

```bash
cargo install --git https://github.com/orgoj/hcom --locked
hcom claude       # or: hcom gemini, hcom codex, hcom opencode, hcom kilo, hcom pi, hcom omp, hcom agy, hcom cursor, hcom kimi, hcom copilot, hcom qoder, hcom grok, hcom hermes
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

**Subscribe** and notify on status changes, file edits, collisions, specific events. React automatically. Scripts can stream matching events as JSON Lines with `hcom events --follow`.

**Spawn**, **fork**, **resume**, **kill** in any terminal emulator or headless. In Herdr, `hcom r` returns an agent to its recorded tab and `hcom f` opens a tab named after the fork; neither inherits your tab. `hcom kill` takes names, `@group`, `tag:<tag>`, `tool:<cli>` (e.g. `tool:agy`) or `SELF` (the calling agent ends itself). Add `--json` to a launch, `hcom r`, `hcom f` or `hcom kill` to get the outcome as one JSON object.

**Idle autokill**: `hcom config -i <name|self> idle_kill <seconds>` (or catalog `"idle_kill"`) kills a local agent (not a remote one or a subagent) that stays idle that long; `0` clears. An agent waiting for a reply to its own `--intent request` message, or with unread messages, is not killed.

**Roaming catalog agent**: `"roaming": true` makes `@<name>` address `<name>_<project>_<hash>` for the sender's Git/project root (`<hash>` = 3-digit number from SHA-256 of the root path, so roots with the same basename get different instances) and start it there when missing.

**Disabled catalog agent**: `"disabled": true` in `agents.json` (entry or `defaults`; a later layer's `false` re-enables) makes hcom treat the agent as undefined: no launch, send autostart, group membership or listing; `hcom agent list --all` shows it marked `disabled`. A running instance keeps receiving messages and can be killed by name.

run `hcom --help` for full command syntax and flags.

---

Works with Claude Code, Gemini CLI, Codex, OpenCode, Kilo Code, Pi, Oh My Pi, Antigravity, Cursor, Kimi, Copilot, Qoder CLI, Grok Build, Hermes Agent, and other tools

---

## setup

Only when setting up hcom or when a send fails, run:

```bash
hcom status
```

If hcom status works: run `hcom list`

If hcom list shows "Your name: <name>" where `<name>` is not "(not participating)": congratulations!

If running `hcom status` returns "command not found", install first:
```bash
cargo install --git https://github.com/orgoj/hcom --locked
```

See the [hcom README](https://github.com/orgoj/hcom#install) for other install options. The `uv` and `brew` packages install upstream hcom without this fork's features.

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

still broken? Report the failing command, its output and the three status
results to the user or the hcom owner agent (`@hcom_dev` where it exists).
Never run `hcom reset` yourself: it replaces the shared database and config
for every agent on the machine, and only the user decides that.


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
| env | `~/.hcom/env` |
| logs | `~/.hcom/.tmp/logs/` |
| user scripts | `~/.hcom/scripts/` |

---

## more info

```bash
hcom --help
hcom <command> --help
hcom run docs --scripts   # script authoring info
```

Github: https://github.com/orgoj/hcom (fork of https://github.com/aannoo/hcom)

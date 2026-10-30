---
id: TASK-67
title: Sandboxed agents via shim + host proxy (nono)
status: Idea
assignee: []
created_date: '2026-10-30 20:53'
labels: []
dependencies: []
type: feature
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Not scheduled; analysis only, implementation later. Needs specification before implementation.

Goal: run selected coding agents (Claude, Codex) in yolo / skip-permissions mode inside a lightweight Landlock sandbox (`nono`), while they stay full hcom members for messaging, without any way to control or impersonate the rest of the fleet.

Background (verified in source): any client with `$HCOM_DIR` access controls the whole fleet (launch/kill/term inject/transcripts/config); `--name` is an unauthenticated DB lookup (`identity::resolve_from_name`); there is no thin client (relay peers are full devices with their own DB and can run launch/kill/term_inject/transcript RPCs). Mounting `~/.hcom` into a sandbox defeats it. Bubblewrap is out; a libkrun microVM is a possible later step, not in scope.

Architecture:
host: hcom PTY wrapper (launch/resume/kill/wake/inject unchanged) -> nono sandbox -> agent CLI; the CLI's hooks call `hcom` = shim -> one unix socket per instance -> host proxy (trusted) -> real hcom binary (exec, no shell). Launch, resume, fork, handoff and `hcom r` stay on the host, so wake/inject TCP ports stay on the host and keep working (without wake, worst case is about 30 s latency: `IDLE_WAIT` in `src/delivery.rs`, hook `poll_loop` in `src/hooks/common.rs`).

Components:
1. Shim (untrusted, inside sandbox): forwards argv + stdin, relays stdout/stderr/exit code exactly; keeps stdin open (`poll` detects closed stdin); supports long calls (Stop `poll` hook up to 86400 s) and many concurrent calls (Claude mod spawns hcom up to 4/s); `send --file` reads the file and sends its content as stdin (max 1 MiB).
2. Proxy (trusted, host): socket -> instance binding fixed at launch.
   - Allowlist: Claude hooks `sessionstart`, `userpromptsubmit`, `pre`, `post`, `poll`, `notify`, `subagent-start`, `subagent-stop`, `sessionend`; Codex hooks `codex-sessionstart`, `codex-userpromptsubmit`, `codex-pretooluse`, `codex-posttooluse`, `codex-stop`, `codex-interrupt`; plus `send` and `listen`. Everything else denied, explicitly `start`, `--as`, `transcript`, `term`, `kill`, launches, `config`, `reset`, `relay`, `events`, `list`.
   - Forces `--name`; sets `HCOM_PROCESS_ID`, `HCOM_INSTANCE_NAME`, `HCOM_DIR` from the binding; drops all guest identity env.
   - Checks that `session_id` in the hook JSON belongs to the bound instance; `subagent-*` only on child rows `<self>_*`; `sessionstart` must not bind or rebind another name.
   - `transcript_path` and `cwd` from the hook JSON must lie in the instance's own transcript/project dir (hcom opens `transcript_path` for the session ownership check).
   - Per-instance recipient allowlist; no messages to agents with server/production access.
3. nono profile: write only the project dir; read only the agent's own transcript dir; no `~/.ssh`, `~/.hcom`, other tokens, browser profiles; egress allowlist (model APIs, npm, PyPI, crates, GitHub read-only); `.git/hooks`, `.git/config`, `.vscode`, `.idea` read-only.
4. Launch integration: hcom launches `nono … <cli>` as the PTY child for instances marked sandboxed; the guest CLI hook config points to the shim.

Residual risks (document, not solved here): the agent's own OAuth/API token is readable in the sandbox and can leave via the allowed API host (check nono phantom-token support); workspace taint on the host (`.envrc`, `mise.toml`, package scripts, `build.rs`, symlinks out of the project); messages from sandboxed agents are untrusted data for recipients.

Draft acceptance criteria (to finalize during specification):
- A sandboxed Claude and Codex instance send and receive messages; wake and inject work.
- Tests: the proxy rejects every denied subcommand, a forged `--name`/env/session_id, a `sessionstart` rebinding attempt, `subagent-*` outside `<self>_*`, foreign `transcript_path`/`cwd`, and a recipient outside the allowlist.
- From inside the sandbox `~/.hcom`, `~/.ssh` and other projects are unreadable and non-allowlisted egress fails.
- The proposal gets a security review before implementation.

Open decisions: owner of shim + proxy (this repo vs a separate tool); nono version vetting and installation.
<!-- SECTION:DESCRIPTION:END -->

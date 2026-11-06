---
id: TASK-67
title: 'Sandboxed agents: bwrap sandbox + hcom shim/proxy'
status: To Do
assignee: []
created_date: 2026-10-30 20:53
updated_date: '2026-11-06T21:36:00Z'
labels: []
dependencies: []
type: feature
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: run selected Claude, Codex and agy agents in yolo mode (skip permissions, no Dippy) inside a bwrap sandbox on the user's subscriptions, as full hcom members (hooks, send, listen, wake, inject), without any way to read other agents' data, control or impersonate the fleet, or escape to the host.

Why: any client with `$HCOM_DIR` access controls the whole fleet (launch/kill/term inject/transcripts/config); `--name` is an unauthenticated DB lookup (`identity::resolve_from_name`); notify/inject TCP ports on 127.0.0.1 are unauthenticated (`src/pty/inject.rs`, `src/notify/server.rs`) and allow PTY injection into any agent. So `~/.hcom` and host loopback must never be reachable from a sandbox.

Design (agreed with agent bin, user decisions 2026-11-06; bin analysis: /home/michael/bin/tmp/sandbox-bin-reply.md, sandbox-bin-reply-2.md):

Runtime: bubblewrap from apt (user decision). Landlock-only tools (nono, landrun) cannot block connect() to pathname unix sockets before kernel 7.1 (`/run/user/1000/bus` -> `systemd-run --user`, wezterm `cli send-text`, gpg-agent) and have no pid namespace; Claude Code srt and Codex also use bwrap. Re-evaluate nono at kernel >= 7.1 or when phantom tokens are required.

Chain: host hcom PTY wrapper (launch/resume/fork/kill/wake/inject unchanged, stays on host) -> `~/bin/agent-sandbox` (owned by bin) -> CLI. Inside the sandbox `hcom` is the hcom binary in shim mode (bound ro over the real path, so the generated hook commands keep working); it forwards argv, stdin, stdout/stderr and exit code over one unix socket per instance to a proxy in the host PTY wrapper, which runs the real hcom (exec, no shell) with forced identity.

Ownership:
- hcom (this task): shim mode, proxy, per-instance socket, catalog `sandbox` setting, launch integration for clean start/resume/fork, generating the private CLI config dirs and hook configs (hcom shim hooks only, no third-party hooks, no Dippy, no MCP unless listed).
- bin (`~/bin` backlog): `agent-sandbox --project DIR --config-dir DIR --socket PATH --net MODE [--ro P]... [--rw P]... -- <cli> args`, the filesystem profile, network modes, escape tests, user docs for one-time token setup. Exit non-zero before the CLI starts on any setup failure; never run with fewer restrictions than requested.

Shim (untrusted side): keeps stdin open (`poll` detects closed stdin); supports long calls (Stop `poll` hook up to 86400 s) and many concurrent calls (Claude mod spawns hcom up to 4/s); `send --file` sends the file content as stdin (max 1 MiB, `MAX_MESSAGE_SIZE`). Without the socket it fails; it never touches a DB.

Proxy (trusted): socket -> instance binding fixed at launch. Allowlist only: Claude hooks `sessionstart`, `userpromptsubmit`, `pre`, `post`, `poll`, `notify`, `subagent-start`, `subagent-stop`, `sessionend`; Codex hooks `codex-sessionstart`, `codex-userpromptsubmit`, `codex-pretooluse`, `codex-posttooluse`, `codex-stop`, `codex-interrupt`; agy hooks `gemini-sessionstart`, `gemini-beforeagent`, `gemini-afteragent`, `gemini-beforetool`, `gemini-aftertool`, `gemini-sessionend`; plus `send` and `listen`. Everything else is denied (explicitly `start`, `--as`, `transcript`, `term`, `kill`, launches, `config`, `reset`, `relay`, `events`, `list`). Forces `--name`; sets `HCOM_PROCESS_ID`, `HCOM_INSTANCE_NAME`, `HCOM_DIR` from the binding and drops all guest env. Checks that the hook JSON `session_id` belongs to the bound instance; `subagent-*` only on child rows `<self>_*`; `sessionstart` never binds or rebinds another name; `transcript_path` and `cwd` must lie under the instance's private config dir / project dir (prefix check from the binding). Recipient deny list: agents marked with server/production access (phase 1: one global deny list, not per instance).

Private state per instance: host dir `~/.local/share/hcom-sandbox/<instance>/` with `claude/` (`CLAUDE_CONFIG_DIR`), `codex/` (`CODEX_HOME`), `gemini/` (bound as `~/.gemini`), caches; kept across launches so logins persist and resume finds the transcripts.

Credentials (user decision: separate grants, never shared host credential files; bind-mounting `.credentials.json`/`auth.json` breaks on refresh with EBUSY and refresh tokens rotate single-use): Claude `claude setup-token` (subscription, 1 year, in `pass`), passed as `CLAUDE_CODE_OAUTH_TOKEN`; Codex own `codex login --device-auth` per instance dir; agy own login inside its private dir (if agy cannot log in without the host browser: stop and ask the user). Updaters/telemetry off inside (`DISABLE_AUTOUPDATER=1`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, codex `check_for_update_on_startup = false`).

Network (user decision: per agent): `none`, `ports:P1,P2` (only listed host ports forwarded via unix sockets; combinable with the others), `lan`, `internet` (RFC1918/link-local/CGNAT/ULA dropped), `full`. Own netns via slirp4netns `--disable-host-loopback` + nft in the holder netns; host loopback is never reachable, and this computer's own LAN address (192.168.68.4) is dropped in every mode (user decision). hcom never passes its own notify/inject ports to `ports:`.

Out of scope: hostname (L7) egress allowlist; phantom-token proxy; nono; libkrun microVM; Dippy inside the sandbox; non-Linux platforms.

Residual risks (documented, not solved): the agent can use and exfiltrate its own scoped token via the API host; workspace taint on the host (`.envrc`, `mise.toml`, package scripts, `build.rs`, symlinks out of the project; bin binds the known ones ro); messages from sandboxed agents are untrusted data for recipients.

Dependencies: bin's `agent-sandbox` prototype with escape tests (separate task in `~/bin`); one-time user steps: `claude setup-token` into pass, codex and agy logins per instance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A catalog agent with a `sandbox` setting (network mode, optional ports, extra ro/rw paths) launches its CLI through the configured sandbox wrapper; agents without it launch exactly as before (dry-run test for both).
- [ ] #2 A sandboxed Claude, Codex and agy instance each receives and sends messages, and wake and `hcom term inject` reach it, on clean start, resume and fork (manual run per CLI, results recorded in notes).
- [ ] #3 The proxy rejects with a non-zero exit and an error every non-allowlisted subcommand, a forged `--name`/`--as`/identity env, a hook `session_id` of another instance, a `sessionstart` that binds or rebinds another name, `subagent-*` outside `<self>_*`, a `transcript_path` or `cwd` outside the instance dirs, and a `send` to a deny-listed recipient (one unit test per case).
- [ ] #4 The shim relays stdout, stderr, exit code and stdin exactly, keeps a long `poll` call open, serves concurrent calls, and sends `send --file` content up to 1 MiB (tests).
- [ ] #5 Generated private config dirs contain only the hcom shim hooks (no Dippy, no third-party hooks) and no host credential file; Claude runs on `CLAUDE_CODE_OAUTH_TOKEN`, Codex and agy on their own logins in the instance dir.
- [ ] #6 From inside a sandbox: `~/.hcom`, `~/.ssh`, `~/.claude`, `~/.codex`, other projects, `/run/user/1000/bus` and the wezterm socket are unreachable, host 127.0.0.1 (including the hcom inject/notify ports) and 192.168.68.4 are unreachable in every network mode, and each mode allows exactly its traffic (bin escape test suite, run once against the hcom launch path).
- [ ] #7 User docs (README, docs/, built-in help, hcom-agent-messaging skill) describe the `sandbox` catalog setting, network modes, the one-time token/login steps and the residual risks.
- [ ] #8 The design and diff pass a security review by agent_review before release.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. bin: prototype `~/bin/agent-sandbox` with escape tests (own task in ~/bin); agree its CLI contract with hcom_dev before hcom step 4.
2. hcom: shim mode + proxy with allowlist and identity checks, unit tests (no launch changes yet).
3. hcom: catalog `sandbox` setting, private instance dirs, generated hook configs, dry-run tests.
4. hcom: launch integration (Claude forced to PTY path; clean start, resume, fork), wrapper call with socket and network mode.
5. Manual end-to-end per CLI (Claude, Codex, agy) incl. one-time user logins; run bin escape tests through the hcom launch path.
6. Docs and help; security review; release.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Spec finding: Claude defaults to no PTY wrapper (`uses_pty_default: false`, src/integration_spec.rs); a sandboxed Claude always launches on the PTY path (as `ClaudePty`), because the proxy lives in the host PTY wrapper. hcom already knows `config_dir_env` (`CLAUDE_CONFIG_DIR`) per tool; reuse it for the private config dir.
<!-- SECTION:NOTES:END -->

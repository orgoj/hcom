---
id: TASK-67
title: 'Sandboxed agents: bwrap sandbox + hcom shim/proxy'
status: To Do
assignee: []
created_date: 2026-10-30 20:53
updated_date: '2026-11-06T21:50:00Z'
labels: []
dependencies: []
type: feature
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: run selected Claude, Codex and agy agents in yolo mode (skip permissions, no Dippy) inside a bwrap sandbox on the user's subscriptions, as full hcom members (hooks, send, listen, wake, inject), without any way to read other agents' data, control or impersonate the fleet, or escape to the host. Everything ships in this repository: anyone with a git clone on Linux gets the feature with only documented system packages (bwrap, slirp4netns, nft); no code or profile outside the repo.

Why: any client with `$HCOM_DIR` access controls the whole fleet (launch/kill/term inject/transcripts/config); `--name` is an unauthenticated DB lookup (`identity::resolve_from_name`); notify/inject TCP ports on 127.0.0.1 are unauthenticated (`src/pty/inject.rs`, `src/notify/server.rs`) and allow PTY injection into any agent. So `~/.hcom` and host loopback must never be reachable from a sandbox.

Design (analysed with agent bin, user decisions 2026-11-06):

Runtime: bubblewrap (user decision). Landlock-only tools (nono, landrun) cannot block connect() to pathname unix sockets before kernel 7.1 (`$XDG_RUNTIME_DIR/bus` -> `systemd-run --user`, terminal-multiplexer sockets such as wezterm `cli send-text`, gpg-agent) and have no pid namespace; Claude Code srt and Codex also use bwrap. Re-evaluate Landlock at kernel >= 7.1 or when phantom tokens are required. Requirement documented: unprivileged user namespaces enabled (Ubuntu `kernel.apparmor_restrict_unprivileged_userns=0` or an AppArmor profile for bwrap); hcom fails with a clear error otherwise.

Chain: host hcom PTY wrapper (launch/resume/fork/kill/wake/inject unchanged, stays on host) -> hcom builds the bwrap/netns invocation itself (Rust, no shell script) -> CLI. Inside the sandbox `hcom` is the same hcom binary in shim mode (bound ro over the real path, so generated hook commands keep working); it forwards argv, stdin, stdout/stderr and exit code over one unix socket per instance to a proxy in the host PTY wrapper, which runs the real hcom (exec, no shell) with forced identity.

All in hcom (this task): sandbox launcher (filesystem profile, namespaces, network modes), shim mode, proxy, per-instance socket, catalog `sandbox` setting, launch integration for clean start/resume/fork, private CLI config dirs and generated hook configs (hcom shim hooks only, no third-party hooks, no Dippy, no MCP unless listed), escape test suite. Machine-specific values (extra ro/rw paths such as a tool manager dir, extra denied addresses, token source) are configuration, never hardcoded. Agent bin only reviews the profile against this machine and sets the local configuration.

Filesystem profile: ro system dirs (`/usr`, `/etc`, `/lib*`, `/bin`, `/sbin`); ro the CLI binary location resolved from PATH at launch (Claude `~/.local/share/claude/versions/<ver>`, Codex binary dir inside `~/.codex/packages`, agy binary + its helper dir); ro `~/.gitconfig` and configured extra ro paths (e.g. tool managers); rw the project dir with `.git/hooks`, `.git/config`, `.envrc`, `mise.toml`, `.mise.toml`, `.vscode`, `.idea` re-bound ro when present; rw the private instance dir and private caches (`~/.cache`, npm/uv/cargo caches; host caches at most ro, never rw: cache poisoning of host builds); `/tmp` tmpfs, minimal `/dev`, new `/proc`, pid namespace, `--clearenv` with an explicit env allowlist, `--die-with-parent`, no `--new-session`. Never bound: `$HCOM_DIR`, `~/.ssh`, `~/.gnupg`, password stores, `~/.config`, `~/.claude`, `~/.claude.json`, `~/.codex` (except the binary dir), `~/.gemini` (except binary helpers), `$XDG_RUNTIME_DIR`, `/tmp/.X11-unix`, other projects.

Shim (untrusted side): keeps stdin open (`poll` detects closed stdin); supports long calls (Stop `poll` hook up to 86400 s) and many concurrent calls (Claude mod spawns hcom up to 4/s); `send --file` sends the file content as stdin (max 1 MiB, `MAX_MESSAGE_SIZE`). Without the socket it fails; it never touches a DB.

Proxy (trusted): socket -> instance binding fixed at launch. Allowlist only: Claude hooks `sessionstart`, `userpromptsubmit`, `pre`, `post`, `poll`, `notify`, `subagent-start`, `subagent-stop`, `sessionend`; Codex hooks `codex-sessionstart`, `codex-userpromptsubmit`, `codex-pretooluse`, `codex-posttooluse`, `codex-stop`, `codex-interrupt`; agy hooks `gemini-sessionstart`, `gemini-beforeagent`, `gemini-afteragent`, `gemini-beforetool`, `gemini-aftertool`, `gemini-sessionend`; plus `send` and `listen`. Everything else is denied (explicitly `start`, `--as`, `transcript`, `term`, `kill`, launches, `config`, `reset`, `relay`, `events`, `list`). Forces `--name`; sets `HCOM_PROCESS_ID`, `HCOM_INSTANCE_NAME`, `HCOM_DIR` from the binding and drops all guest env. Checks that the hook JSON `session_id` belongs to the bound instance; `subagent-*` only on child rows `<self>_*`; `sessionstart` never binds or rebinds another name; `transcript_path` and `cwd` must lie under the instance's private dir / project dir (prefix check from the binding). Recipient deny list from configuration (phase 1: one global list, not per instance).

Private state per instance: `<hcom data dir>/sandbox/<instance>/` is wrong (it sits in `$HCOM_DIR`, which is never bound); use `$XDG_DATA_HOME/hcom-sandbox/<instance>/` with `claude/` (`CLAUDE_CONFIG_DIR`), `codex/` (`CODEX_HOME`), `gemini/` (bound as `~/.gemini`), caches; kept across launches so logins persist and resume finds the transcripts.

Credentials (user decision: separate grants, never shared host credential files; bind-mounting `.credentials.json`/`auth.json` breaks on refresh with EBUSY and refresh tokens rotate single-use): Claude long-lived token from `claude setup-token` (subscription), passed as `CLAUDE_CODE_OAUTH_TOKEN` from a configured source (env var or a command such as `pass show ...`; hcom stores no token); Codex own `codex login --device-auth` in the instance dir; agy own login in its private dir (if agy cannot log in without the host browser: stop and ask the user). Updaters/telemetry off inside (`DISABLE_AUTOUPDATER=1`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, codex `check_for_update_on_startup = false`).

Network (user decision: per agent): `none`, `ports:P1,P2` (only listed host ports forwarded into the sandbox loopback via unix sockets, forwarder in hcom; combinable with the others), `lan`, `internet` (RFC1918/link-local/CGNAT/ULA dropped), `full`. Own netns via slirp4netns `--disable-host-loopback` + nft rules loaded in the holder netns before the CLI starts; host loopback is never reachable, and every address of the host's own interfaces (detected at launch, plus configured extras) is dropped in every mode (user decision). hcom never forwards its own notify/inject ports. Any setup failure aborts before the CLI starts; never run with fewer restrictions than requested.

Out of scope: hostname (L7) egress allowlist; phantom-token proxy; nono/Landlock; libkrun microVM; Dippy inside the sandbox; non-Linux platforms.

Residual risks (documented, not solved): the agent can use and exfiltrate its own scoped token via the API host; workspace taint on the host (`.envrc`, `mise.toml`, package scripts, `build.rs`, symlinks out of the project; the known files are bound ro); messages from sandboxed agents are untrusted data for recipients.

One-time user steps (documented): install bwrap/slirp4netns/nft, create the Claude token, log in Codex and agy per instance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A sandboxed Claude, Codex and agy instance each receives and sends messages, and wake and `hcom term inject` reach it, on clean start, resume and fork (manual run per CLI, results recorded in notes).
- [ ] #2 The proxy rejects with a non-zero exit and an error every non-allowlisted subcommand, a forged `--name`/`--as`/identity env, a hook `session_id` of another instance, a `sessionstart` that binds or rebinds another name, `subagent-*` outside `<self>_*`, a `transcript_path` or `cwd` outside the instance dirs, and a `send` to a deny-listed recipient (one unit test per case).
- [ ] #3 The shim relays stdout, stderr, exit code and stdin exactly, keeps a long `poll` call open, serves concurrent calls, and sends `send --file` content up to 1 MiB (tests).
- [ ] #4 Generated private config dirs contain only the hcom shim hooks (no Dippy, no third-party hooks) and no host credential file; Claude runs on `CLAUDE_CODE_OAUTH_TOKEN`, Codex and agy on their own logins in the instance dir.
- [ ] #5 User docs (README, docs/, built-in help, hcom-agent-messaging skill) describe the `sandbox` catalog setting, network modes, the one-time token/login steps and the residual risks.
- [ ] #6 The design and diff pass a security review by agent_review before release.
- [ ] #7 A catalog agent with a `sandbox` setting (network mode, optional ports, extra ro/rw paths) launches its CLI inside the hcom-built bwrap sandbox; agents without it launch exactly as before (dry-run test for both).
- [ ] #8 Repo tests (skipped with a clear message when bwrap, userns or slirp4netns are unavailable) prove from inside a sandbox: `$HCOM_DIR`, `~/.ssh`, `~/.claude`, `~/.codex`, other projects and `$XDG_RUNTIME_DIR` sockets are unreachable; host 127.0.0.1 (including hcom inject/notify ports) and the host interface addresses are unreachable in every network mode; each mode allows exactly its traffic.
- [ ] #9 No sandbox code, profile or script lives outside this repository; machine-specific values come only from documented configuration, and a fresh clone on Linux with the documented packages runs the feature.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Sandbox launcher in hcom (bwrap argv builder, netns holder with slirp4netns + nft, ports forwarder) with the escape test suite as cli_smoke/integration tests that skip when bwrap or userns is unavailable.
2. Shim mode + proxy with allowlist and identity checks, unit tests.
3. Catalog `sandbox` setting and hcom config for machine-specific values, private instance dirs, generated hook configs, dry-run tests.
4. Launch integration (Claude forced to PTY path; clean start, resume, fork).
5. Manual end-to-end per CLI (Claude, Codex, agy) incl. one-time logins; agent bin reviews the profile on this machine and sets local config.
6. Docs and help; security review; release.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Spec finding: Claude defaults to no PTY wrapper (`uses_pty_default: false`, src/integration_spec.rs); a sandboxed Claude always launches on the PTY path (as `ClaudePty`), because the proxy lives in the host PTY wrapper. hcom already knows `config_dir_env` (`CLAUDE_CONFIG_DIR`) per tool; reuse it for the private config dir.
<!-- SECTION:NOTES:END -->

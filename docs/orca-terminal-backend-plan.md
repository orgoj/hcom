# Local Orca Terminal Backend Plan

## Goal

Add Orca as a first-class local terminal backend for hcom so catalog agents can
run in Orca terminals and remain visible and controllable from Orca's desktop,
browser, and mobile clients.

The first version is local-only. Remote Orca servers, SSH execution hosts, and
cross-host hcom state are explicitly out of scope.

## Ownership and coordination

- The hcom implementation belongs in this repository.
- Any required Orca change belongs in `/home/michael/projects/orca` and must be
  implemented by a dedicated `orca_dev` agent.
- Before implementation starts, ask `agent_coach` only to create and configure
  the `orca_dev` agent as the owner of that repository.
- As its first repository task, `orca_dev` must fork the canonical Orca
  repository, configure `origin` as its writable fork and `upstream` as the
  canonical repository, and create a dedicated development branch from the
  current upstream base. The branch must contain only the minimal changes
  required by this integration and remain suitable for submission as an
  upstream pull request.
- After `orca_dev` exists, communicate and coordinate directly with it for the
  Orca proposal, implementation, verification, commit, release, and any later
  cross-repository contract questions. Do not route normal implementation work
  through `agent_coach`.
- Do not edit Orca from the hcom agent or copy Orca internals into hcom.
- Land and release the Orca side first when hcom depends on a new Orca CLI or
  RPC capability. The hcom implementation must retain an actionable version or
  capability error for older Orca installations.

## Why Orca needs a dedicated adapter

Orca already exposes the required terminal lifecycle through its CLI:

| Requirement | Orca command |
|---|---|
| Create a terminal | `orca terminal create --worktree … --command … --json` |
| Obtain a handle | `result.terminal.handle` |
| Inspect metadata | `orca terminal show --terminal … --json` |
| Read output or screen | `orca terminal read --terminal … [--screen] --json` |
| Send input | `orca terminal send --terminal … --text … --enter --json` |
| Wait for idle or exit | `orca terminal wait --terminal … --for … --json` |
| Rename | `orca terminal rename --terminal … --title … --json` |
| Focus | `orca terminal switch --terminal … --json` |
| Close the exact terminal | `orca terminal close --terminal … --json` |

This does not fit a generic `TerminalPreset`. hcom must parse structured JSON,
store a runtime-scoped terminal handle, distinguish transport failure from a
rejected operation, and close the exact terminal later. Implement an explicit
Orca path alongside the existing native Herdr path in `src/terminal.rs`.

Orca is an execution/UI backend only. hcom remains authoritative for catalog
resolution, hcom identity, messages, hooks, delivery, and participant lifecycle.
Do not create a parallel Orca orchestration Run or Dispatch for ordinary hcom
catalog launches.

## Scope

### Included

- Built-in `orca` terminal preset on macOS, Linux, and Windows where the Orca
  CLI is available.
- Local Orca desktop runtime or local `orca serve` runtime.
- Catalog launch, direct launch, autostart-on-send, nested launch, clean start,
  and session switch paths already exercised by hcom's launch machinery.
- Normal hcom runner scripts and their existing environment/instruction setup.
- Capture and persistence of the Orca terminal handle.
- Exact close on `hcom kill`.
- Clear diagnostics when the CLI, runtime, workspace, or required capability is
  unavailable.
- User-facing help, README, agent catalog documentation, skill reference, and
  smoke coverage required by the repository change matrix.

### Excluded

- Remote Orca Server selection (`--environment`, pairing codes, or
  `ORCA_ENVIRONMENT`).
- Orca SSH execution hosts.
- Web/mobile implementation work; those clients consume the local Orca runtime
  automatically.
- Replacing hcom messaging with Orca orchestration.
- Automatic worktree creation or deletion.
- hcom-driven Orca installation, account setup, pairing, or runtime upgrades.
- Resume-specific behavior beyond proving that the existing hcom launch path
  does not regress. The current user does not use hcom resume.

## Static Orca repository findings

Reviewed against the local Orca checkout on 2026-09-22. The repository already
contains almost all runtime support needed by hcom:

- `src/shared/rpc-contract/terminal-unary-params.ts` defines
  `TerminalCreateParams` with structured `env`, `envToDelete`, `launchAgent`,
  `rendererBacked`, `presentation`, `command`, `title`, and workspace fields.
- `src/main/runtime/rpc/methods/terminal/terminal-lifecycle-methods.ts` forwards
  those fields to `runtime.createTerminal`.
- `src/main/runtime/orca-runtime-create-terminal.ts` can create the PTY in a
  headless/background runtime, publish it as a durable terminal surface, retain
  its title and agent hint, and return an opaque handle plus PTY/process and
  execution-host metadata.
- When `rendererBacked` is requested but no authoritative desktop window exists,
  the runtime safely falls back to the background/headless PTY path. The same
  contract therefore works for a local desktop app and local `orca serve`.
- Folder workspaces and `path:<absolute-path>` selectors are first-class in the
  terminal creation path; no Git worktree is required.
- `src/main/runtime/orca-runtime-stop-explicitly-closed-tab-ptys.ts` waits for
  the addressed PTY to stop and returns `ptyKilled`, `ptyStopVerdict`, and an
  optional reason. It does not silently turn an unverified kill into success.
- `src/cli/handlers/terminal-close.ts` converts `live` and `unverifiable` stop
  verdicts into a non-zero CLI result while preserving the structured receipt.
- Terminal handles are incarnation-fenced. Stale or cross-incarnation handles
  raise `terminal_handle_stale`; their absence is not evidence that the process
  exited. Runtime-owned handles can be re-linked to a restored PTY when Orca has
  authoritative matching state.

The missing surface is the public CLI, not the runtime:

- `src/cli/handlers/terminal.ts` exposes `--command`, `--shell`, `--title`, and
  `--focus`, but does not expose `launchAgent`, structured environment fields,
  or an explicit renderer-backed intent.
- `src/cli/codex-command-classification.ts` enables renderer-backed mode only
  when the literal command begins with an interactive `codex` or `claude`
  executable (after a limited shell/env prefix).
- hcom launches its generated runner script, which later execs `hcom pty` and
  the agent. That command does not pass Orca's literal-agent classifier, so it
  loses the same explicit agent/renderer intent a direct Orca agent launch gets.
- `terminal.createAgentSession` and `agent.launch` are not substitutes: they
  construct and own the agent command themselves, while hcom must retain its
  runner, environment, hook, and lifecycle authority.

Static decision: **a small Orca CLI extension is required for a first-class
integration; no new Orca runtime/RPC primitive is required.** Do not bypass the
CLI by reading Orca runtime metadata and speaking its private socket protocol
from hcom.

## Preconditions and runtime discovery gate

Before writing production code, add an isolated CLI smoke or adapter-level
fixture that confirms the remaining runtime-dependent behavior against the
installed Orca CLI:

1. Does `orca terminal create --worktree path:<cwd> --command <runner> --json`
   create a terminal visible in the desktop UI when `<runner>` ultimately execs
   `hcom pty <tool>` rather than naming the agent CLI directly?
2. Does the created terminal receive a stable `result.terminal.handle`?
3. Does the hcom runner bind through existing hooks exactly like another
   terminal backend when launched with the new explicit agent intent?
4. Does the explicit agent intent produce the same interactive rendering as a
   direct `codex` or `claude` terminal command?
5. Does the CLI expose the runtime's exact close receipt and non-zero outcome
   unchanged for confirmed-live and unverifiable PTYs?
6. After a local Orca restart, what structured result is returned for the old
   handle, and can hcom safely treat that handle as stale without inferring that
   the agent process died?

Use an isolated hcom data directory and a disposable Orca workspace/terminal.
Never point a smoke fixture at live `~/.hcom`. Do not automate the visible Orca
window or steal focus.

These checks validate the agreed contract; the repository analysis already
establishes that the current public CLI needs the extension below.

## Required Orca CLI change

Add one generic explicit-agent option to `orca terminal create`. Proposed
public shape:

```text
orca terminal create
  --worktree path:<cwd>
  --command <wrapper-command>
  --interactive-agent <agent-kind>
  --title <title>
  --json
```

The final flag name belongs to Orca's design review, but its semantics must be:

- validate the value with Orca's existing TUI-agent registry;
- forward it as the existing `launchAgent` RPC field;
- request the existing `rendererBacked` behavior for a local desktop runtime;
- retain the existing safe headless fallback when no authoritative renderer is
  available;
- keep `--command` authoritative, rather than replacing it with Orca's standard
  agent launcher;
- remain additive and backward-compatible;
- advertise a machine-readable runtime/CLI capability so hcom can fail closed
  with an actionable upgrade message.

Expected Orca files are limited primarily to:

- `src/cli/specs/core.ts` for the flag/help contract;
- `src/cli/handlers/terminal.ts` for validation and RPC mapping;
- `src/cli/index-terminal-commands.test.ts` and command/help parity tests;
- `src/shared/protocol-version.ts` only if capability negotiation requires a
  new identifier.

The runtime RPC schema already accepts the required fields. Avoid changing
`terminal.create`, `createTerminal`, or PTY ownership unless `orca_dev` finds a
failing contract test that demonstrates a real gap.

The hcom runner script already embeds the required `HCOM_*` environment in its
platform-specific launch script. Do not add a public Orca `--env` flag for this
integration unless cross-platform tests prove the runner cannot preserve it.
If such a flag becomes necessary, values must use the existing structured `env`
RPC map, never shell interpolation.

Do not add an hcom-specific launch endpoint to Orca. The contract should remain
generic enough for any supervised wrapper that needs a visible interactive
terminal.

Any Orca proposal and implementation must follow Orca's own `AGENTS.md`, remote
wire compatibility rules, and cross-platform process-launch abstractions. The
future `orca_dev` task must include tests for native Windows, Unix, and an older
runtime that lacks the new optional field.

## Proposed hcom design

### 1. Built-in preset and availability

- Add `orca` to the built-in terminal preset catalog.
- Resolve the platform executable without guessing:
  - use `orca` where installed;
  - on Linux support the packaged `orca-ide` executable according to Orca's
    documented naming;
  - avoid shell aliases and interactive shell functions.
- Availability requires both the executable and a reachable local runtime.
- Do not auto-start Orca in the first version. Return a concise instruction to
  run the desktop app, `orca open`, or local `orca serve`.
- Do not inherit or honor remote selectors in the first version. If
  `ORCA_ENVIRONMENT`, `ORCA_PAIRING_CODE`, or `ORCA_REMOTE_PAIRING` is present,
  fail closed with a local-only explanation rather than launching remotely by
  accident.

### 2. Workspace resolution

- Use the canonical hcom launch cwd as `path:<absolute-cwd>`.
- Ask Orca to resolve that path to an existing folder/worktree workspace.
- Do not register repositories, create worktrees, or silently fall back to an
  active Orca workspace.
- When the path is not registered, surface Orca's structured error and tell the
  user to add/import that folder in Orca.

This preserves the project's catalog isolation and prevents an agent from being
launched in whichever Orca workspace happens to be focused.

### 3. Launch

- Reuse hcom's existing platform-specific runner script. It already carries
  `HCOM_DIR`, `HCOM_PROCESS_ID`, `HCOM_INSTANCE_NAME`, catalog context, tool
  arguments, system instructions, and cwd handling.
- Invoke Orca with argv, never through a composed shell:

  ```text
  orca terminal create
    --worktree path:<canonical-cwd>
    --title <instance-name>
    --command <platform-specific-runner-command>
    --interactive-agent <agent-kind>
    --json
  ```

- Keep the default unfocused/background presentation so agent launches do not
  steal the user's desktop focus.
- Parse one complete JSON response and require a non-empty terminal handle.
- Pass the agreed explicit-agent CLI flag for the launched hcom tool. Do not
  rely on Orca recognizing the generated runner command text.
- Treat malformed JSON, missing handle, rejected creation, and unreachable
  runtime as distinct failures.
- If creation succeeds but validation fails afterward, close the returned
  terminal best-effort before reporting failure so no empty terminal is left in
  Orca.

### 4. Handle persistence

- Extend terminal-output normalization with a narrow Orca response parser; do
  not reuse the Herdr JSON shape parser.
- Store the Orca terminal handle through the existing terminal-id sidecar path
  keyed by `HCOM_PROCESS_ID`.
- Record that the handle belongs to the `orca` preset so later cleanup never
  sends it to another terminal CLI.
- Treat handles as runtime-scoped opaque strings. Never parse or synthesize
  them.
- A stale handle is loss of terminal authority, not proof that the agent exited.
  Preserve hcom's process/hook evidence ordering for lifecycle decisions.

### 5. Close and cleanup

- Map the built-in Orca close operation to:

  ```text
  orca terminal close --terminal <opaque-handle> --json
  ```

- Require a structured success response before reporting that terminal cleanup
  succeeded.
- If Orca is unreachable or the handle belongs to an old runtime, report
  cleanup as unverifiable and continue hcom's existing process-backed kill
  checks. Never claim success from absence alone.
- Preserve current distinctions between `stop`, `kill`, and natural tool exit;
  the terminal adapter must not invent hcom lifecycle events.

### 6. Status and delivery boundaries

- Existing hcom hooks and delivery remain authoritative.
- Do not use `terminal read --screen` as the primary agent status source.
- Screen reads may be added later as diagnostic evidence, subject to the same
  cursor/style/live-layout safeguards used by existing PTY parsing.
- Do not route normal hcom messages through `orca terminal send`. It is reserved
  for explicit recovery or future adapter diagnostics; normal delivery must
  keep its current hook/PTY semantics.

## Configuration and user-facing behavior

The intended configuration is:

```toml
terminal = "orca"
```

Catalog agents continue to use their existing `terminal`, `session`, and
`window` fields. For the first version:

- `terminal = "orca"` selects Orca;
- `session` and `window` are ignored with the same explicit warning policy used
  by unsupported non-tmux presets;
- the Orca tab title is the stable hcom instance name;
- every agent receives its own Orca terminal tab;
- launch remains unfocused unless an existing hcom command explicitly requests
  current-terminal behavior, which Orca should reject rather than emulate in
  the first version.

Update all user-facing surfaces in the same hcom change:

- built-in terminal/config help;
- `README.md` terminal support list;
- `docs/agent.md` catalog placement behavior;
- relevant `skills/hcom-agent-messaging/` references;
- practical help/smoke assertions.

## Test plan

### Unit tests

- Orca executable resolution (`orca` and Linux `orca-ide`).
- JSON success parsing and exact handle extraction.
- JSON error, malformed response, empty handle, and incompatible capability.
- Explicit agent intent maps the hcom tool to Orca's supported TUI-agent value.
- Unsupported hcom tools fail or deliberately omit the hint according to the
  agreed Orca contract; never mislabel one agent as another.
- Remote-selector environment rejection.
- Correct argv construction for paths and titles containing spaces and Unicode.
- No shell interpolation of cwd, title, runner path, or handle.
- Preset-specific terminal handle persistence.
- Exact close argv and structured-result validation.
- Stale handle and runtime-unavailable outcomes remain unverifiable.

### `tests/cli_smoke.rs`

Use `Hcom::new()` and a fake `orca`/`orca-ide` executable in the isolated PATH.
The fake must capture argv and return complete JSON fixtures. Cover:

- configured default `terminal=orca`;
- per-agent `terminal: orca` override;
- direct launch and catalog launch;
- autostart-on-send launch path;
- nested child launch with no remote-selector leakage;
- `--dry-run` output without creating a terminal;
- launch failure before instance readiness;
- successful handle capture followed by exact `hcom kill` cleanup;
- failure after create closes the half-created terminal;
- spaces/Unicode in cwd and agent name;
- missing workspace and unreachable runtime diagnostics;
- remote environment variables fail closed.

Where per-CLI invocation values are involved, retain the repository-required
matrix: clean start, tracked resume, fork, session switch, and nested child
launch. Resume/fork only need regression coverage; they do not gain new
Orca-specific behavior.

### Real Orca acceptance

Run only after isolated tests pass:

1. Start a local Orca runtime without focusing or revealing a test window.
2. Import a disposable folder workspace.
3. Launch one hcom-managed Codex agent with `terminal=orca`.
4. Verify the terminal appears with the hcom instance title and the agent binds.
5. Send an hcom message and verify normal hook delivery.
6. Confirm Orca desktop and browser clients show the same terminal/session.
7. Kill the agent through hcom and verify only its exact Orca terminal closes.
8. Repeat with Claude if installed.
9. Restart Orca and verify stale-handle behavior is explicit and conservative.

Do not use live `~/.hcom` for the acceptance fixture. Do not run third-party CLI
smokes excluded by local release policy.

## Implementation sequence

1. Ask `agent_coach` to create and configure the dedicated `orca_dev` owner.
   This is the coach's only role in the implementation workflow.
2. Have `orca_dev` fork the canonical Orca repository, set its writable fork as
   `origin` and the canonical repository as `upstream`, fetch the current
   upstream base, and create a dedicated development branch. Verify the remote
   URLs and branch base before any code changes. Keep every change surgical,
   limited to the agreed CLI contract, and ready for an upstream pull request.
3. Give `orca_dev` the static findings and proposed generic CLI contract above.
4. Work directly with `orca_dev` to finalize the flag name and capability
   identifier, then have it implement, test, commit, and release that minimal
   CLI extension first.
5. Run the runtime discovery gate against the released local Orca CLI.
6. Add failing hcom unit and CLI smoke tests for the agreed contract.
7. Implement the built-in preset, native adapter, JSON parsing, handle storage,
   and exact close path.
8. Complete the hcom user-facing documentation/help matrix.
9. Run hcom verification required for a binary-affecting change:
   `cargo fmt --check`, `cargo clippy --bin hcom --all-targets`,
   `cargo test --bin hcom`, and focused `cargo test --test cli_smoke <filter>`.
10. Inspect the full diff, remove `tmp/` artifacts, commit, build release, and
   verify `hcom --version` according to local deployment rules.
11. Run the real Orca acceptance sequence and record the supported Orca version
    or capability in the release notes.

## Acceptance criteria

- `terminal=orca` launches an hcom-managed agent into the exact registered Orca
  workspace matching the canonical cwd.
- Launch never focuses a window unless explicitly supported in a later change.
- The agent receives the same hcom identity, catalog context, instructions, and
  delivery behavior as another local terminal backend.
- hcom stores the opaque Orca terminal handle and `hcom kill` closes only that
  terminal.
- A failed second launch step does not leave an empty Orca terminal.
- Missing CLI/runtime/workspace/capability errors are actionable.
- Remote Orca selection is rejected, not silently attempted.
- Runtime restart or stale handle never becomes false evidence of process exit.
- No Orca orchestration Run, Task, or Dispatch is created for a normal hcom
  launch.
- All documentation/help/test matrices and local release steps are complete.

## Deferred remote design

Remote support needs a separate design because direct local creation of a
terminal on a remote Orca server would split authority:

```text
local hcom database     creates and tracks the instance
remote Orca execution  runs hooks against the remote filesystem/database
```

Do not enable it by forwarding `--environment`. A future design must execute
the authoritative hcom launch on the Orca execution host or bridge it through
hcom relay so registration, hooks, delivery, and lifecycle share one authority.
That work is intentionally not a prerequisite for the local desktop backend.

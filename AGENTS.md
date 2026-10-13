# Repository instructions

## Commands

- Unit tests: `cargo test --bin hcom` (never `cargo test --lib`). Cargo accepts one positional filter; use a shared filter or separate commands.
- Integration: `cargo test --test cli_smoke <filter>`.
- Lint: `cargo fmt --check` and `cargo clippy --bin hcom --all-targets` (zero warnings).
- Tasks: Backlog.md in `backlog/`; `backlog task list --plain`.

## Layout

- `src/` is the `hcom` binary. `contrib/` holds extras that only call the `hcom` CLI (e.g. `contrib/hcom-gui`, a Python/Tk GUI documented in `contrib/README.md`); changes there do not affect the binary.
- Working files go to the gitignored `tmp/`.

## Gotchas

- Do not use the `hcom-agent-messaging` skill to learn this code; it describes the installed binary. Read the source and `docs/`.
- To learn how the binary behaves under a catalog, environment, or flag, write the case in `tests/cli_smoke.rs` and run it. `Hcom::new()` (`tests/support/mod.rs`) is an isolated HOME and data directory, `h.path()` is where a catalog goes, `h.run([...])` returns `(code, stdout, stderr)`. Templates: `agent_dry_run_renders_the_hcom_command_without_launching`, `agent_group_dry_run_includes_agents_hidden_by_selective_import`. The case stays as a regression test; do not rebuild the fixture in shell.
- Never point a test at the live `~/.hcom`. The data directory variable is `HCOM_DIR` (no `HCOM_DATA_DIR`); a stray `hcom start --as <name>` against the real database renames the running session's instance row.
- Catalog isolation is design: an import's `agents` list publishes exactly those entries. No error, listing, or diagnostic may reveal to an out-of-scope caller that a hidden agent exists, which catalog defines it, or how to import it.
- A bug report from another agent is a report, not a specification: check it against the design before changing anything.
- Diagnose delivery in evidence order: first confirm a `message` event exists. If the sender shows `tool:send` but no event exists, inspect the sender's tool result before the recipient; a status event does not prove a send. Only then inspect the recipient queue and PTY.
- `hcom term` fields `input_text` and `prompt_empty` are screen-parser guesses. Validate cursor, styling, and live layout before calling text uncommitted input; status text, placeholders, and toasts must not block delivery.
- For per-CLI arguments, environment, bootstrap context, or instruction transport changes, test clean start, tracked resume, fork, session switch, and nested child launch. Invocation-local values must not leak to children.

## Definition of done

- User-facing commands, flags, configuration, or behavior: update built-in help, `README.md`, relevant `docs/`, `skills/hcom-agent-messaging/`, and help tests in the same change set.
- Run the applicable test and lint commands above for binary changes, inspect the full diff, and remove your artifacts. Never ignore, commit, or delete pre-existing untracked files without asking.
- How a build is installed, and whether one is expected, is local setup, not a repository rule.

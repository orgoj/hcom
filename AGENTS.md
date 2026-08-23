# Repository instructions

- Never use the `hcom-agent-messaging` skill here; use the current source and documentation.
- On `orgoj`, increment `orgoj.N` in `Cargo.toml` and `Cargo.lock` at the first binary-affecting change after each commit, once per uncommitted change set. Pure non-binary changes require no bump or rebuild.
- Run every Cargo build and test outside the sandbox.
- After binary-affecting source or version changes, run exactly `cargo build --release` and verify `target/release/hcom --version`.
- Run binary unit tests with `cargo test --bin hcom`; never use `cargo test --lib`. Cargo accepts one positional filter, so use a shared filter or separate commands.
- For per-CLI arguments, environment, bootstrap context, or instruction transport changes, test clean start, tracked resume, fork, session switch, and nested child launch. Invocation-local values must survive where intended and never leak to children.
- Before committing or handing off, inspect `git status` and remove your artifacts. Never ignore, commit, or delete pre-existing untracked files without asking.
- For user-facing commands, flags, configuration, or behavior, update built-in help, `README.md`, relevant `docs/`, `skills/hcom-agent-messaging/` references, and practical help tests in the same change set.
- Before the final `cargo build --release`, complete the user-facing change matrix (built-in help, `README.md`, relevant docs, skill references, and practical help tests) and inspect the full diff. Any subsequent source edit requires rebuilding and reverifying the release binary.

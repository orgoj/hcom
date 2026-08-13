# Repository instructions

## `orgoj` branch versioning

On the `orgoj` branch, increment the `orgoj.N` prerelease component in both
`Cargo.toml` and `Cargo.lock` when starting the first change after a commit that
affects the `hcom` binary. That version covers the entire working change set
until it is committed: do not increment again for follow-up edits, fixes,
documentation, instructions, tests, or rebuilds made before that commit. Pure
documentation, instruction, metadata, and ignore-file changes do not require a
version increment or rebuild. After the binary-affecting change set is
committed, the first new binary-affecting change starts the next version.
Rebuild `hcom` after source or version changes that affect the binary.

## Cargo builds

Never run a Cargo build inside the sandbox. Run every Cargo build with sandbox
escalation so Cargo can use its cache and target-directory locks correctly.
For the required `hcom` rebuild, run exactly `cargo build --release`: use the
standard full release build, do not add `--bin` or otherwise narrow its targets,
and verify the result with `target/release/hcom --version`.

## Handoff cleanliness

Before committing or handing off completed work, inspect `git status`.
Remove artifacts created by the current work. Do not silently ignore,
commit, or delete pre-existing untracked files: identify them explicitly
and resolve their disposition with the user before declaring the worktree
clean.

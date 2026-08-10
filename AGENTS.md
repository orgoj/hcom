# Repository instructions

## `orgoj` branch versioning

On the `orgoj` branch, increment the `orgoj.N` prerelease component in both
`Cargo.toml` and `Cargo.lock` when starting the first change after a commit.
That version covers the entire working change set until it is committed: do
not increment again for follow-up edits, fixes, documentation, instructions,
tests, or rebuilds made before that commit. After the change set is committed,
the first new change starts the next version. Rebuild `hcom` after source or
version changes.

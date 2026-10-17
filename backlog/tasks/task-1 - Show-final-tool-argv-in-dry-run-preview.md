---
id: TASK-1
title: Show final tool argv in dry-run preview
status: To Do
assignee: []
created_date: '2026-10-13 12:21'
updated_date: '2026-10-17 06:57'
labels: []
dependencies: []
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `hcom <tool> --dry-run` preview shows only tool, directory and terminal, not the effective argv hcom would execute. Flags hcom injects (e.g. Codex `-c sandbox_workspace_write.*`) therefore cannot be verified without reading the source. User-approved idea (2026-10-10); implement only when the user asks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dry-run preview prints the final tool argv, including flags injected by hcom
- [ ] #2 Built-in help and README describe the argv in the dry-run output
- [ ] #3 A cli_smoke test asserts injected flags appear in the dry-run output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred: the final argv is built per instance inside launcher::launch() (src/launcher.rs), interleaved with side effects (instance row, name, hooks, system-prompt file, workspace trust) and per-instance bootstrap text (Codex developer_instructions). A faithful preview needs a plan/execute split of launch() - a large upstream-touching refactor needing a proposal and review. Smaller alternative: preview merged args (prepare_launch_execution) plus side-effect-free injections with a <bootstrap> placeholder, documented as excluding per-instance values.
<!-- SECTION:NOTES:END -->

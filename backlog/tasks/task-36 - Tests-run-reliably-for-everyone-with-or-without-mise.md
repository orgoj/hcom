---
id: TASK-36
title: 'Tests run reliably for everyone, with or without mise'
status: To Do
assignee: []
created_date: '2026-10-16 22:01'
updated_date: '2026-10-17 07:17'
labels: []
dependencies: []
priority: high
type: chore
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cli_smoke fails in some environments for reasons unrelated to the code under test. Full run from a plain shell: 118 passed, 6 failed, all for cause 1; cause 2 appears when TMPDIR is under the real HOME.

1. node via mise shim. Codex tests (agent_catalog_system_prompt_file_renders_and_reaches_spawn_script, agent_group_continues_after_a_member_fails, targeted_send_*) run the pinned target/mock-tools/bin/codex (codex.js, #!/usr/bin/env node). node resolves to the mise shim, which locates its installs via MISE_DATA_DIR / XDG_DATA_HOME / HOME; the fixture sets HOME and XDG_* to the temp root, so the shim fails: 'mise ERROR node is not a valid shim'. Verified: with the real node dir first in PATH all codex tests pass. Codex itself is not mise-installed; setup confirmed fine by agent bin. Fix: tests/support/pins.rs tool_path resolves the real node once from the parent env (node -p process.execPath) and puts its directory after the mock-tools dir, ahead of the inherited PATH. No hardcoded mise paths, no mise env passthrough (would leak user mise state). No-op without node or without mise.

2. Real ~/.hcom leaks into tests when TMPDIR is under the real HOME (e.g. the Claude harness TMPDIR /home/michael/.hermes/cache/scratch). find_project_hcom_dirs (src/commands/agent.rs, fork code) walks up from the workspace; the fixture's global catalog is the temp one, so the real ~/.hcom is taken as a project catalog (kill_by_group_* then fail with 'unknown or empty agent group'). The unit-test guard outside_test_tree only exists under cfg(test). Fix: in the non-test build, ignore project catalogs outside HCOM_TEST_ROOT when it is set (same pattern as hook_cleanup_allowed in src/runtime_env.rs).

3. Setup is undocumented: add to AGENTS.md Commands that cli_smoke needs 'just mock-tools' (pinned CLIs from scripts/mock-tools.pins) once and after a pin bump.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo test --bin hcom and cargo test --test cli_smoke pass from a plain shell with node installed via mise, and on a machine without mise
- [ ] #2 Tests pass without overriding TMPDIR
- [ ] #3 The required setup (mock tools install, prerequisites) is a single documented command
<!-- AC:END -->

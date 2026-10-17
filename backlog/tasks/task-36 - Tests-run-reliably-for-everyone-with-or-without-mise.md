---
id: TASK-36
title: 'Tests run reliably for everyone, with or without mise'
status: Done
assignee: []
created_date: '2026-10-16 22:01'
updated_date: '2026-10-17 07:37'
labels: []
dependencies: []
priority: high
type: chore
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cli_smoke failed in some environments for reasons unrelated to the code under test. Full run from a plain shell: 118 passed, 6 failed (cause 1); cause 2 appeared when TMPDIR was under the real HOME.

1. node via mise shim. Codex tests run the pinned target/mock-tools/bin/codex (codex.js, #!/usr/bin/env node). The mise node shim finds its installs via MISE_DATA_DIR / XDG_DATA_HOME / HOME, which the fixture isolates, so it failed with 'mise ERROR node is not a valid shim'. Codex itself is not mise-installed (confirmed by agent bin). Fix (tests only): tests/support/pins.rs tool_path adds the directory of 'mise which node' when mise exists; no-op without mise.

2. Real ~/.hcom leaked into tests when TMPDIR was under the real HOME (e.g. the Claude harness scratch dir): find_project_hcom_dirs walks up from the workspace and took the real ~/.hcom as a project catalog (kill_by_group_* failed with 'unknown or empty agent group'). Fix (tests only): Hcom::new creates the fixture root in /tmp on Linux. An explicit HCOM_TEST_KEEP_DIR is the caller's choice and keeps its location.

Scope (user decision): minimal tests-only fix; no src change, no AGENTS.md or setup documentation change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo test --bin hcom and cargo test --test cli_smoke pass from a plain shell with node installed via mise, and on a machine without mise
- [x] #2 Tests pass without overriding TMPDIR
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tests-only fix: tests/support/pins.rs puts the dir of 'mise which node' on the tool PATH when mise exists (mise node shim fails under the isolated HOME/XDG); tests/support/mod.rs creates the fixture root in /tmp on Linux so a TMPDIR under the real HOME cannot reach the live ~/.hcom. Verified: full cli_smoke 124/124 with TMPDIR under HOME and plain PATH, cargo test --bin hcom 2807 passed, codex smoke tests pass with mise absent from PATH, fmt and clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

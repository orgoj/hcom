---
id: TASK-36
title: 'Tests run reliably for everyone, with or without mise'
status: To Do
assignee: []
created_date: '2026-10-16 22:01'
labels: []
dependencies: []
priority: high
type: chore
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cli_smoke tests fail in some environments for reasons unrelated to the code under test. Known so far (not yet fully analyzed):
- Tests that launch codex (e.g. targeted_send_materializes_roaming_agent_in_the_senders_git_root) fail with 'Codex hooks/list preflight failed ... mise ERROR node is not a valid shim'. The pinned mock codex (target/mock-tools/bin/codex) starts with '#!/usr/bin/env node'; node resolves to a mise shim (~/.local/share/mise/shims/node), which does not work under the test's isolated HOME.
- Catalog-group smoke tests fail falsely when the harness sets TMPDIR to a scratch dir (e.g. /home/michael/.hermes/cache/scratch); the current workaround is running cargo test with TMPDIR=/tmp.
Suggested direction (to be confirmed): resolve real interpreter paths (node) once before isolating HOME, or have the mock-tools install pin an absolute node, so launched tools do not depend on mise/HOME; make the fixture independent of the caller's TMPDIR; document one command that runs the full suite on a clean machine.
Needs analysis before implementation: list every environment-dependent failure, find each root cause, then complete the description and acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo test --bin hcom and cargo test --test cli_smoke pass from a plain shell with node installed via mise, and on a machine without mise
- [ ] #2 Tests pass without overriding TMPDIR
- [ ] #3 The required setup (mock tools install, prerequisites) is a single documented command
<!-- AC:END -->

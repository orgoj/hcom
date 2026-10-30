---
id: TASK-66
title: >-
  Herdr launch: place an agent in the workspace named after its own directory,
  not the launcher's
status: To Do
assignee: []
created_date: '2026-10-30 17:11'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed: hcom_dev (~/projects/hcom) and knowledge run in the Herdr workspace 'ansible-wdt', although each has its own working directory and the Herdr workspace should be named after the agent's session directory. Likely the workspace is taken from the launching agent's context (e.g. send autostart from an ansible-wdt agent) instead of the target's cwd. Root cause not yet analyzed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An agent launched into Herdr (hcom send autostart, hcom agent, hcom N <cli>) lands in the workspace named after its own working directory, regardless of the launcher's workspace
- [ ] #2 A roaming agent started for a project lands in that project's workspace
- [ ] #3 Regression test in tests/cli_smoke.rs covers a launch from a caller in a different directory
<!-- AC:END -->

---
id: TASK-85
title: hcom handoff relaunches with the CLI the agent was started with
status: To Do
assignee: []
created_date: '2026-11-03 16:25'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An agent started exceptionally with agy (catalog default is claude) was relaunched by 'hcom handoff' with the default claude. Handoff must keep the launch settings of the running instance (CLI tool, and the launch overrides it was started with), not re-resolve them from catalog defaults.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom handoff of an instance started with a CLI other than its catalog default relaunches it with the same CLI
- [ ] #2 Launch overrides of the running instance (model, args) survive the handoff relaunch
- [ ] #3 An instance started with the catalog default still relaunches as before
- [ ] #4 A cli_smoke regression test covers the non-default CLI case
<!-- AC:END -->

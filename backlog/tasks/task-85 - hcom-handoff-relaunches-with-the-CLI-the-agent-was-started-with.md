---
id: TASK-85
title: hcom handoff relaunches with the CLI the agent was started with
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-03 16:25'
updated_date: '2026-11-03 17:02'
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
- [x] #1 hcom handoff of an instance started with a CLI other than its catalog default relaunches it with the same CLI
- [x] #2 An instance started with the catalog default still relaunches as before
- [x] #3 A cli_smoke regression test covers the non-default CLI case
- [x] #4 Handoff adds only --cli; it passes no model or CLI args of its own (user: the CLI keeps its model itself)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cause: handoff relaunched via 'hcom agent <name>' without --cli, so the catalog default CLI won. Fix: without --cli, handoff passes the instance's recorded tool. User decision: preserve only the CLI, no model/args.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
handoff passes --cli <recorded tool> when --cli is omitted (src/commands/handoff.rs); help, docs/agent.md, README, skill reference updated. Verified by cli_smoke handoff_keeps_the_cli_the_instance_was_launched_with (fails without the fix), all handoff smoke tests, cargo test --bin hcom, fmt, clippy.
<!-- SECTION:FINAL_SUMMARY:END -->

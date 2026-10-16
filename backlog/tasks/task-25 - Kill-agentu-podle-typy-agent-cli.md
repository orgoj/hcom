---
id: TASK-25
title: 'hcom kill tool:<cli>: kill all agents of one CLI type'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 14:44'
updated_date: '2026-10-16 05:02'
labels: []
dependencies: []
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Kill all running agents of a given CLI (e.g. all Antigravity agents) in one command. Syntax follows the existing tag:<name> target and the 'tool' field of 'hcom list': 'hcom kill tool:antigravity'. The prefix '#' proposed originally does not work, because the shell treats '#agy' as a comment. hcom-gui offers the same in its Kill menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 'hcom kill tool:<cli>' kills every running instance whose tool is <cli> and reports the count, like tag:
- [x] #2 Known CLI aliases (e.g. agy for antigravity) resolve to the same tool; an unknown tool fails with an error listing valid tools
- [x] #3 A tool with no running instances reports that nothing was killed and exits non-zero, like an empty tag:
- [x] #4 hcom-gui Kill menu lists the CLIs of running instances and kills them after confirmation listing their names
- [x] #5 Built-in help, README.md, docs/, skills/hcom-agent-messaging/, contrib/README.md and tests (cli_smoke, contrib/test-hcom-gui) cover tool:
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
kill_by_tag generalized to kill_matching(instance/orphan filters), shared with kill_by_tool; tool parsed via Tool::from_str (aliases). GUI Kill group menu adds tool:<cli> entries (kills confirmed names). Clippy: only pre-existing warning in src/hooks/claude.rs:2111. Sent to agent_review.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 'hcom kill tool:<cli>' (aliases via Tool::from_str, unknown tool lists valid ones, none running exits 1) sharing kill_matching with tag:; hcom-gui Kill group menu lists running CLIs (tool:<cli> (n)) and kills confirmed names. Help (top-level + kill), README, skill, contrib/README updated. Verified: cargo test --bin hcom, cli_smoke kill (incl. new kill_by_tool_kills_only_instances_of_that_cli), contrib/test-hcom-gui 58 OK; reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

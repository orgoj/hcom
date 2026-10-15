---
id: TASK-20
title: 'hcom-gui: catalog row of a running agent does not look stopped'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 13:44'
labels: []
dependencies: []
type: bug
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When an agent runs only under another instance name (--as x), its catalog row agent| stays and is greyed as stopped, although the agent runs. After Start with --as, the old catalog row stays selected instead of the new instance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A catalog row whose agent has running instances is not styled as stopped and its Status names the running instances
- [x] #2 After Start with --as <name>, the new instance row is selected once it appears
- [x] #3 Covered by contrib/test-hcom-gui
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Tests: catalog row a2| (agent running as a2x) has Status 'running as a2x' and no stopped tag; Start with --as x selects a3|x once it appears. 2. build_rows: collect running instance names per agent, set catalog-row status. 3. render: tag stopped only without status; pending selection set by run_button from ask:name, applied when the row exists, dropped after the command finishes without it. 4. README.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
build_rows: catalog row of an agent running only under other names gets Status 'running as <names>' and is not tagged stopped. Start with --as sets a pending selection (select_next) applied once the row appears; replaced by every Start, cancelled by a manual selection or by failure of its own command. Review (agent_review_hcom): two low findings fixed. Verified: contrib/test-hcom-gui 48 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

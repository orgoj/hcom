---
id: TASK-61
title: 'hcom-gui: agent status columns and a bottom panel tab with all status values'
status: Idea
assignee: []
created_date: '2026-10-29 05:39'
updated_date: '2026-10-29 05:41'
labels: []
dependencies:
  - TASK-60
priority: medium
type: feature
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up to TASK-60 (agent status columns parsed from the final reply), consuming its data via 'hcom agent list --status --json'. User request: status columns in the agent table like other columns, and a new bottom panel tab showing all status values of the selected agent, including the columns that are enabled in the table. Open: value order in the tab (order of [[status_column]] in config.toml, a configurable order in gui.json, or a fully user-defined layout), multi-line values (full Done/Issues/Next text), staleness display (updated_at age). Suggested direction: config order first, no extra GUI config. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decision (2026-10-29): the tab order is an explicit list in ~/.hcom/gui.json, independent of the table columns, because the tab must put different values first than the columns show. Proposed: values not in the list follow in [[status_column]] config order (to confirm at specification).
<!-- SECTION:NOTES:END -->

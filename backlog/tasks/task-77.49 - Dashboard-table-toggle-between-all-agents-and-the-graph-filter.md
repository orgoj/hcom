---
id: TASK-77.49
title: 'Dashboard: table toggle between all agents and the graph filter'
status: To Do
assignee: []
created_date: '2026-11-06T23:07:00Z'
updated_date: '2026-11-06T23:09:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 144000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user also uses the table to launch agents that are not running, so a table limited by the search/filters hides rows they need. Requested: a toggle that switches the table between all agents (full) and the same filter as the graph (filtered); the graph filter is unaffected. Selecting a node already selects its row (cross-selection exists); in scope is only making the table scroll to that row, if it does not already.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A toggle switches the table between full (all agents and catalog agents) and filtered (same search and filters as the graph); the graph always follows the search and filters
- [ ] #2 The toggle value persists in the state file and is restored after a restart
- [ ] #3 Selecting a node in the graph scrolls the table so that agent's row is visible whenever the table shows it (keep the existing behavior if it already does this)
<!-- AC:END -->

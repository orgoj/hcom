---
id: TASK-42
title: 'hcom-gui: agent usage statistics columns'
status: To Do
assignee: []
created_date: '2026-10-23 13:34'
updated_date: '2026-10-23 14:07'
labels: []
dependencies:
  - TASK-42.1
priority: medium
type: feature
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: see how agents are used (how often they start, how long they run, how often they hand off, message volume, how they end) directly in hcom-gui.

Facts: start/stop are already logged as `life` events (actions created, ready, stopped with reason, launch_failed, launch_blocked); messages are `message` events. History lives in the live DB since the last `hcom reset`. Handoffs need the marker from TASK-42.1.

In scope (contrib/hcom-gui only): new optional columns computed in the GUI from `hcom events` output of the live DB, aggregated per instance name (catalog agents map 1:1; each roaming instance has its own row). Metrics: Starts, Runtime (sum of start→stop intervals, a running session counts up to now), Handoffs, Messages sent/received, Last start, Stops by reason (killed / idle autokill / exited by the CLI), Failed launches (launch_failed + launch_blocked). Statistics refresh at a lower rate than the table refresh.
Out of scope: archived databases and a rolling window (TASK-42.2); active/listening utilisation from status events (too many rows to fetch through the CLI); any binary change besides TASK-42.1.
Risks: event volume (thousands of life/message rows) must not slow the 5 s refresh; the GUI only calls the hcom CLI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Columns Starts, Runtime, Handoffs, Msgs (sent/received), Last start, Stops (by reason: killed/idle/exit), Failed launches exist in COLUMNS, are selectable in the column dialog and gui.json columns, and sort numerically (Last start by time)
- [ ] #2 Values are computed from life and message events of the live database via the hcom CLI and aggregated per instance name; a catalog row without any events shows empty cells
- [ ] #3 Runtime counts a currently running session up to now; Handoffs counts only events carrying the TASK-42.1 handoff marker
- [ ] #4 Statistics are recomputed at most once per a configurable interval (gui.json, default 60 s), not on every table refresh
- [ ] #5 contrib/test-hcom-gui covers each column from fixture events, including a running session and a row without events; contrib/README.md documents the columns and the interval
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read how hcom-gui builds rows and COLUMNS; check which hcom events invocation returns all life+message events of the live DB without a --last cap.
2. Implement a cached stats loader (interval from gui.json) and the aggregation per instance.
3. Add columns, sorting, column-dialog entries.
4. Fixture-based tests in contrib/test-hcom-gui, README update.
<!-- SECTION:PLAN:END -->

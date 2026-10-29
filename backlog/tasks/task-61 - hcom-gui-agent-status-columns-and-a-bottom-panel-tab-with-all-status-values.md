---
id: TASK-61
title: 'hcom-gui: agent status columns and a bottom panel tab with all status values'
status: To Do
assignee: []
created_date: '2026-10-29 05:39'
updated_date: '2026-10-29 05:45'
labels: []
dependencies:
  - TASK-60
priority: medium
type: feature
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: show the agent status values of TASK-60 in contrib/hcom-gui: as optional table columns and in a bottom panel tab with all values of the selected row, full text, so the user sees an agent's reported state without opening its screen.

Data: the existing refresh call 'hcom agent list --all --json' gains --status (TASK-60); values are per instance; a catalog row without a running instance shows its agent's most recent instance values. No extra hcom call per row or tab.

Columns: one column per [[status_column]] reported by hcom, key 'status:<name>', title <name>, value shortened to one line; selectable and orderable in gui.json 'columns' and the Columns dialog like built-in columns; gui-state.json columns_known handling treats them as known columns.

Tab: new bottom tab 'Report' after Log, key F9; one block per value: name, age of updated_at, full wrapped text. Order: gui.json 'report' list given -> exactly the listed values in list order; no list -> values shown as table columns first (column order), then the rest in config order. Empty when the row has no values.

Out of scope: editing [[status_column]] from the GUI, remote/relay agents, adhoc instances without a catalog agent, changes to the binary beyond TASK-60.

Risks: dynamic column keys vs gui-state.json columns_known and LEGACY_KNOWN_COLUMNS logic (a removed config column must not break a saved choice); refresh cost unchanged (same call).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each configured status column appears in the Columns dialog and can be shown, hidden and ordered; gui.json 'columns' accepts its key status:<name>
- [ ] #2 A shown status column cell holds the instance's value shortened to one line; a stopped catalog row holds its agent's most recent values
- [ ] #3 The Report tab (F9, after Log) shows for the selected row each value with its name, age since updated_at and full wrapped text
- [ ] #4 With gui.json 'report' set, the tab shows exactly those values in that order; without it, shown status columns first in column order, then the remaining values in config order
- [ ] #5 A saved column choice referring to a status column no longer configured loads without error and drops that column
- [ ] #6 contrib/test-hcom-gui covers each AC; contrib/README.md and the module docstring document the columns, the tab and 'report'
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Refresh: add --status to the agent list call; map per-instance values into rows (most recent for catalog rows without instance). 2. Dynamic COLUMNS: append status:<name> specs from the payload; adapt pick_columns/with_new_columns/columns_known. 3. Report tab: Text widget, F9, render from the selected row (no hcom call), wrap, age from updated_at; gui.json 'report' ordering. 4. Tests, README, docstring. 5. Review per step 5 (non-small: plan review with agent_review first).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decision (2026-10-29): the tab order is an explicit list in ~/.hcom/gui.json, independent of the table columns, because the tab must put different values first than the columns show. Proposed: values not in the list follow in [[status_column]] config order (to confirm at specification).

User decision (2026-10-29, replaces the proposal above): with the gui.json list defined, the tab shows only the listed values in list order; without it, the default shows everything: values enabled as table columns first (in column order), then the rest in [[status_column]] config order.
<!-- SECTION:NOTES:END -->

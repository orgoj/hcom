---
id: TASK-41
title: >-
  hcom-gui: keep the selection at the same row index when the selected row
  disappears
status: To Do
assignee: []
created_date: '2026-10-20 16:42'
updated_date: '2026-10-20 16:45'
labels: []
dependencies: []
priority: medium
type: enhancement
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When the selected row disappears from the table, render() (contrib/hcom-gui) falls back to the first row of the same catalog agent, or selects nothing for an ad-hoc instance or when no row of that agent is visible (e.g. Running only filter). After Kill (Del) the user loses their place in the list as they have it sorted and filtered.

Goal: when the selected row disappears, select the row now at the same position (index) in the current sort order and filters, so Kill and similar removals keep the user's place.

In scope: render() selection fallback; tracking the index of the selected row as displayed before the refresh; the rule applies to every disappearance (Kill, Kill group, an instance ending on its own, a refresh that filters it out), including a catalog agent's instance whose row turns into the stopped catalog row (index wins over the same-agent jump); contrib/README.md; cases in contrib/test-hcom-gui.

Exception (user decision): commands that relaunch the selected agent keep today's behavior: Start (when=catalog) selects the new instance or the same agent's row, and Restart (when=live) keeps the same agent while its row is briefly missing. Index fallback applies to every other disappearance.

Out of scope: selection when the selected row still exists; select_next for Start --as; sort and filter behavior itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After the selected row disappears, the row at the same index in the current sort and filter is selected and focused, also when the disappearing row was a catalog agent's instance whose agent still has another row elsewhere
- [ ] #2 When the disappeared row was the last one, the new last row is selected; nothing is selected only when the table is empty
- [ ] #3 While and after a Start (when=catalog) or Restart (when=live) command for the selected row, selection follows the same agent as today (new instance via select_next or the agent's row), not the index
- [ ] #4 contrib/README.md describes the selection rule after a row disappears
- [ ] #5 contrib/test-hcom-gui covers: middle row killed, last row killed, catalog instance killed, row hidden by a filter refresh, Start and Restart keeping the agent
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. In render(), remember the displayed index of the selected row before rebuilding the tree.
2. If the selected id no longer exists: when a Start/Restart for that agent is pending or just finished, keep the existing same-agent fallback; otherwise select children[min(index, len-1)].
3. Mark Start/Restart in run_button/run_command (by the button's when: catalog/live) and clear the mark when the command's refresh has applied.
4. Update contrib/README.md; add cases to contrib/test-hcom-gui and run it.
<!-- SECTION:PLAN:END -->

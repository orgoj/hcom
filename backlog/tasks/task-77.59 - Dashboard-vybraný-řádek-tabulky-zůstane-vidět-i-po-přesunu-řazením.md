---
id: TASK-77.59
title: 'Dashboard: vybraný řádek tabulky zůstane vidět i po přesunu řazením'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T11:01:00Z'
updated_date: '2026-11-07T11:04:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 154000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Po spuštění agenta drží tabulka výběr na něm, ale řádek se podle zvoleného řazení přesune na jiné místo, mimo zobrazené řádky. Vybraný řádek musí být vždy ve viditelné části; tabulka se k němu posune.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Když se vybraný řádek přesune na jinou pozici (řazení, nový stav po spuštění), tabulka se posune tak, aby byl vidět
- [x] #2 Když se pozice vybraného řádku nemění, tabulka nescrolluje sama (uživatelův scroll zůstává)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tabulka si pamatuje (id, index) posledního scrollu; přesun vybraného řádku na jiný index (řazení, spuštěný agent) ho jednou posune do zobrazení, beze změny pozice zůstává scroll uživatele. Test test_task_77_59_selected_row_moved_by_sorting_scrolls_into_view (bez opravy selže). Malá změna, bez review.
<!-- SECTION:FINAL_SUMMARY:END -->

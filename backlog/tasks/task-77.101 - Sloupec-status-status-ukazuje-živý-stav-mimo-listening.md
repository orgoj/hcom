---
id: TASK-77.101
title: Sloupec status:status ukazuje živý stav, mimo listening
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-09T20:29:00Z'
updated_date: '2026-11-09T20:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 198000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: status:status (STATUS z poslední odpovědi agenta, catalog status_columns) platí jen když agent čeká (listening). Běžící agent v jiném stavu (active, blocked, ...) má ve status:status svůj hcom stav; zastavený agent poslední STATUS, se kterým skončil. Místo: TableRow::get_cell_value (table_data.rs), takže tabulka, řazení, hledání i Report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Běžící agent ve stavu active/blocked (jiném než listening) má ve status:status svůj stav
- [x] #2 Agent ve stavu listening má ve status:status hodnotu STATUS z poslední odpovědi
- [x] #3 Zastavený agent má ve status:status poslední hodnotu STATUS
- [x] #4 README popisuje chování
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
get_cell_value: status:status of a running row whose status is set and not listening returns the status. Test test_task_77_101_*; test_build_table_rows_merges_live_and_catalog updated (active row now shows active). Dashboard fmt, clippy, 253 tests green. Small change, no review. Done after the user's check.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented, tested and accepted by the user in the running dashboard.
<!-- SECTION:FINAL_SUMMARY:END -->

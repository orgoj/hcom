---
id: TASK-77.102
title: Konfigurovatelné pořadí řazení podle stavu (status:status)
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-09T21:02:00Z'
updated_date: '2026-11-09T21:11:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 199000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: řazení tabulky hcom-dashboard podle stavu nemá být řetězcové (sort_rows v contrib/hcom-dashboard/src/table_data.rs porovnává get_cell_value), ale podle toho, na co má uživatel reagovat; pořadí definovatelné v konfiguraci (~/.hcom/dashboard.json). Navržené výchozí pořadí: blocked, NEEDS_USER, FOLLOWUP (DONE_FOLLOWUP), DONE (jen běžící agenti), ostatní listening (např. WAITING), active, jiné stavy, všichni neaktivní (zastavení). Souvislost: status:status ukazuje živý stav mimo listening, jinak STATUS poslední odpovědi (TASK-77.101). Otevřené: kterých sloupců se to týká (status:status, status, oba), přesné názvy klíčů a tvar konfigurace, pořadí uvnitř skupiny, směr řazení po kliknutí na hlavičku. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Řazení podle status:status jde v pořadí: blocked, error, NEEDS_USER, DONE_FOLLOWUP, DONE (běžící), ostatní listening, active, jiné stavy, neaktivní (zastavení)
- [x] #2 Stejný stav řadí podle since, nejstarší první; agent bez since za ostatními
- [x] #3 Obrácené řazení (druhé kliknutí na hlavičku) obrátí celé pořadí
- [x] #4 Pořadí je v kódu (bez konfigurace); README ho popisuje
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decisions: column status:status only; equal state ordered by since, oldest first; order hardcoded for now (no config), configurable later after trying it. Proposal under discussion: error right after blocked.

Specified with the user: error exists (ST_ERROR, src/shared/constants.rs) and goes right after blocked. Implementation: rank in sort_rows (table_data.rs) for col status:status, bypassing empty_last (rank puts stopped last).

status_rank + status:status branch in sort_rows. Test test_task_77_102_*; dashboard fmt, clippy, 254 tests green (overload throttle test flaked once, passed on rerun). Small change, no review. Done after the user's check.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
status:status sorts by urgency (blocked, error, NEEDS_USER, DONE_FOLLOWUP, DONE, other listening, active, other, stopped), oldest since first; hardcoded. Accepted by the user.
<!-- SECTION:FINAL_SUMMARY:END -->

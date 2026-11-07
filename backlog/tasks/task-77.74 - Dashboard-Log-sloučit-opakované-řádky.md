---
id: TASK-77.74
title: 'Dashboard Log: sloučit opakované řádky'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:59:00Z'
updated_date: '2026-11-07T15:09:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 169000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07 (návrh 3): v Logu bylo 6x po sobě stejné [ERROR] Failed to fetch live/agent data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Po sobě jdoucí stejné řádky Logu se zobrazí jednou s počtem (×N); barva podle ERROR/OK zůstává
- [x] #2 Test pokrývá sloučení i oddělené stejné řádky
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Log slučuje po sobě jdoucí stejné řádky (collapse_repeats) s počtem (×N), barvy beze změny. Testy test_task_77_74_*; review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

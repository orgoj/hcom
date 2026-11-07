---
id: TASK-77.75
title: 'Dashboard Sessions: řetězec handoffů'
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
ordinal: 170000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07 (návrh 5): ukázat, ze které sešny sešna pokračuje a do které přešla. Handoff event předchází stopped staré sešny a created nové; dnes se konec sešny hlásí jako killed (cli).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sešna ukončená handoffem ukazuje ended: handoff a odkaz handed off to session <sid8>; sešna spuštěná handoffem ukazuje continues from session <sid8>
- [x] #2 Test pokrývá řetězec dvou handoffů
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Konec handoffem = ended: handoff, odkazy handoff_from/handoff_to mezi po sobě jdoucími sešnami. Test test_task_77_75_handoff_chain; review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

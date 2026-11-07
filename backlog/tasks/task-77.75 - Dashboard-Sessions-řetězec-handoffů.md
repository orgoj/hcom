---
id: TASK-77.75
title: 'Dashboard Sessions: řetězec handoffů'
status: To Do
assignee: []
created_date: '2026-11-07T14:59:00Z'
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
- [ ] #1 Sešna ukončená handoffem ukazuje ended: handoff a odkaz handed off to session <sid8>; sešna spuštěná handoffem ukazuje continues from session <sid8>
- [ ] #2 Test pokrývá řetězec dvou handoffů
<!-- AC:END -->

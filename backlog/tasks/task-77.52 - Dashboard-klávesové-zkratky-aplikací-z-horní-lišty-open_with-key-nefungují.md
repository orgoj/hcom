---
id: TASK-77.52
title: 'Dashboard: klávesové zkratky aplikací z horní lišty (open_with key) nefungují'
status: To Do
assignee: []
created_date: '2026-11-07T08:10:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 147000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
V ~/.hcom/dashboard.json mají položky open_with klíč key (repos=g, gitg=j, gitk=h, Zed=z, Backlog=b). Lišta je zobrazuje, ale stisk klávesy aplikaci nespustí.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stisk klávesy key položky open_with spustí tuto aplikaci pro vybraného agenta, stejně jako klik v liště
- [ ] #2 Zkratka nespustí aplikaci, když má fokus textové pole
<!-- AC:END -->

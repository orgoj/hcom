---
id: TASK-77.84
title: 'Dashboard: detail panel v minimální šířce ořezává obsah zleva'
status: To Do
assignee: []
created_date: '2026-11-07T15:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 179000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F8, A9). Při minimální šířce detail panelu (~205 px, app_tester v3-15/16) chybí začátky řádků (ty pilot na Cyrilu, :04 – running), tlačítko Resume, názvy záložek (Summary → ary). Při ~330 px je vše v pořádku. Očekávání: zalamování / ořez jen vpravo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Detail panel v minimální šířce: první znak každého řádku Sessions, každý název záložky a tlačítko Resume jsou viditelné (zalomení nebo ořez jen vpravo)
- [ ] #2 Headless test pokrývá minimální šířku panelu
<!-- AC:END -->

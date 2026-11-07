---
id: TASK-77.52
title: 'Dashboard: klávesové zkratky aplikací z horní lišty (open_with key) nefungují'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T08:10:00Z'
updated_date: '2026-11-07T09:06:00Z'
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
- [x] #1 Stisk klávesy key položky open_with spustí tuto aplikaci pro vybraného agenta, stejně jako klik v liště
- [x] #2 Zkratka nespustí aplikaci, když má fokus textové pole
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
open_with key je psaný znak (jako /), spustí položku pro vybraného agenta jako tlačítko; ne v textovém poli ani s otevřeným dialogem. Test test_task_77_52_open_with_key_runs_the_app_like_its_button, review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

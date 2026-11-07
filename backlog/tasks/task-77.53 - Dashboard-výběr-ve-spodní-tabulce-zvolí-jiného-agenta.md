---
id: TASK-77.53
title: >-
  Dashboard: globální Alt+šipka nahoru/dolů volí předchozího/dalšího agenta ve
  spodní tabulce
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T08:10:00Z'
updated_date: '2026-11-07T09:06:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 148000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Upřesnění uživatele k diktátu 'Jiného agenta zvolí ve spodní tabulce': globální zkratka Alt+šipka nahoru/dolů vybere předchozího/dalšího agenta ve spodní tabulce, ať je fokus kdekoli. Dnes šipky fungují jen s fokusem v tabulce (TASK-77.20).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alt+šipka dolů vybere dalšího agenta ve spodní tabulce, ať je fokus kdekoli (graf, pravý panel, textové pole)
- [x] #2 Alt+šipka nahoru vybere předchozího agenta ve spodní tabulce, ať je fokus kdekoli
- [x] #3 Výběr se propíše do grafu a pravého panelu jako při kliku na řádek a vybraný řádek je vidět
- [x] #4 Šipky bez Alt se chovají jako dosud
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Alt+Up/Down (z modifikátorů události klávesy) posouvá výběr v tabulce i s fokusem v textovém poli; výběr jde do grafu a detailu jako klik. Test test_task_77_53_alt_arrows_select_rows_wherever_the_keyboard_is, review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

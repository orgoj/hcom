---
id: TASK-77.110
title: 'Dashboard: šipky ve vyhledávání předají fokus tabulce'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-13T21:32:00Z'
updated_date: '2026-11-13T21:44:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
type: enhancement
ordinal: 221000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ve vyhledávacím poli tabulky šipky Nahoru/Dolů posouvají výběr, ale fokus zůstává v poli (app.rs search keys), takže Del (kill), Enter, s/m/r nefungují – shortcuts_on je vypnuté při fokusu textového pole. Rozhodnutí uživatele (varianta 1A): Up/Down v hledání přesune výběr a předá fokus tabulce; hledaný text a filtr zůstanou, / vrací fokus do hledání.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Up/Down ve vyhledávacím poli posune výběr o řádek a textové pole ztratí fokus
- [x] #2 Po přechodu šipkou funguje Del jako kill vybraného řádku (shortcuts_on)
- [x] #3 Text vyhledávání a filtr zůstanou po přechodu beze změny
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Up/Down ve vyhledávání tabulky posunou výběr a odevzdají fokus pole (surrender_focus), takže Del/Enter/s/m/r jednají nad řádkem; Alt+Up/Down fokus nechají. Ověřeno test_task_77_104_table_search_quick_select (Del → confirm_kill_target) a test_task_77_53, README aktualizováno.
<!-- SECTION:FINAL_SUMMARY:END -->

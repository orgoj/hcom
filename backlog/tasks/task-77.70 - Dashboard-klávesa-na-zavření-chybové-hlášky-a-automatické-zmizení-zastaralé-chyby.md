---
id: TASK-77.70
title: >-
  Dashboard: klávesa na zavření chybové hlášky a automatické zmizení zastaralé
  chyby
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:38:00Z'
updated_date: '2026-11-07T14:46:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 165000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: chybová hláška nahoře v detail panelu (action_error, banner s ✖, src/app.rs render detail panelu) zůstává, i když už neplatí. Například po startu nového agenta hlásí 'Screen for X failed', protože agent ještě nenaběhl (očekávaný stav). Když agent naběhne, hláška nezmizí. Uživatel chce klávesovou zkratku na zavření hlášek. Návrh: Escape zavře banner, když není otevřený dialog ani kontextové menu a fokus nemá textové pole. Úspěšné načtení Screen/Transcript/Events/Sessions pro agenta smaže chybu stejného druhu pro téhož agenta. Implementace až po commitu TASK-77.69 (stejné soubory).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Escape zavře zobrazenou chybovou hlášku detail panelu, když není otevřený modální dialog ani kontextové menu a textové pole nemá fokus; jinak se Escape chová jako dosud
- [x] #2 Úspěšné načtení Screen, Transcript, Events nebo Sessions agenta smaže zobrazenou chybu '<druh> for '<agent>' failed' téhož druhu a agenta; jiné chyby zůstanou
- [x] #3 Zkratka je uvedená v contrib/hcom-dashboard/README.md a tooltipu tlačítka ✖
- [x] #4 Headless testy pokrývají Escape (zavře / nezavře při dialogu a fokusu v poli) a automatické smazání po úspěšném načtení
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Escape zavře chybový banner detail panelu (ne při dialogu, kontextovém menu nebo fokusu v textovém poli z konce minulého snímku, protože egui fokus na Escape ruší před snímkem); úspěšné načtení Screen/Transcript/Events/Sessions smaže chybu stejného druhu a agenta (clear_stale_error). Tooltip ✖ 'Dismiss (Esc)', README. Ověřeno testy test_task_77_70_*, fmt, clippy 0, 213 + 17 testů; review agent_review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

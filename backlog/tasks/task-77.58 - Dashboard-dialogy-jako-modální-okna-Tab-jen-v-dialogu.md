---
id: TASK-77.58
title: 'Dashboard: dialogy jako modální okna (Tab jen v dialogu)'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T10:13:00Z'
updated_date: '2026-11-07T10:21:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 153000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Všech šest dialogů (Start, Send Message, Confirm Kill/Restart/Kill Group, potvrzení příkazu) je egui::Window: Tab chodí po tlačítkách v horní liště a zbytek aplikace reaguje. Převést na egui::Modal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Otevřený dialog blokuje vstup do zbytku aplikace a Tab prochází jen jeho prvky
- [x] #2 Esc zavře každý dialog bez akce
- [x] #3 Dialog Start dál funguje podle TASK-77.56 (uprostřed, Enter OK)
- [x] #4 Po otevření má fokus první pole dialogu (Start: resume, Send: zpráva); potvrzovací dialogy fokusují Cancel, aby Enter nic nezabil
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Všech šest dialogů přes show_modal (egui::Modal, uprostřed okna, pozadí blokuje vstup, Esc mimo otevřený popup zavře, klik mimo ne); fokus prvního pole (Start resume, Send zpráva, potvrzení Cancel); globální zkratky vypnuté, když byl dialog otevřen na začátku snímku. Opraven test kontextového menu (bod 100,100 byl tlačítko Send...). Test test_task_77_58_dialogs_are_modal, review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

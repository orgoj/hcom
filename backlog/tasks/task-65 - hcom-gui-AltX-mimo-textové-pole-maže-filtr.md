---
id: TASK-65
title: 'hcom-gui: Alt+X mimo textové pole maže filtr'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-30 08:01'
updated_date: '2026-10-30 08:02'
labels: []
dependencies: []
type: enhancement
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozšíření TASK-64: Alt+X v hlavním okně mimo editovatelné textové pole (tabulka, záložka jen pro čtení) vymaže pole Filter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alt+X s fokusem v tabulce vymaže pole Filter a tabulka znovu zobrazí všechny agenty
- [x] #2 Alt+X v editovatelném poli dál maže jen to pole
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Alt+X navázané i na hlavní okno maže Filter; clear_input u needitovatelného pole klávesu propustí. Ověřeno contrib/test-hcom-gui (OK), test test_alt_x_outside_text_inputs_clears_the_filter.
<!-- SECTION:FINAL_SUMMARY:END -->

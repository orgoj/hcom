---
id: TASK-64
title: >-
  hcom-gui: ovládání – zkratka / na Find, tlačítko X pro clear, Kill group:
  filtered
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-30 07:30'
updated_date: '2026-10-30 07:41'
labels: []
dependencies: []
type: enhancement
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Zlepšení ovládání hcom-gui (contrib/hcom-gui): klávesová zkratka / skočí do pole Find, tlačítko X za Find filtr vymaže, a v Kill group přibude volba 'filtered', která zabije všechny running agenty zobrazené v aktuální (filtrované) tabulce.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kill group nabízí volbu 'filtered', která zabije všechny running agenty právě zobrazené v tabulce a žádné jiné
- [x] #2 Stisk / v hlavním okně (mimo editovatelné textové pole) přesune fokus do pole Filter
- [x] #3 Za polem Filter je tlačítko ✕, které vymaže text filtru a tabulka znovu zobrazí všechny agenty
- [x] #4 Alt+X smaže obsah jakéhokoli editovatelného textového pole, které má fokus (Filter i pole v dialozích)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Kill group ▾ má první položku Filtered (N) (běžící instance zobrazených řádků, potvrzení, neaktivní při 0); / fokusuje Filter mimo editovatelné pole; tlačítko ✕ maže filtr; Alt+X maže editovatelné textové pole ve všech oknech. Ověřeno contrib/test-hcom-gui (166 OK), revize agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

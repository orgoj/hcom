---
id: TASK-77.78
title: 'Dashboard Sessions: Restore window po Show in graph'
status: To Do
assignee: []
created_date: '2026-11-07T15:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 173000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F2, A3, A4; Q2 rozhodnuto: jen Restore window, pravidlo seznamu podle okna se nemění). Show in graph dnes zúží seznam na 1 session a cesta zpět je All + Live + ruční posun. Řešení: při Show in graph uložit aktuální okno (preset/interval, Live, last-N) a scroll offset Sessions, pokud už uložené z předchozího Show in graph není. V hlavičce Sessions pruh 'Graph shows session <id8> (<od> – <do>)   [Restore window]' (neutrální, ne chybový vzhled). Restore window (i Escape, když má Sessions fokus a není otevřený text/dialog) vrátí okno i scroll a zapomene je. Jakákoli jiná změna okna uživatelem (preset, slider, pole, Live) uložené zapomene a pruh skryje. Karta, jejíž interval je zobrazen, má zvýraznění výběru.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Seznam Sessions agenta se 171 sessions odscrollovaný na 3. kartu: po Show in graph se objeví pruh Graph shows session … [Restore window]; po Restore window je okno zpět All s Live zapnutým, seznam ukazuje 171 sessions a 3. karta je na stejné pozici
- [ ] #2 Po Show in graph a kliknutí na preset 6h pruh zmizí a obnova se nenabízí
- [ ] #3 Restore window je dosažitelné Tabem z hlavičky Sessions a Escapem
- [ ] #4 Karta zobrazené session je zvýrazněná
<!-- AC:END -->

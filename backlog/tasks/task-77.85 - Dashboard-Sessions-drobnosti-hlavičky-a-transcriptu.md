---
id: TASK-77.85
title: 'Dashboard Sessions: drobnosti hlavičky a transcriptu'
status: To Do
assignee: []
created_date: '2026-11-07T15:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: enhancement
ordinal: 180000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (P1, P2, P4, A12). P1: hlavička '171 sessions in the time window (171 total)' je při rovnosti zbytečná a '1 sessions' má špatný tvar; nově '24 sessions', jinak '1 of 171 sessions in the time window', jednotné '1 session'; prázdné okno 'No sessions in the time window' + (N total) a při N > 0 tlačítko Show all (nastaví All). P2: hlavička mimo scroll oblast s čárou pod ní, karty pod ní nepřejíždějí. P4: hlavička transcriptu session ukáže titulek/čas/id místo celé cesty .jsonl, cesta v tooltipu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Při 1 session v okně hlavička zní 1 of 171 sessions in the time window, nikdy 1 sessions
- [ ] #2 Hlavička Sessions je mimo scroll oblast, oddělená čárou
- [ ] #3 Prázdné okno ukazuje No sessions in the time window (N total) a tlačítko Show all
- [ ] #4 Hlavička transcriptu session ukazuje titulek, čas a id; cesta souboru je v tooltipu
<!-- AC:END -->

---
id: TASK-77.85
title: 'Dashboard Sessions: drobnosti hlavičky a transcriptu'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:58:00Z'
updated_date: '2026-11-07T16:56:00Z'
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
- [x] #1 Při 1 session v okně hlavička zní 1 of 171 sessions in the time window, nikdy 1 sessions
- [x] #2 Hlavička Sessions je mimo scroll oblast, oddělená čárou
- [x] #3 Prázdné okno ukazuje No sessions in the time window (N total) a tlačítko Show all
- [x] #4 Hlavička transcriptu session ukazuje titulek, čas a id; cesta souboru je v tooltipu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sessions header: sessions_header_text ('N sessions' / '1 of N sessions in the time window' / 'No sessions in the time window (N total)'), Show all sets All when N > 0; header above the scroll area with a rule, separators only between cards; session transcript header 'Session transcript: title · start · id8' with the path as tooltip. README updated. Tests test_task_77_85_*, 77.69 (tooltip) updated; gates green; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

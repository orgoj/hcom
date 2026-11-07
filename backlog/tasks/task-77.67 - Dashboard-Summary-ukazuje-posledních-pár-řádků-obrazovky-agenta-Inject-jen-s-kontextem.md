---
id: TASK-77.67
title: >-
  Dashboard: Summary ukazuje posledních pár řádků obrazovky agenta, Inject jen s
  kontextem
status: Done
assignee: []
created_date: '2026-11-07T12:01:00Z'
updated_date: '2026-11-07T12:17:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: pravý panel Summary pořád nemá posledních několik řádků ze Screen; u pole Inject v Summary nevím, do čeho píšu. Teď Summary má jen Quick Actions, pole Inject (vždy s Enter) a volitelně Recent Transcript; obrazovka se načítá jen na záložce Screen. Otevřené rozhodnutí: pole Inject v Summary buď zůstane pod náhledem obrazovky (stejný Enter checkbox jako ve Screen), nebo se ze Summary odstraní a inject zůstane jen ve Screen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Summary běžící instance ukazuje posledních několik řádků obrazovky (hcom term), obnovovaných jako záložka Screen
- [x] #2 Instance bez obrazovky (stopped, bez PTY) ukáže místo náhledu krátkou poznámku nebo nic
- [x] #3 Inject v Summary je buď přímo pod náhledem obrazovky, nebo je ze Summary odstraněný (podle rozhodnutí uživatele)
- [x] #4 Headless test ověří náhled v Summary
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele: Inject zůstane v Summary přímo pod náhledem obrazovky (jako na terminálu), se stejným Enter checkboxem jako ve Screen.

Rozhodnutí uživatele změněno: nekomplikovat, Inject zůstává jen na záložce Screen, ze Summary odstraněn (commit s tímto záznamem). Test test_table_arrow_keys_select_rows_in_shown_order dřív omylem klikal do prázdného Inject pole v Summary místo vyhledávacího pole (to bylo při šířce 1000 px mimo okno); teď běží na 1600 px a hledá pole podle textu '_keep'. Otevřené: zda Summary má mít náhled posledních řádků obrazovky (AC #1, #2, #4).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Summary of a running instance ends with the last 8 screen lines (screen_tail, trailing blank lines dropped), loaded and refreshed like the Screen tab (on_selection_changed dispatches Screen for Summary). A stopped instance shows no preview. Inject removed from Summary (user decision), stays on Screen. Tests: test_task_77_67_summary_shows_the_screen_tail; the arrow-key table test now clicks the real search field. Review: agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

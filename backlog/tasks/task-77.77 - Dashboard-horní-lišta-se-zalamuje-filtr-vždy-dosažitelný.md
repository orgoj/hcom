---
id: TASK-77.77
title: 'Dashboard: horní lišta se zalamuje, filtr vždy dosažitelný'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:57:00Z'
updated_date: '2026-11-07T16:16:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 172000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F1, P3, A1, A2, A10). Řada 2 horní lišty je nezalamující ui.horizontal plná už při 1600 px: Running only a Filter table jsou při 1200/1280 px mimo obrazovku, Range řádek také oříznutý. Řešení: řady zalamují po skupinách (skupina se nedělí, oddělovače zůstávají); skupina filtru (vyhledávací pole, Running only, Filter table) jde na začátek své řady; vyhledávací pole má pevnou šířku podle placeholderu. Pole okna je dost široké pro nejdelší tvar; v rámci jednoho dne ukazuje 11-06 10:34 – 12:09. Indikátor načítání sessions odchází z řady 1 do hlavičky Sessions (řada 1 má jen prvky stálé šířky).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při oknu 1024x640 i 1280x800 jsou vyhledávací pole, Running only a Filter table celé viditelné a klikatelné bez maximalizace; žádný prvek horní lišty není oříznutý vpravo (řady se zalamují)
- [x] #2 Vlastní okno ze session v rámci jednoho dne ukazuje v poli okna celý rozsah bez oříznutí (např. 11-06 10:34 – 12:09)
- [x] #3 Při přepnutí agenta drží prvky horní lišty svou x pozici, zatímco se sessions načítají
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Řada 2 horní lišty je horizontal_wrapped se skupinami (wrap_group zalomí celou skupinu podle šířky z minulého snímku), skupina filtru je první, vyhledávací pole má šířku placeholderu, pole okna šířku svého textu, ⏳ hláška je na konci řady 3. Ověřeno testem test_top_bar_wraps_and_keeps_controls_on_screen (1024x640, 1280x800); review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

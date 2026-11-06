---
id: TASK-77.11
title: Animované impulzy zpráv a vizuální indikátory aktivity
status: Done
assignee: []
created_date: 2026-11-01 14:54
updated_date: '2026-11-06T21:47:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: feature
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální oživení 2D dashboardu: animované světelné impulzy (packet pulses) pohybující se po komunikačních hranách při doručení nových zpráv, pulzující záře kolem aktivně pracujících agentů (provádějících nástroje/příkazy) a vizuální indikace chybových/blokovaných stavů.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Nová zpráva přijatá za běhu v okně Live krátce (FLASH_SECS 0,4 s) přebarví svou viditelnou hranu barvou výběru; historie při startu ani okno Fixed nebliká
- [x] #2 Mimo bliknutí nevzniká žádné překreslování navíc (jen jedno request_repaint_after na konec bliknutí)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): AC #3 jen blocked (hcom nemá stav chyba); impulz ~1 s jen pro nové živé zprávy. Otevřené: indikátor aktivních agentů — uživatel navrhuje šetrnější blikání změnou barvy místo plynulé záře.

Uživatel (2026-11-06): stačí jedno krátké bliknutí (změna barvy) na zprávu; je to pokus, s impulzy po hraně, září aktivních a indikátorem blocked si případně pohrajeme později. Rozsah zúžen na bliknutí hrany, ostatní AC odstraněna.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Zúženo uživatelem na jedno bliknutí: v okně Live hrana s novou zprávou na 0,4 s v barvě výběru (CanvasView::flash_new_messages při výměně grafu, flash_until, jedno request_repaint_after). Ověřeno test_task_77_11_new_live_messages_flash_their_edge_once; fmt, clippy 0 warnings, cargo test 17+184. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

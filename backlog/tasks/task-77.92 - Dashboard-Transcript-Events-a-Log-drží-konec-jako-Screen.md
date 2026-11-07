---
id: TASK-77.92
title: 'Dashboard: Transcript, Events a Log drží konec jako Screen'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T19:39:00Z'
updated_date: '2026-11-07T19:42:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 188000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: rostoucí záložky (Transcript, Events, Log) mají scrollovat na konec jako Screen, aby při práci agenta bylo vidět, že něco běží a jaká je poslední zpráva; na začátek se uživatel vrátí sám. Fakta: Screen používá ScrollArea::stick_to_bottom(true) (app.rs render_screen_tab); Transcript (hcom transcript), Events (hcom events --last N, nejstarší první) a Log (append) mají nejnovější dole a stick_to_bottom nemají. Sessions je řazený nejnovější nahoře, takže konec = horní okraj; mimo rozsah (beze změny).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Transcript, Events a Log se při otevření zobrazí na konci obsahu a při novém obsahu zůstanou na konci, stejně jako Screen
- [x] #2 Když uživatel odscrolluje nahoru, nový obsah ho nevrací na konec; po návratu na konec se konec znovu drží
- [x] #3 Headless test pokrývá všechny tři záložky; README popisuje chování
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Transcript, Events and Log ScrollAreas got stick_to_bottom(true) like Screen; test_task_77_92_growing_tabs_stick_to_bottom covers open at end, new content at end, scrolled up stays, back at end sticks again, for all three tabs; README Layout paragraph. Small change, no review.
<!-- SECTION:FINAL_SUMMARY:END -->

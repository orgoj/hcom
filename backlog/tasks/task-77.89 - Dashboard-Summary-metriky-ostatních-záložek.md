---
id: TASK-77.89
title: 'Dashboard Summary: metriky ostatních záložek'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T17:53:00Z'
updated_date: '2026-11-07T18:09:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 184000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: Summary má ukázat souhrn ostatních záložek najednou: Screen (počet řádků, zda Summary ukazuje celou obrazovku), Transcript (počet výměn / délka, zda je oříznutý), Sessions (v okně / celkem), Events (počet), Log (počet řádků, chyby). Rozhodnutí: při výběru agenta na Summary načíst na pozadí i transcript, sessions a events (dnes Summary načítá jen Screen, ostatní záložky až při otevření: on_selection_changed v app.rs); hodnoty se doplňují postupně, dokud nejsou načtené ukazují '…'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Summary ukazuje řádek metrik pro Screen, Transcript, Sessions, Events a Log vybraného agenta
- [x] #2 Při výběru agenta na Summary se data Transcriptu, Sessions a Events načtou na pozadí; nenačtená hodnota ukazuje …
- [x] #3 Metriky se aktualizují s obnovou dat a po změně časového okna (Sessions v okně)
- [x] #4 README popisuje metriky v Summary
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Summary shows a metrics grid under the screen tail (Screen lines/all shown, Transcript exchanges·KB·truncated, Sessions in window/total via sessions_header_text, Events count, Log lines/errors). On Summary, on_selection_changed also dispatches Sessions (stopped agents too), Transcript and Events; table refresh re-runs it; unloaded values show …, stopped agents — (stopped). Verified with test_task_77_89_summary_metric_texts and test_task_77_89_summary_loads_tabs_and_shows_metrics (dispatch, …, values, window change, rendering, stopped agent); measured the background commands under 0.2 s; fmt, clippy 0 warnings, 236+17 tests; review OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

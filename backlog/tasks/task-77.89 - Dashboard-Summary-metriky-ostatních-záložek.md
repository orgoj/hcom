---
id: TASK-77.89
title: 'Dashboard Summary: metriky ostatních záložek'
status: To Do
assignee: []
created_date: '2026-11-07T17:53:00Z'
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
- [ ] #1 Summary ukazuje řádek metrik pro Screen, Transcript, Sessions, Events a Log vybraného agenta
- [ ] #2 Při výběru agenta na Summary se data Transcriptu, Sessions a Events načtou na pozadí; nenačtená hodnota ukazuje …
- [ ] #3 Metriky se aktualizují s obnovou dat a po změně časového okna (Sessions v okně)
- [ ] #4 README popisuje metriky v Summary
<!-- AC:END -->

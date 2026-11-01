---
id: TASK-77.9
title: Konfigurovatelný hover popup a Rich Tooltip agenta
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 14:53'
updated_date: '2026-11-01 18:51'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Při najetí myší (hover) na uzel agenta zobrazit plovoucí informační kartu / Rich Tooltip s klíčovými metrikami (PID, CLI, model, reasoning effort, cwd/projekt, věk, unread zprávy, zaplnění kontextu, aktuální stav a detail). Konfigurace zobrazovaných polí v JSON souboru (např. ~/.hcom/graph.json), obdobně jako u Report tabu v hcom-gui.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Najetí myší na uzel agenta otevře přehledný plovoucí popup / tooltip s metrikami agenta
- [x] #2 Tooltip se nezasekává, respektuje hranice okna plátna a plynule mizí při opuštění uzlu
- [x] #3 Tooltip nabízí stejná pole jako postranní panel (Report: všechny klíče Report včetně status:*); výchozí podmnožina: status, detail, cli, since, age, dir, unread, context
- [x] #4 Zobrazovaná pole a jejich pořadí se nastavují samostatným klíčem tooltip v graph.json/gui.json, nezávisle na report; prázdný seznam tooltip vypne
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele: PID se nedoplňuje (hcom list --json ho nemá); tooltip má stejné možnosti jako postranní panel (Report), vlastní config a podmnožinu polí.

Review agent_review: plán OK (6 poznámek zapracováno), diff APPROVED; nit max_col_width(280) zapracován. Odchylky: Id::unique (Id::new deprecated v egui 0.37), is_dragging se negatuje (po uvolnění zůstává true), drag pokrývá pointer_down + dragged_node.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Hover tooltip uzlu v hcom-graph: Area (Order::Tooltip) u kurzoru, constrain_to plátna, fade 0.15 s, okamžité přepnutí mezi uzly; skryje se při tažení, stisku tlačítka, otevřeném popup menu, dialogu či kontextovém menu. Pole z nového klíče config tooltip (stejné klíče jako report vč. status:*, výchozí status/detail/cli/since/age/dir/unread/context, [] vypne); prázdné hodnoty vynechány, víceřádkové zkráceny na 1. řádek s …; placeholder uzel jen jméno. PID vypuštěn rozhodnutím uživatele. Ověřeno: cargo test (80 passed), fmt --check, clippy --all-targets 0 warnings; README doplněn.
<!-- SECTION:FINAL_SUMMARY:END -->

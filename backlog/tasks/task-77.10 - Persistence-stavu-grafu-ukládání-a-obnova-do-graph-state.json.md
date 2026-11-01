---
id: TASK-77.10
title: 'Persistence stavu grafu: ukládání a obnova do graph-state.json'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 14:54'
updated_date: '2026-11-01 18:58'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implementace ukládání a obnovy stavu aplikace do ~/.hcom/graph-state.json: pozice ručně pinovaných a přesunutých uzlů, zoom, posun kamery (pan), šířka a zobrazení postranního panelu, aktivní filtry a zvolený režim layoutu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pozice pinovaných a ručně přesunutých uzlů se ukládají a obnovují po restartu aplikace
- [x] #2 Zvolený zoom, pozice kamery (pan) a šířka/viditelnost postranního panelu se persistují
- [x] #3 Stav se ukládá do ~/.hcom/graph-state.json bez kolize s konfigurací nebo hcom-gui
- [x] #4 Poškozený nebo nevalidní graph-state.json nezpůsobí pád a bezpečně použije výchozí hodnoty
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Pin v aplikaci neexistuje (pinned jen během tažení), AC1 = ručně přetažené uzly: ukládají se po puštění (jen při pohybu > 3 px) podle jména, cap 500 nejnovějších; Manual = finální pozice, Cluster = start, Hierarchy ignoruje. Review agent_review: plán OK po bodu 1 (sanitizace i před uložením, tolerantní záznamy), diff APPROVED; nity tolerance (approx_eq) a is_dragging zapracovány.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
graph-state.json nově drží pozice přetažených uzlů (node_positions s saved_at, max 500), zoom, pan a šířku postranního panelu; zoom/pan/šířka se ukládají 1 s po poslední změně a při on_exit. Stav se sanitizuje při načtení i před uložením (NaN, zoom mimo ZOOM_MIN..MAX, šířka < 240), vadný záznam pozice se přeskočí, poškozený soubor = výchozí hodnoty; temp soubor je unikátní (pid.seq), gui-state.json se nezapisuje. Ověřeno: cargo test (87 passed), fmt --check, clippy 0 warnings; README doplněn.
<!-- SECTION:FINAL_SUMMARY:END -->

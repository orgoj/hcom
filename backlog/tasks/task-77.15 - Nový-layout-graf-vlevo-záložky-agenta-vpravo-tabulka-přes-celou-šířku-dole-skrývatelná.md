---
id: TASK-77.15
title: >-
  Nový layout: graf vlevo, záložky agenta vpravo, tabulka přes celou šířku dole
  (skrývatelná)
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:39'
updated_date: '2026-11-01 19:56'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review (F3): spodní inspect panel zabírá místo i prázdný, tabulka ukáže jen ~5 řádků. Rozhodnutí uživatele: dole přes celou šířku jen tabulka agentů (skrývatelná); nad ní vlevo graf, vpravo panel se všemi záložkami zvoleného agenta (Summary, Report, Screen, Transcript, Events, Log). Nahrazuje dnešní pravý panel detailu i spodní inspect panel. Mimo rozsah: persistence proporcí splitů (samostatný úkol).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Spodní oblast přes celou šířku okna obsahuje jen tabulku agentů
- [x] #2 Tabulku lze skrýt a znovu zobrazit; skrytá tabulka uvolní místo grafu a panelu záložek
- [x] #3 Nad tabulkou je vlevo graf a vpravo jeden panel se záložkami Summary, Report, Screen, Transcript, Events a Log pro zvoleného agenta
- [x] #4 Dosavadní spodní inspect panel a samostatný pravý panel detailu už neexistují; F-klávesy záložek přepínají záložky pravého panelu
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review agent_review: plan OK (notes 1-7 applied), diff APPROVED; nit applied: table height save debounced via mark_view_dirty. Deviations: table ScrollArea fills the panel (otherwise it shrank to its rows), Summary tab scrolls (otherwise it overflowed over the table).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bottom panel holds only the full-width agent table (Table (F4) toggle, height clamped to keep 150 px of canvas); above it the canvas and one right detail panel with Summary/Report/Screen/Transcript/Events/Log (F5-F9 select and reopen). view_mode/main_split_ratio and the inspect panel removed; legacy bottom_tab maps to detail_tab. Verified: headless layout tests (App::ui, panel rects), state migration tests, fmt, clippy 0, 91 tests.
<!-- SECTION:FINAL_SUMMARY:END -->

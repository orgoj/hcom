---
id: TASK-77.93
title: 'Dashboard: horní bar odlišuje popisky sekcí barvou'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T19:49:00Z'
updated_date: '2026-11-07T19:51:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 189000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: v horním baru splývají popisky sekcí (View, Layout, Edges...) s klikacími prvky, vše šedočerné, není poznat, co k sobě patří. Rozhodnutí uživatele: jen barevný popisek (tučně, barva zvýraznění, velkými písmeny bez dvojtečky), oddělovače zůstanou; bez rámečků. Popisky: Filter, View, Layout, Edges, Width (app.rs top bar row 2), Window, Last (render_window_bar).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Headless test ověřuje text a barvu popisků v obou motivech; README popisuje styl
- [x] #2 Popisky Filter, View, Layout, Edges, Width, Window a Last se v horním baru zobrazují velkými písmeny bez dvojtečky v modré barvě zvýraznění, čitelné ve světlém i tmavém motivu; klikací prvky a oddělovače beze změny
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Tučné písmo vypuštěno: výchozí egui font nemá bold řez a RichText::strong() jen mění barvu, kterou explicitní barva přepíše; odlišení dává barva a velká písmena.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Top bar section labels (Filter, View, Layout, Edges, Width, Window, Last) render via section_label(): uppercase, no colon, accent blue (light 30,90,190 / dark 120,170,255); separators and controls unchanged. Bold dropped (egui default font has no bold). Test test_task_77_93_top_bar_section_labels_use_accent (both themes), README. Small change, no review.
<!-- SECTION:FINAL_SUMMARY:END -->

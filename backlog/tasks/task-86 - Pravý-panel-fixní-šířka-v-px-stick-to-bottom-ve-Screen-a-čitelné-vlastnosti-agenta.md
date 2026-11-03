---
id: TASK-86
title: >-
  Pravý panel: fixní šířka v px, stick-to-bottom ve Screen a čitelné vlastnosti
  agenta
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-03 18:55'
updated_date: '2026-11-03 20:08'
labels: []
dependencies:
  - TASK-77
priority: medium
type: bug
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: Pravý panel detailu mění šířku při změnách okna a někdy zmizel tak, že nešel otevřít. Šířka musí být stálá (v px). Záložka Screen má být zarovnaná odspodu (poslední řádky), vlastnosti agenta (Report/Summary) nesmí tlačit labely doprava a musí mít viditelné posuvníky (slidery).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Šířka pravého panelu je uložena a držena v pixelech (ne jako proměnný zlomek šířky okna); při změně velikosti okna zůstává šířka panelu stálá a panel nikdy nezkolabuje na nulu ani nezmizí
- [x] #2 V záložce Screen je výstup zarovnán odspodu (stick_to_bottom), takže uživatel po přepnutí i aktualizaci vidí poslední řádky
- [x] #3 V záložkách s vlastnostmi agenta (Report a Summary) jsou labely zarovnané vlevo, hodnoty nepřetlačují layout a ScrollArea má funkční posuvníky pro zobrazení všech údajů
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. config.rs: Nahradit detail_width_frac za detail_width_px v GraphState s výchozí hodnotou 380 px, min 260 px, zachovat zpětnou kompatibilitu s legacy poli.
2. app.rs: Použít detail_width_px přímo pro exact_size pravého panelu, tažení splitu přímo mění px hodnotu; double-click resetuje na výchozí. Pokud je panel zavřený, zachovat tenký proužek / úchyt na pravém okraji okna.
3. app.rs (render_screen_tab): Nastavit stick_to_bottom(true) na ScrollArea Screenu.
4. app.rs (render_side_panel_summary, render_report_view, render_config_tab): Použít ScrollArea::both() s horizontálním scrollem a zalamování textu/ořez pro dlouhé hodnoty, aby dlouhý obsah neroztahoval šířku panelu a labely zůstaly čitelné vlevo.
5. Testy: Přidat headless regresní testy v contrib/hcom-graph/tests/ ověřující neměnnou šířku panelu při výběru agentů s dlouhými hodnotami, persistenci v px a stick_to_bottom.
6. fmt, clippy, testy pro hcom-graph, review agent_review a deploy build.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Stored and rendered detail panel in pixels (detail_width_px with fallback from fractions), stick_to_bottom(true) for Screen terminal lines, left-aligned ScrollArea::both() for properties preventing panel expansion, edge dock handle unconditionally visible when panel is closed, and full test suite passing.
<!-- SECTION:FINAL_SUMMARY:END -->

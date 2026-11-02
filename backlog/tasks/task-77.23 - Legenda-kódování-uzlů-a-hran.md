---
id: TASK-77.23
title: Legenda kódování uzlů a hran
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:40'
updated_date: '2026-11-02 19:46'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F8/S7: barvy prstenců (CLI), vnitřní barva (stav), čárkovaný obrys (roaming), styly hran nejsou bez README čitelné.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Legenda uvádí každou barvu prstence, styl obrysu a styl hrany, které jsou v aktuálním grafu vidět, a nic jiného
- [x] #2 Záložka Legend v pravém detail panelu zobrazí kompaktní legendu (rozhodnutí uživatele místo přepínače v rohu plátna)
- [x] #3 Výběr záložky Legend se ukládá do graph-state.json (detail_tab) a po restartu se obnoví
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Umístění: uživatel zvolil záložku v pravém panelu místo překryvu v rohu plátna; AC #1 a #3 přepsány podle toho.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Záložka Legend v pravém detail panelu (DETAIL_TABS, persistence přes detail_tab). canvas::legend vrací jen kódování viditelná na plátně (uzly v layoutu a filtru, hrany s oběma konci): barvy stavového okraje, CLI prstence seskupené podle barvy, roaming obrys, spawn/message/neaktivní message hrany; swatche kreslí sdílené barvy plátna. Ověřeno: test_legend_lists_only_visible_encodings (mutant bez filtru viditelnosti zachycen), test_legend_tab_is_selected_saved_and_restored (klik, load_state, restart), fmt/clippy/126+17 testů. Plán i diff reviewoval agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

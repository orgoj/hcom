---
id: TASK-77.106
title: 'Dashboard: silná vazba instancí agenta a plán do skutečně ustáleného stavu'
status: Done
assignee: []
created_date: '2026-11-12T14:40:00Z'
updated_date: '2026-11-12T14:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 207000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Instance jednoho catalog agenta se v grafu musí stáhnout k sobě (silná pružina). Plán klidových pozic musí simulovat až do skutečného ustálení (ne zamrznutí chlazením alpha), pak se uzly rychle a plynule přesunou na cíl; po zastavení lehký posun uzlu nesmí rozhýbat celý graf.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Instance téhož agenta skončí v plánu u sebe (vzdálenost blízko kontaktu)
- [x] #2 Plán končí v rovnováze: další simulace z plánovaného stavu uzly téměř nepohne
- [x] #3 Přesun na plánované pozice je rychlejší
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Cluster pružina: k = max(k_msg, 300), klidová délka 2*collision_radius+5.
2. Plán: plnosilná fáze bez chlazení (PLAN_DT 1/30) do ustálení (žádný uzel se nepohnul o 1 px za 60 kroků, strop 3000), pak chladnoucí dojezd od alpha_reheat (relaxace kontaktního tlaku), celkem max 4000 kroků.
3. Rychlejší dojezd: SETTLE_SPEED 230 px/s, max 2 s.
4. Testy: instance u sebe, re-plan z klidu < 3 px, posun o 10 px pohne ostatními < 25 px; testy živé fyziky (chlazení/kolize) běží s dragging = true.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Diff: /home/michael/projects/hcom/tmp/task-77.106.diff, review u agent_review. Starý kód: instance 180 px od sebe, re-plan z klidu posunul uzel o 13 px. Nový: 88 px, posun 10 px pohne ostatními ~2 px. test_overload_10k a test_layout_large_graph_frame_budget padají pod zátěží i na HEAD (NOTES).

Review agent_review: krok uřezaný rozpočtem snímku se nepočítá do okna konvergence (step_complete); plán s trvale uřezanými kroky je omezená aproximace se stropem 3000/4000 kroků, regresní test test_task_77_106_cut_plan_steps_do_not_converge_but_plan_is_bounded.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Instance jednoho agenta drží silná pružina (k>=300, l=85). Plán klidových pozic běží v plné síle do skutečné rovnováhy (okno 60 kompletních kroků, posun < 1 px), pak krátký chladnoucí dojezd; přesun 230 px/s, max 2 s. Re-plan z klidu < 3 px, posun uzlu o 10 px pohne ostatními ~2 px.
<!-- SECTION:FINAL_SUMMARY:END -->

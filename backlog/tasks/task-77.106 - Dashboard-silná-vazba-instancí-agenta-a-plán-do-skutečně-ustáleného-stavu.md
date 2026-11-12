---
id: TASK-77.106
title: 'Dashboard: silná vazba instancí agenta a plán do skutečně ustáleného stavu'
status: Done
assignee: []
created_date: '2026-11-12T14:40:00Z'
updated_date: '2026-11-12T16:14:00Z'
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

Uživatel: vazba v živém grafu pořád volná. Příčina: agent_name instance přichází často s pozdějším snapshotem než uzel (placeholder ze zprávy); structure_key ho neobsahoval, layout se neprobudil a cluster pružiny se nepřepočítaly. Oprava: agent instance je součástí structure_key; klidová délka cluster pružiny = dotyk kroužků (2*(r+1)). Test test_task_77_106_late_agent_name_wakes_layout_and_pulls_instances_together.

Uživatel (2. kolo): instance pořád daleko, graf dlouho stál. Reprodukce na živých datech (193 uzlů, pin wdt_main): instance 145-158 px, plán 7.7 s (2 ms/krok kvůli HashMap v párových smyčkách). Oprava: tuhá vazba instancí (bind_agent_groups, korekce sdílená s kotvou jako u kolize, připnutá kotva se nehýbe), párové smyčky odpuzování a kolizí přes pole (7.7 s -> 0.7 s), plán smí 4x rozpočet snímku, strop plnosilné fáze 1500/2500 kroků. Živá data: instance 80-82 px. Diff k review: /home/michael/projects/hcom/tmp/task-77.106-rigid.diff

Review 2. kolo (agent_review): bez blokujících nálezů.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Instance jednoho agenta drží silná pružina (k>=300, l=85). Plán klidových pozic běží v plné síle do skutečné rovnováhy (okno 60 kompletních kroků, posun < 1 px), pak krátký chladnoucí dojezd; přesun 230 px/s, max 2 s. Re-plan z klidu < 3 px, posun uzlu o 10 px pohne ostatními ~2 px.
<!-- SECTION:FINAL_SUMMARY:END -->

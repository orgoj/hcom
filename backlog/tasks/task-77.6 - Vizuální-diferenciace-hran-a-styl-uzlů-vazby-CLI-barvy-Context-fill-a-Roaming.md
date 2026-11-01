---
id: TASK-77.6
title: >-
  Vizuální diferenciace hran a styl uzlů: vazby, CLI barvy, Context fill a
  Roaming
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 14:53'
updated_date: '2026-11-01 18:15'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozlišení vazeb spuštění (Spawn) a komunikace (Message), barevné kódování uzlů podle typu CLI/tagu, vizualizace zaplnění kontextového okna (liquid fill odspodu) a vizuální odlišení roaming agentů (přerušovaný obrys).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Spawn hrana je vizuálně odlišena od Message hrany (např. přerušovaná čára, specifická barva, šipka rodič->potomek)
- [x] #2 Uzel agenta má barevný akcent / prstenec odpovídající CLI (Claude, Codex, Antigravity, Hermes atd.)
- [x] #3 Kruh agenta zobrazuje zaplnění kontextového okna vyplněním odspodu (liquid/gauge fill) s barevným prahem
- [x] #4 Roaming agenti mají zřetelně odlišený obrys (např. přerušovaný outline / dvojitý kruh)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Roaming a context fill jsou v AgentGraph::node_styles (z table řádků, přenášeno přes sync výměnu grafu). Context % se parsuje z context_fill ve formátu hcom-gui ('123k 45%'); sběr dat zatím chybí (TASK-77.12), do té doby se fill nevykreslí. Testy: 5 nových v model_tests.rs, 60/60 ok, fmt+clippy čisté. Review: agent_review (čeká).

Review agent_review: APPROVED. Opraveny nity: hrany začínají/končí za vnějšími prstenci (edge_gap +13, sdíleno s hit testem). Nit o blízkosti Claude prstence a oranžového fillu ponechán (fill je poloprůhledný uvnitř kruhu, prstenec vně).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spawn hrany přerušované se šipkou rodič->potomek, CLI barevný prstenec, context fill odspodu s prahy 70/85 %, přerušovaný obrys roaming agentů; roaming/context v AgentGraph::node_styles z table řádků. Ověřeno 5 novými testy (60/60), fmt, clippy 0 varování.
<!-- SECTION:FINAL_SUMMARY:END -->

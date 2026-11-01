---
id: TASK-77.8
title: 'Inteligentní automatický layout grafu: hierarchie, shlukování a pokročilé síly'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 14:53'
updated_date: '2026-11-01 18:40'
labels: []
dependencies: []
references:
  - /home/michael/wiki/2-Knowledge/Research/2d-layout-agentni-graf-hcom-graph.md
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nahrazení statického kruhového uspořádání inteligentním layoutem. Výzkum a implementace dynamického hybridního uspořádání (inspirováno prací Jeffreyho Emanuela): hierarchický strom pro vazby spuštění (rodič->potomek), komunikační přitažlivé síly zpráv shlukující aktivní agenty, Barnes-Hut aproximace pro O(N log N) repulzi, collision radius proti překryvu a přepínatelné režimy uspořádání.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Layout eliminuje výchozí fixní kruhové rozmístění a inteligentně rozmísťuje uzly v ploše
- [x] #2 Hierarchický layout (strom) přehledně zobrazuje vazby spuštění od kořenových agentů
- [x] #3 Silový layout (force-directed) shlukuje uzly s intenzivní vzájemnou komunikací
- [x] #4 Ochrana proti překryvu (collision radius) garantuje čitelnost popisků a kruhů bez překrývání
- [x] #5 Možnost přepínat režim layoutu (Hierarchie / Silový shlukový / Ruční)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Plán i diff schválil agent_review. Barnes-Hut vynechán (AC ho nežádají, desítky uzlů, repulze je časově dělená pod 2 ms). Odchylky: přeškálované konstanty sil (původní síly uzly reálně nehýbaly) a uspání až po 20 klidných krocích. Opraveny nity: klíč pružin zahrnuje strukturu spawn hran (přepojení rodiče bez nové události), invalidace cache layoutu při výměně grafu. Ponecháno: kolize zpracuje minimálně 64 párů i při vyčerpaném rozpočtu, aby nevyhladověla. Pozice v režimu Manual se neukládají (TASK-77.10).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Layout hcom-graph s režimy Hierarchy (strom spuštění po vrstvách), Cluster (silový, pružiny ln(1+count) agregované po páru) a Manual (jen drag), přepínač v liště ukládaný do graph-state.json; nové uzly u rodiče/partnera nebo na spirále; kolizní poloměr s vlastním kurzorem a rozpočtem. Ověřeno 10 novými testy (77/77), fmt, clippy 0 varování.
<!-- SECTION:FINAL_SUMMARY:END -->

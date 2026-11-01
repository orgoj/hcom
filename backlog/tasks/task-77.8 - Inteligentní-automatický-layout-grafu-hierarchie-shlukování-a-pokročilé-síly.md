---
id: TASK-77.8
title: 'Inteligentní automatický layout grafu: hierarchie, shlukování a pokročilé síly'
status: To Do
assignee: []
created_date: '2026-11-01 14:53'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nahrazení statického kruhového uspořádání inteligentním layoutem. Výzkum a implementace dynamického hybridního uspořádání (inspirováno prací Jeffreyho Emanuela): hierarchický strom pro vazby spuštění (rodič->potomek), komunikační přitažlivé síly zpráv shlukující aktivní agenty, Barnes-Hut aproximace pro O(N log N) repulzi, collision radius proti překryvu a přepínatelné režimy uspořádání.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Layout eliminuje výchozí fixní kruhové rozmístění a inteligentně rozmísťuje uzly v ploše
- [ ] #2 Hierarchický layout (strom) přehledně zobrazuje vazby spuštění od kořenových agentů
- [ ] #3 Silový layout (force-directed) shlukuje uzly s intenzivní vzájemnou komunikací
- [ ] #4 Ochrana proti překryvu (collision radius) garantuje čitelnost popisků a kruhů bez překrývání
- [ ] #5 Možnost přepínat režim layoutu (Hierarchie / Silový shlukový / Ruční)
<!-- AC:END -->

---
id: TASK-77.22
title: Umístění hover tooltipu mimo hovered uzel
status: To Do
assignee: []
created_date: '2026-11-01 19:40'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F6/S6: u okraje plátna se tooltip (TASK-77.9) posune přes uzel, který popisuje; šířka neodpovídá obsahu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tooltip zkouší pozice vpravo, vlevo, pod a nad uzlem a použije první, kde se vejde do plátna bez překrytí kruhu hovered uzlu
- [ ] #2 Uzel do 150 px od okraje plátna není tooltipem zakryt
- [ ] #3 Šířka tooltipu se řídí obsahem do maxima; dir zkracuje domovský adresář na ~ a dlouhé cesty elipsou uprostřed
<!-- AC:END -->

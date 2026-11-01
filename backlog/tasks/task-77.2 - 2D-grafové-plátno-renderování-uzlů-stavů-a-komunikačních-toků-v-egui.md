---
id: TASK-77.2
title: '2D grafové plátno: renderování uzlů, stavů a komunikačních toků v egui'
status: To Do
assignee: []
created_date: '2026-11-01 11:21'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální komponenta v egui/eframe s 2D force-directed nebo pružinovým rozložením. Zobrazuje agenty jako uzly obarvené podle stavu, animované/orientované šipky zpráv a stromové vazby spuštění.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Uzly jsou vykresleny s názvy a barvou odpovídající stavu (active, listening, blocked, stopped)
- [ ] #2 Komunikační hrany zobrazují směr a počet zpráv mezi agenty
- [ ] #3 Podpora zoomu (kolečko myši) a posunu plátna (pan)
- [ ] #4 Uzel lze myší uchopit a přesunout (drag & drop pozicování)
<!-- AC:END -->

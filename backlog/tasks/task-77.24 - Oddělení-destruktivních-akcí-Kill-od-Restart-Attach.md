---
id: TASK-77.24
title: Oddělení destruktivních akcí Kill od Restart/Attach
status: To Do
assignee: []
created_date: '2026-11-01 19:40'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F9/S8: Kill stojí hned vedle Restart. Potvrzovací dialogy Kill i Kill group už existují.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 V toolbaru tvoří Kill a Kill group samostatnou skupinu na konci řádku, oddělenou separátorem od Restart a Attach
- [ ] #2 Kill group je neaktivní, když jeho menu nemá na co působit
- [ ] #3 V panelu agenta je Kill oddělený mezerou od Attach a Transcript a stojí poslední
<!-- AC:END -->

---
id: TASK-45
title: 'hcom-gui: filter to hide non-persistent hcom agents'
status: To Do
assignee: []
created_date: '2026-10-26 13:40'
labels: []
dependencies: []
type: feature
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User request: a filter in hcom-gui that hides non-persistent hcom agents (instances not backed by a persistent catalog agent; currently only kato).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A filter toggle in hcom-gui hides non-persistent hcom agent rows
- [ ] #2 Persistent (catalog) agents stay visible when the filter is on; with the filter off all agents show
- [ ] #3 The filter combines with the other filters and the displayed-agent count respects it
<!-- AC:END -->

---
id: TASK-42.2
title: 'hcom-gui statistics: archived databases and a rolling window'
status: Idea
assignee: []
created_date: '2026-10-23 14:07'
labels: []
dependencies: []
parent_task_id: TASK-42
priority: low
type: feature
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Known facts: TASK-42 computes agent statistics only from the live database, so `hcom reset` (which archives and clears it) resets them. The user wants optional aggregation over the live DB plus archives, and over a configurable rolling window of N days (gui.json).

Suggested direction: read archived databases through an existing hcom command (contrib must only call the hcom CLI), add a period selector or gui.json setting. Needs analysis before implementation: archive format and the CLI path to query it, cost of reading all archives, whether the period is a gui.json setting or a UI control.
<!-- SECTION:DESCRIPTION:END -->

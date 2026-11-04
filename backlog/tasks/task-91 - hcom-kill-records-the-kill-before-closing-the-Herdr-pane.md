---
id: TASK-91
title: hcom kill records the kill before closing the Herdr pane
status: Idea
assignee: []
created_date: '2026-11-04 08:59'
labels: []
dependencies: []
type: bug
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A Herdr restart popup ran `hcom agent knowledge --as knowledge --restart`; its kill closed the knowledge pane and workspace, Herdr then killed the popup's processes too, so the restart died before relaunching. hcom.log and events showed only 'stopped by:system process_exit', no kill or launch, so the interrupted kill left no trace (reported by herdr_dev). Suggested direction: `hcom kill` logs/records the kill (life event, log line) before closing the pane. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

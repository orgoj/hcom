---
id: TASK-45
title: 'hcom-gui: filter to hide non-persistent hcom agents'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-26 13:40'
updated_date: '2026-10-26 13:44'
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
- [x] #1 A filter toggle in hcom-gui hides non-persistent hcom agent rows
- [x] #2 Persistent (catalog) agents stay visible when the filter is on; with the filter off all agents show
- [x] #3 The filter combines with the other filters and the displayed-agent count respects it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Checkbox 'Hide non-persistent' (default off, persisted as hide_adhoc in gui-state.json) hides instance rows without a catalog agent; combines with other filters and the title count. Verified with test_hide_non_persistent_hides_instances_without_catalog_agent_and_is_restored.
<!-- SECTION:FINAL_SUMMARY:END -->

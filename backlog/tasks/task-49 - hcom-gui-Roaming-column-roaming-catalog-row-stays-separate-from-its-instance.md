---
id: TASK-49
title: 'hcom-gui: Roaming column, roaming catalog row stays separate from its instance'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 05:20'
updated_date: '2026-10-27 06:09'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User request. hcom-gui needs a Roaming column that marks roaming catalog agents and their running instances (instance rows link to their catalog agent). When exactly one roaming instance runs, its row and the roaming catalog row must stay two separate rows: the catalog row is the archetype for other projects. Today build_rows keeps them separate only because the instance name differs from the catalog name; the column must not change that. Out of scope: binary changes (TASK-48 covers CLI listings).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Roaming column exists, can be shown/hidden like other columns, and marks the roaming catalog row and each running instance of a roaming agent; non-roaming rows stay empty
- [x] #2 With exactly one running roaming instance, the GUI shows two rows: the instance row and the catalog row (status 'running as <instance>')
- [x] #3 contrib/test-hcom-gui covers both cases and passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom-gui has a Roaming column (after Instance, ↻ on roaming catalog rows and their instance rows, hideable via Columns… like any column; saved choices gain it via with_new_columns). A roaming catalog row is never merged into an instance row, also when an instance holds the catalog name; it shows 'running as <instance>'. Verified with contrib/test-hcom-gui (118 OK): test_roaming_column_marks_roaming_agent_and_its_instances, test_one_running_roaming_instance_keeps_its_catalog_row, updated legacy column tests. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

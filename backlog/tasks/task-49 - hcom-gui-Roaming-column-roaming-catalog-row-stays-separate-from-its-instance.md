---
id: TASK-49
title: 'hcom-gui: Roaming column, roaming catalog row stays separate from its instance'
status: To Do
assignee: []
created_date: '2026-10-27 05:20'
updated_date: '2026-10-27 05:20'
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
- [ ] #1 A Roaming column exists, can be shown/hidden like other columns, and marks the roaming catalog row and each running instance of a roaming agent; non-roaming rows stay empty
- [ ] #2 With exactly one running roaming instance, the GUI shows two rows: the instance row and the catalog row (status 'running as <instance>')
- [ ] #3 contrib/test-hcom-gui covers both cases and passes
<!-- AC:END -->

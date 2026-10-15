---
id: TASK-7
title: 'hcom-gui: show status age as its own sortable column'
status: To Do
assignee: []
created_date: '2026-10-15 09:46'
labels: []
dependencies: []
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom list --json gives status_age_seconds; the GUI folds it into the Status text ('listening 3h'), so rows cannot be sorted by how long an agent has been idle or blocked. Add a separate Since column.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Since column shows the status age (e.g. 3h, 10m) for running instances and is empty for catalog-only rows
- [ ] #2 Sorting by Since orders by the numeric age, not the text
- [ ] #3 contrib/test-hcom-gui covers the column
<!-- AC:END -->

---
id: TASK-106
title: 'Review rejected: AGY mode-status suffix may hide a real draft'
status: Done
assignee: []
created_date: '2026-11-13T08:19:00Z'
labels:
  - review:rejected
dependencies: []
ordinal: 214000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
agent_review_hcom_884 on TASK-105: a real draft ending in '(shift+tab to cycle)' with the cursor moved to the start (Home) is read as an empty prompt. Rejected: it needs a user draft that ends exactly in agy's own status suffix and a cursor parked at the start; agents never type it. The exact-string version (TASK-104) had the same failure mode for those texts. The status cells carry no discriminating attribute the parser can rely on (they are not dim, otherwise is_dim_after_prompt would already have handled them).
<!-- SECTION:DESCRIPTION:END -->

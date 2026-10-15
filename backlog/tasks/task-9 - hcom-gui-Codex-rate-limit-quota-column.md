---
id: TASK-9
title: 'hcom-gui: Codex rate-limit quota column'
status: To Do
assignee: []
created_date: '2026-10-15 09:46'
labels: []
dependencies: []
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Codex rollout token_count events carry rate_limits.primary.used_percent and window_minutes (the 5h quota). Show it per Codex instance so the user sees quota exhaustion before an agent stalls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Quota column shows the latest primary used_percent for running Codex instances
- [ ] #2 Non-Codex rows and transcripts without rate_limits show an empty value
- [ ] #3 contrib/test-hcom-gui covers the column
<!-- AC:END -->

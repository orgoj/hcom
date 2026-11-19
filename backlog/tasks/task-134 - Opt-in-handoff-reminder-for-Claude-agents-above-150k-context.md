---
id: TASK-134
title: Opt-in handoff reminder for Claude agents above 150k context
status: To Do
assignee: []
created_date: '2026-11-19T20:25:00Z'
labels: []
dependencies: []
priority: medium
ordinal: 245000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Request from agent_coach (user order) 2026-11-19, record only, not implemented yet. Opt-in hook for Claude agents only (codex/agy out of scope: their long-context price limits are unknown). When a Claude agent ends a turn with context above 150k tokens, inject a reminder to run hcom-handoff before starting its next task (never mid-task). Existing behavior unchanged by default (opt-in config key). Evidence: cca_dev day review 2026-11-19, session 1 ran 3.5 h past 150k, 75% of the day cache_read (/home/michael/.hcom/tmp/session-reviews/cca_dev-day-2026-11-19-20261119-205948-298648.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Off by default; with the opt-in enabled, a Claude agent whose context exceeds the threshold (default 150k tokens) at turn end gets one reminder to run hcom-handoff before its next task
- [ ] #2 No reminder for other CLIs (codex, agy, ...) and none injected mid-task
- [ ] #3 Help, README, docs and the hcom-agent-messaging skill describe the option; tests cover on/off and the threshold
<!-- AC:END -->

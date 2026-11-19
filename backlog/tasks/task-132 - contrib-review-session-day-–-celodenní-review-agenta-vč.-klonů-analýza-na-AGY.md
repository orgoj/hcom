---
id: TASK-132
title: >-
  contrib: review-session --day – celodenní review agenta vč. klonů, analýza na
  AGY
status: Done
assignee: []
created_date: '2026-11-19T19:56:00Z'
updated_date: '2026-11-19T20:07:00Z'
labels:
  - feature
dependencies: []
priority: medium
ordinal: 243000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Request agent_coach #403011 (user approved), brief ~/.hcom/tmp/review-session-day-brief.md. Opt-in --day YYYY-MM-DD: all sessions of NAME and NAME_* clones (stopped snapshots + running) in that local day, detailed transcripts, hcom messages, token totals from transcript usage records (size estimate fallback), timeline (prompts, gaps, blocked, messages). --cli agy (default model gemini-4-neon-medium). Default behavior unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Without --day behavior and output are unchanged
- [x] #2 --day collects stopped and running sessions of NAME and NAME_* in the local day
- [x] #3 Dump has token totals per session and period, timeline and detailed transcripts
- [x] #4 --cli agy runs agy -p in the report dir with model gemini-4-neon-medium
- [x] #5 Prompt asks where time/tokens were lost and which SOUL/NOTES/MEMORY lines cause it; report saved under session-reviews, coach gets path
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-19: implemented, tests 17 OK, real AGY run on cca_dev 2026-11-19 OK (4 sessions, dump 608 KB, ~7 min). Diff tmp/review-session-day.diff sent to agent_review; commit after its OK, then reply to agent_coach #403011 with report path and command.

agent_review OK; fixed DST day end and per-instance gap state; agent_security suggestion rejected (separate review:rejected task).
<!-- SECTION:NOTES:END -->

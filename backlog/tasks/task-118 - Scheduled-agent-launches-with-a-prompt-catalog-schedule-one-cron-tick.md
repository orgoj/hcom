---
id: TASK-118
title: Scheduled agent launches with a prompt (catalog schedule + one cron tick)
status: Idea
assignee: []
created_date: '2026-11-18T17:07:00Z'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 229000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User need (2026-11-18): launch knowledge as knowledge_vulncheck several times a day with a prompt to check current vulnerabilities (it has its own procedure and scripts), ideally configured from hcom-dashboard. No scheduler exists in hcom or upstream (checked git log upstream/main and src). Works today without code: a cron line 'hcom agent knowledge --as knowledge_vulncheck --hcom-prompt <text>' (or a catalog entry knowledge_vulncheck with prompt + idle_kill so the instance ends after the run; a launch while the instance still runs is a no-op and the prompt is lost). Unverified: launching from cron into Herdr (cron env: PATH from mise, Herdr socket). Suggested direction: optional 'schedule' key (cron expressions) in a catalog agent entry; one user crontab line runs 'hcom schedule tick' every minute, which launches due agents with their prompt, logs a life event per run and skips (logs) a run whose instance still runs; no daemon. Dashboard shows schedules, last and next run; editing from the dashboard is an open decision (catalog is user-edited config). Open questions: catalog key vs separate schedules file, behavior when the previous run still runs (skip, send the prompt to it, or --restart), whether the dashboard edits schedules. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

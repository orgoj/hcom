---
id: TASK-76
title: 'AGENTS.md: note cli_smoke Codex timeouts under parallelism'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 10:44'
updated_date: '2026-11-01 10:44'
labels: []
dependencies: []
priority: low
type: docs
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cli_smoke roaming/send cases failed with a Codex app-server timeout under default parallelism and passed with --test-threads=4 (TASK-75 session); agent_coach asked to record it in the repo guide.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AGENTS.md Commands names the symptom and the --test-threads=4 rerun
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the symptom and the --test-threads=4 rerun to AGENTS.md Commands.
<!-- SECTION:FINAL_SUMMARY:END -->

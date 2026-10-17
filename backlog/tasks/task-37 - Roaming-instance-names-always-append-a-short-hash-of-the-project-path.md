---
id: TASK-37
title: 'Roaming instance names: always append a short hash of the project path'
status: Idea
assignee: []
created_date: '2026-10-17 10:54'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Roaming instance name is <agent>_<slug>, where roaming_project_slug (src/commands/agent.rs) keeps only the project root basename, lowercased, non-alphanumerics folded to _. Different roots with the same basename collide: ~/.hcom and ~/projects/hcom both give agent_review_hcom, so roaming_route refuses 'project directory basenames must be unique' (reported by agent_coach, msg #172496, sending @agent_review from ~/.hcom). User decision: ALWAYS append a short hex hash of the canonical project root path, e.g. agent_review_hcom_3f2a (deterministic per directory, across machines and DB resets; readable basename kept). Length 4-6 hex still to confirm (4 hex ~ 1:65k per same-basename pair); keep the existing collision check as a safety net. Touches roaming_project_slug, retarget_roaming_route (recomputes the expected name), roaming tests in tests/cli_smoke.rs and unit tests, agent help (Roaming agents), send help (roaming line in src/commands/help.rs), README Roaming section, skills/hcom-agent-messaging. Existing roaming instances (old names without hash) will no longer be selected; after the fix is installed, kill all old-name roaming instances (user request). Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

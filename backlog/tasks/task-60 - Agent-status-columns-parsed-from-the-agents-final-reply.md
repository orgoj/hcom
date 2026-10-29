---
id: TASK-60
title: Agent status columns parsed from the agent's final reply
status: Idea
assignee: []
created_date: '2026-10-29 05:22'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents are prompted to end every turn with a fixed block: 'STATUS: DONE | DONE_FOLLOWUP | NEEDS_USER | WAITING', 'Done:', 'Issues:', 'Next:'. Idea (user): configurable columns, each defined in config by a name and a regexp that extracts its value from the agent's last reply; shown in hcom (and hcom-gui) and in 'hcom agent list', e.g. via an option or 'hcom agent list --status', mainly so agent_manager sees who is in which state. Open questions: where the data comes from (stored last-reply text vs transcript parse), when it is extracted (on idle/stop event vs on read), storage (instance row/event field), config shape and placement, scope of 'hcom list' vs 'hcom agent list', JSON output, staleness after a new turn starts, upstream mergeability (fork fallback). Suggested direction: needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

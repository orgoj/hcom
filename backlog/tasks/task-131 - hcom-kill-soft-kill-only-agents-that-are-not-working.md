---
id: TASK-131
title: 'hcom kill --soft: kill only agents that are not working'
status: Done
assignee: []
created_date: '2026-11-19T18:12:00Z'
updated_date: '2026-11-19T18:21:00Z'
labels:
  - feature
dependencies: []
priority: medium
ordinal: 242000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User order 2026-11-19 via agent_coach: opt-in `hcom kill --soft <name>...` stops an agent only when it is idle (listening, no active turn). One result line per name: `not running`, `killed`, or `not killed: working (<status>)`. Plain `hcom kill` unchanged. Why: agents (cca_manager) almost killed a busy shared agent (dippy_dev) after it replied; the tool decides, not agent instructions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 --soft kills a listening local agent and prints '<name>: killed'
- [x] #2 --soft on an active/blocked/launching agent, or a listening one with unread messages or a TUI gate, prints 'not killed: working (<status>)', leaves it running, exits 1
- [x] #3 --soft on an unknown or stopped name prints 'not running'
- [x] #4 --soft accepts only names (no all, @group, tag:, tool:, SELF); plain kill unchanged
- [x] #5 Status check and kill marker in one write transaction
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-19: Implemented in src/commands/kill.rs (kill_soft). Review (agent_review): busy status is now classified before the missing-PID check (launching placeholder -> working (launching)); accepted boundary: a turn starting in the ms between the check transaction and the signal is still killed (documented in code). Remote rows and rows without PID report 'not killed: ...'; exit 1 when any target is spared.
<!-- SECTION:NOTES:END -->

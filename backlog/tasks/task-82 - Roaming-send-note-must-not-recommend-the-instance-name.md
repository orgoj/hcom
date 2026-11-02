---
id: TASK-82
title: Roaming send note must not recommend the instance name
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 19:51'
updated_date: '2026-11-02 20:03'
labels:
  - 'resolution:rejected'
dependencies: []
type: bug
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom send @<role> to a roaming agent prints 'in this project it is <instance> (use that name)'. Agents follow it and address instances, against the rule to address the role. Requested by agent_coach (#279579).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The send note names the instance it was routed to and tells the sender to keep addressing @<role>; it no longer says 'use that name'
- [x] #2 README, docs/agent.md and cli_smoke tests match the new wording
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reverted by user decision: both the role and the instance name are intended addresses; the original note stays.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Send note now reads 'delivered to <instance> in this project. Keep addressing @<role>.'; README/docs/cli_smoke updated. Verified: fmt, clippy, cargo test --bin hcom, cli_smoke roaming (16 passed).
<!-- SECTION:FINAL_SUMMARY:END -->

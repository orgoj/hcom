---
id: TASK-129
title: herdr agent.rename retried every tick for names herdr rejects
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T15:56:00Z'
updated_date: '2026-11-19T15:59:00Z'
labels: []
dependencies: []
type: bug
ordinal: 240000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
HostLabel::sync (src/delivery.rs, fork commit 77fe3d27) calls herdr agent.rename every delivery tick until it succeeds, because before pane classification herdr also rejects it. herdr rejects names that do not start with a lowercase letter or contain other than [a-z0-9-_] or exceed 32 chars ('agent name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (1-32 characters)'). Roaming names like agent_architect_killbottleneck_228 (34) are permanently rejected: 18945 op_rejected lines in one 8 MB hcom.log, rotating the log every ~1.5h. hcom itself targets herdr panes by pane_id (focus, close), so the herdr name only serves manual 'herdr agent send <name>'; no shortened name is stored (user, 2026-11-19).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A name herdr's rule rejects is never sent to agent.rename; hcom logs it once per delivery loop and does not retry
- [x] #2 Valid names keep the existing retry-until-success behavior
- [x] #3 Unit test covers the name rule (34-char name, uppercase, leading digit, valid 4-letter and 32-char names)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
HostLabel::sync skips agent.rename once (log agent_rename_skipped) for names failing herdr's rule (herdr_accepts_agent_name: lowercase start, [a-z0-9_-], <=32); valid names keep retry-until-success. hcom targets herdr panes by pane_id, so no shortened name is stored. Verified: unit tests herdr_agent_name_rule and rejected_agent_name_is_not_sent_to_herdr, cargo test --bin hcom (2936 ok), cli_smoke herdr (5 ok), fmt, clippy. Review OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

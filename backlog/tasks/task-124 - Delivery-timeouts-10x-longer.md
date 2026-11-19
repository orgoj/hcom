---
id: TASK-124
title: Delivery timeouts 10x longer
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T11:19:00Z'
updated_date: '2026-11-19T11:22:00Z'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 235000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User order 2026-11-19 after two agy agents stalled on the 10 s phase-1 render limit (agy renders ~130 chars/s under load): raise the delivery limits at least 10x at once. PHASE1_TIMEOUT 10 s -> 100 s, PHASE2_TIMEOUT 2 s -> 20 s, VERIFY_TIMEOUT 10 s -> 100 s (src/delivery.rs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 PHASE1_TIMEOUT is 100 s, PHASE2_TIMEOUT 20 s, VERIFY_TIMEOUT 100 s
- [x] #2 Comments and tests that name the old values are updated
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
PHASE1_TIMEOUT 10->100 s, PHASE2_TIMEOUT 2->20 s, VERIFY_TIMEOUT 10->100 s; test and agy comment updated. Verified: fmt, clippy, cargo test --bin hcom, cli_smoke antigravity and non_destructive_reset_paths_deliver_pending_messages. Small change, no review (user order).
<!-- SECTION:FINAL_SUMMARY:END -->

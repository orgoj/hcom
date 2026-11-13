---
id: TASK-113
title: >-
  Request watch: a reply to one request must not close the sender's other open
  requests
status: Done
assignee: []
created_date: '2026-11-13T22:41:00Z'
updated_date: '2026-11-13T23:21:00Z'
labels: []
dependencies: []
ordinal: 224000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Case 2026-11-13: nuntora_dev sent two requests to agent_architect_nuntora_435 (agy) within 2 s (#359498 TASK-39, #359501 TASK-40). The agent reviewed only TASK-40, replied once with --reply-to 359498, and ended DONE. reqwatch_reply_exists (same in upstream) treats any later mention-scoped message from the target to the sender as a reply, so one message closed both watches and the sender got no notice. Proposal: a message with reply_to answers only that request; a message without reply_to keeps today's behavior. When the target replies to one request while another from the same sender stays open, notify the sender right away ('X replied to #A; request #B is still unanswered'), because with request_idle_notify=false (this machine) the open watch would otherwise fire only on stop. Upstream logic: keep it a minimal fork hook or offer it as an upstream PR.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. reqwatch_reply_exists: mention-scoped message counts only without reply_to (match live cancel_request_watches_by_flow). 2. notify_request_still_open: one notice when target replies to another event while this request stays open. 3. Regression test (fails without 1). 4. Review, commit, release build. 5. Upstream PR candidate: part 1 only (one-line SQL + test).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause: live cancel path already respects reply_to; only the backstop check at listening/stopped ignored it and silently deleted the open watch. Implemented, gates green, diff tmp/task-113.diff at agent_review.

Review agent_review: agy grace claim DELETE duplicated the old reply predicate; fixed the same way with a regression test. Upstream PR candidate: both SQL predicates + tests, without the open-request notice.
<!-- SECTION:NOTES:END -->

---
id: TASK-126
title: >-
  agy: long message with indented lines is never submitted and is re-typed in a
  loop
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T13:52:00Z'
updated_date: '2026-11-19T13:58:00Z'
labels: []
dependencies: []
type: bug
ordinal: 237000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Since TASK-114, agy delivery types message bodies into the prompt as one line (one_line in src/delivery/antigravity.rs replaces newlines/tabs by spaces, keeps space runs). When agy wraps the prompt right before a run of spaces (e.g. body line '  * Repo: ...'), the continuation row starts with more than two spaces, get_antigravity_input_text (src/pty/screen.rs) treats it as the footer and stops joining. The read-back prompt is then shorter than the injected text, prompt_ownership is never Exclusive, phase 1 times out after 10s and hcom never sends Enter. The user's manual Enter submits it, but the ack is committed only after hcom's own Enter, so the same message is typed again. Seen on agent_architect_killbottleneck_228, message #396820 (1503 chars), inject_attempt 2-4, 2026-11-19. Fix: collapse whitespace runs to single spaces when building the agy prompt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The agy prompt built from a message with newlines, tabs and runs of spaces contains no run of two or more whitespace characters
- [x] #2 A unit test covers a message whose lines are indented by several spaces
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
one_line() in src/delivery/antigravity.rs now collapses every whitespace run to one space, so an agy wrap can no longer start a row the screen parser reads as the footer. Verified with new unit test prompt_collapses_indented_lines_to_single_spaces, cargo test --bin hcom (2934 ok), cli_smoke antigravity, fmt, clippy. Caused by TASK-114 (prompt typing). Not changed: a manual Enter by the user still does not ack the typed prompt.
<!-- SECTION:FINAL_SUMMARY:END -->

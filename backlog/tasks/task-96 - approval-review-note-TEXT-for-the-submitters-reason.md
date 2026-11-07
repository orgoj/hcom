---
id: TASK-96
title: 'approval-review: --note TEXT for the submitter''s reason'
status: Done
assignee: []
created_date: '2026-11-07T19:21:00Z'
updated_date: '2026-11-07T19:22:00Z'
labels: []
dependencies: []
ordinal: 187000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User order via herdr_dev: optional --note TEXT appended to the recipient message as 'Note from the submitter: <text>' inside the untrusted-data framing, and written into the snapshot header. Without --note behavior unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Optional --note TEXT: whitespace-collapsed note in snapshot header (note: line) and message (Note from the submitter, inside untrusted framing, metadata list names it). Test test_note_goes_into_header_and_untrusted_message; README updated.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-93
title: Send error for ack on inform should suggest --intent inform --reply-to
status: Done
assignee: []
created_date: '2026-11-06T08:05:00Z'
updated_date: '2026-11-06T08:28:00Z'
labels: []
dependencies: []
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
send.rs:666 rejects ack on inform without saying what to use; a hint catches agents with an old skill in context. Recommended by skills agent review (TASK-92).
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Error now ends with: To reply with content, use --intent inform --reply-to <id>.
<!-- SECTION:NOTES:END -->

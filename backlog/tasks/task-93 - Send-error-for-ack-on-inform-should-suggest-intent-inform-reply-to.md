---
id: TASK-93
title: Send error for ack on inform should suggest --intent inform --reply-to
status: Idea
assignee: []
created_date: '2026-11-06T08:05:00Z'
labels: []
dependencies: []
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
send.rs:666 rejects ack on inform without saying what to use; a hint catches agents with an old skill in context. Recommended by skills agent review (TASK-92).
<!-- SECTION:DESCRIPTION:END -->

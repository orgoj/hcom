---
id: TASK-92
title: 'Agents ack inform messages: fix skill reply example'
status: Done
assignee: []
created_date: '2026-11-06T08:05:00Z'
updated_date: '2026-11-06T08:05:00Z'
labels: []
dependencies: []
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Skill hcom-agent-messaging shows ack as the only reply form; binary rejects ack on inform/ack parent (send.rs:666) without saying what to use. Fix: skill reply examples per parent intent (reviewed by skills agent), error message suggests --intent inform --reply-to.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Skill fixed per skills agent review (examples keyed to message header [request #id]/[inform #id]). Binary error hint ('use --intent inform --reply-to') left as an Idea, user asked for skill only.
<!-- SECTION:NOTES:END -->

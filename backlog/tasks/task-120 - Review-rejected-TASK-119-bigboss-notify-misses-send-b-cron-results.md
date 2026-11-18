---
id: TASK-120
title: 'Review rejected: TASK-119 bigboss notify misses send -b cron results'
status: Done
assignee: []
created_date: '2026-11-18T17:50:00Z'
labels:
  - review:rejected
dependencies: []
ordinal: 231000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
agent_review (TASK-119) said the motivating cron case never notifies because send -b is --from bigboss and a broadcast. Rejected: the cron message is only the trigger; the result the user must see is knowledge_vulncheck's reply to bigboss, whose event mentions bigboss (event 392368 mentions=[bigboss]), so it notifies. Also not done: agent_security review (internal tool; the command is configured by the user, no shell, values as single argv tokens). Exit status of the detached command: not logged (short hcom send exits first); --info text says so.
<!-- SECTION:DESCRIPTION:END -->

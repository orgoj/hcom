---
id: TASK-5
title: Deliver wdt_main replies into kato's Hermes context
status: To Do
assignee: []
created_date: '2026-10-13 14:50'
labels: []
dependencies: []
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
kato (Hermes gateway, adhoc, ~/.hermes) does not get replies from wdt_main into its context, while fresh test messages (hermes_dev #153581, #154041: intent=request, no thread/reply_to) arrive. Evidence 2026-10-13: wdt_main #155656 (14:41:54, inform, reply_to 155646, thread berta2-disk-space) and #155710 (14:48:02, inform, reply_to 155707, thread claude-remote-control) exist as message events; kato keeps 1 unread; every 60s kato's hcom listen returns with status context 'message received' (src/commands/listen.rs manual-ack/batch path) yet the message never reaches the conversation; kato read #155656 only manually (deliver:wdt_main at 14:44:59). Suspected: the Hermes hcom plugin skips or does not ack envelopes with intent=inform, reply_to or thread (owner hermes_dev, asked), or hcom manual-ack hands the same head envelope out repeatedly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Root cause identified with evidence: hcom listen side or Hermes plugin side
- [ ] #2 A reply (intent=inform with reply_to and thread) sent to kato appears in kato's context without kato reading it manually
- [ ] #3 Regression test covers the reply/inform/thread case on the side that is fixed
<!-- AC:END -->

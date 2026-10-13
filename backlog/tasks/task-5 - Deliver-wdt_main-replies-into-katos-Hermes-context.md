---
id: TASK-5
title: Deliver wdt_main replies into kato's Hermes context
status: To Do
assignee: []
created_date: '2026-10-13 14:50'
updated_date: '2026-10-13 15:07'
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
- [x] #1 Root cause identified with evidence: hcom listen side or Hermes plugin side
- [x] #2 A reply (intent=inform with reply_to and thread) sent to kato appears in kato's context without kato reading it manually
- [ ] #3 Regression test covers the reply/inform/thread case on the side that is fixed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause (hermes_dev #155747): Hermes plugin, not hcom. /home/michael/projects/hermes-hcom-plugin/__init__.py polls 'hcom listen --name kato --json --manual-ack --timeout 86400' and acks only after dispatch (:484). Envelopes with a thread route only via SessionRouteStore (hcom_session_routes.json, :445-450), filled only when kato itself sent with --thread (:232-308). Unknown threads (berta2-disk-space, claude-remote-control) raise 'has no correlated Hermes session' -> no ack -> retry up to 60s -> head-of-line block. Fix (fallback to binding.origin for unknown threads) is owned by hermes_dev; hcom manual-ack behaves as designed.

Fixed by hermes_dev in hermes-hcom-plugin 5272695 (branch fix/thread-session-routing, not pushed): route capture required argv[1]==send while kato sends 'hcom --name kato send --thread X'; now every threaded send form is captured and unknown threads go to the root chat instead of blocking. Gateway restarted (PID 907121); #155815 delivered to kato session :30785 and acked. Regression test in the plugin not yet confirmed. Nothing needed in hcom.
<!-- SECTION:NOTES:END -->

---
id: TASK-119
title: Notify command for messages to bigboss
status: To Do
assignee: []
created_date: '2026-11-18T17:33:00Z'
labels: []
dependencies: []
priority: high
type: feature
ordinal: 230000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User need (2026-11-18): agent results sent to bigboss (e.g. the cron-driven knowledge_vulncheck threat check, sent with send -b) go unnoticed because the user does not watch hcom. Today nothing notifies: hcom only stores the message; events sub delivers only as an hcom message. Decision (user, variant 2): an optional hcom config key, e.g. bigboss_notify_command = ["notify-send", "hcom: {from}", "{text}"] (argv list, placeholders as single tokens, no shell). When a message event has bigboss among its recipients, the sending hcom process spawns the command detached (no daemon; a sender inside Herdr has the desktop session, so notify-send works; mail or a custom script also fit). Unset = no change. notify-send, mail and sendmail exist on this machine. Opt-in fork feature: keep the hook in upstream send code minimal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With the key unset, sending to bigboss behaves exactly as before, covered by a test
- [ ] #2 With the key set, a message whose recipients include bigboss runs the command once, detached, with {from}, {text} (and the message id) substituted as single argv tokens; messages not addressed to bigboss run nothing; covered by tests
- [ ] #3 A failing or missing notify command never fails or delays the send; the failure is logged
- [ ] #4 hcom config help, README, docs and skills/hcom-agent-messaging document the key with a notify-send example
<!-- AC:END -->

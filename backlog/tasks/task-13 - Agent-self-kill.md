---
id: TASK-13
title: 'hcom kill SELF: an agent kills itself'
status: To Do
assignee: []
created_date: '2026-10-15 12:31'
updated_date: '2026-10-16 04:53'
labels: []
dependencies: []
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An agent needs a simple way to end its own process (and close its terminal pane) without knowing its instance name. Target SELF (uppercase) cannot collide with an instance name or catalog group, because both allow only lowercase letters, digits and underscore. Resolution uses the caller identity (--name or the hcom session environment), like 'hcom stop' without arguments.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 'hcom kill SELF' kills the calling instance's process and closes its pane, same as 'hcom kill <own name>'
- [ ] #2 Without a resolvable caller identity, 'hcom kill SELF' fails with a clear error and kills nothing
- [ ] #3 Lowercase 'self' keeps its meaning as an ordinary instance name
- [ ] #4 Built-in help, README.md, docs/, skills/hcom-agent-messaging/ and help/smoke tests cover SELF
<!-- AC:END -->

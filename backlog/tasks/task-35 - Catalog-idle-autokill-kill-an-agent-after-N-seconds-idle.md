---
id: TASK-35
title: 'Catalog idle autokill: kill an agent after N seconds idle'
status: To Do
assignee: []
created_date: '2026-10-16 20:40'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Roaming (and optionally persistent) catalog agents stay running after finishing their work. A catalog field "idle_kill": <seconds> (on an agent or in catalog defaults; merges like other fields, 0/false disables it for one agent) makes hcom kill the instance (same as 'hcom kill <name>': process ends, pane closes, session stays resumable via hcom r or catalog autostart) once it is idle. Idle means: status listening for at least idle_kill seconds, not blocked, no unread messages, and no unanswered own request (a message it sent with intent=request that the recipient has not replied to). hcom cannot tell whether the agent's last text was a question for the human; the unanswered-request guard protects agents waiting for a delegate. Only catalog agents are affected; no global hcom config key. Open for the plan: which process enforces the timeout (the agent's hcom PTY wrapper, the throttled reaper, or another watcher) so it also works for launches without a PTY wrapper, and how a running instance learns its catalog value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An agent with idle_kill=N that stays listening N seconds with no unread messages and no unanswered request is killed like 'hcom kill <name>' (pane closed), and the kill is logged as an event with reason idle_kill
- [ ] #2 A blocked agent, one with unread messages, or one waiting for a reply to its own intent=request message is not killed; the idle clock restarts when it becomes active or receives a message
- [ ] #3 idle_kill set in catalog defaults applies to every catalog agent; a later layer or the agent itself overrides it, and 0/false disables it; agents launched outside the catalog are never auto-killed
- [ ] #4 Invalid values (non-integer, negative) are rejected by catalog validation with a clear error naming the agent
- [ ] #5 A killed roaming agent is restarted by the next send autostart, and a persistent one resumes with hcom r
- [ ] #6 agent help, README.md, skills/hcom-agent-messaging/ and cli_smoke tests (kill after timeout, blocked/unread/pending-request guards, defaults override) cover the field
<!-- AC:END -->

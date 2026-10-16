---
id: TASK-35
title: 'Idle autokill: kill an agent after N seconds idle'
status: To Do
assignee: []
created_date: '2026-10-16 20:40'
updated_date: '2026-10-16 20:45'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents (mainly roaming catalog agents) stay running after finishing their work. An idle_kill value in seconds is stored per running instance and makes hcom kill it (same as 'hcom kill <name>': process ends, pane closes, session stays resumable via hcom r or catalog autostart) once it is idle. Sources: catalog field "idle_kill" (on an agent or in catalog defaults; merges like other fields, 0/false disables) fills the value at launch; 'hcom config -i <name> idle_kill <seconds>' (also -i self) sets or clears it for any running instance, including agents launched outside the catalog. No launch flag and no global hcom config key. Not to be confused with the existing instance key 'timeout' (headless listen timeout). Idle means: status listening for at least idle_kill seconds, not blocked, no unread messages, and no unanswered own request (a message it sent with intent=request that the recipient has not replied to). hcom cannot tell whether the agent's last text was a question for the human; the unanswered-request guard protects agents waiting for a delegate. Open for the plan: which process enforces the timeout (the agent's hcom PTY wrapper, the throttled reaper, or another watcher) so it also works for launches without a PTY wrapper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An agent with idle_kill=N that stays listening N seconds with no unread messages and no unanswered request is killed like 'hcom kill <name>' (pane closed), and the kill is logged as an event with reason idle_kill
- [ ] #2 A blocked agent, one with unread messages, or one waiting for a reply to its own intent=request message is not killed; the idle clock restarts when it becomes active or receives a message
- [ ] #3 A killed roaming agent is restarted by the next send autostart, and a persistent one resumes with hcom r
- [ ] #4 idle_kill in catalog defaults applies to every catalog agent and a later layer or the agent itself overrides it (0/false disables); 'hcom config -i <name> idle_kill N' sets it for any running instance, also one launched outside the catalog, and 'idle_kill 0' clears it; without either source no agent is auto-killed
- [ ] #5 agent help, config help (instance keys), README.md, skills/hcom-agent-messaging/ and cli_smoke tests (kill after timeout, blocked/unread/pending-request guards, defaults override, config -i on a non-catalog instance) cover the value
- [ ] #6 Invalid values (non-integer, negative) are rejected with a clear error, both by catalog validation (naming the agent) and by 'hcom config -i <name> idle_kill'
<!-- AC:END -->

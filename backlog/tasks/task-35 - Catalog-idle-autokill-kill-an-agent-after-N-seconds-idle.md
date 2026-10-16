---
id: TASK-35
title: 'Idle autokill: kill an agent after N seconds idle'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-16 20:40'
updated_date: '2026-10-16 21:54'
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
- [x] #1 A blocked agent, one with unread messages, or one waiting for a reply to its own intent=request message is not killed; the idle clock restarts when it becomes active or receives a message
- [x] #2 A killed roaming agent is restarted by the next send autostart, and a persistent one resumes with hcom r
- [x] #3 agent help, config help (instance keys), README.md, skills/hcom-agent-messaging/ and cli_smoke tests (kill after timeout, blocked/unread/pending-request guards, defaults override, config -i on a non-catalog instance) cover the value
- [x] #4 Invalid values (non-integer, negative) are rejected with a clear error, both by catalog validation (naming the agent) and by 'hcom config -i <name> idle_kill'
- [x] #5 An agent with idle_kill=N that stays listening N seconds with no unread messages and no unanswered request is killed like 'hcom kill <name>' (pane closed), and the kill is logged as a stopped event with initiator idle_kill
- [x] #6 idle_kill in catalog defaults applies to every catalog agent and a later layer or the agent itself overrides it (0/false disables); 'hcom config -i <name> idle_kill N' sets it for any running local top-level instance, also one launched outside the catalog (remote agents and subagents are rejected with an error), and 'idle_kill 0' clears it; without either source no agent is auto-killed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design: tmp/task-35-plan.md approved by agent_review_hcom; KV idle_kill:<name> storage, throttled sweep in router/PTY delivery/Grok/hook poll/listen with 5s wait cap and wake_all on writes, CAS claim with incarnation, detached __idle-kill worker bound to claim+incarnation+eligibility, read errors fail closed. Security review (agent_security_hcom) resolved: no kill target parsing (instance named all), no wrong-incarnation kill. Scope: local top-level agents; remote/subagent rejected.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added idle_kill (catalog field and config -i key) that kills an idle local agent like hcom kill with initiator idle_kill. Verified: fmt, clippy 0 warnings, cargo test --bin hcom 2807 ok, cli_smoke idle_kill(3) kill_ help config listen autostart agent_dry_run lifecycle ok; roaming filter has one environment failure (codex preflight, mise node shim in isolated HOME).
<!-- SECTION:FINAL_SUMMARY:END -->

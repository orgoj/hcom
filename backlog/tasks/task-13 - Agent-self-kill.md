---
id: TASK-13
title: 'hcom kill SELF: an agent kills itself'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 12:31'
updated_date: '2026-10-16 19:47'
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
- [x] #1 'hcom kill SELF' kills the calling instance's process and closes its pane, same as 'hcom kill <own name>'
- [x] #2 Without a resolvable caller identity, 'hcom kill SELF' fails with a clear error and kills nothing
- [x] #3 Lowercase 'self' keeps its meaning as an ordinary instance name
- [x] #4 Built-in help (kill topic and hcom --help overview), README.md, skills/hcom-agent-messaging/ and help/smoke tests cover SELF
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
kill SELF resolves the caller via build_ctx_for_command (--name, HCOM_PROCESS_ID, Codex thread) and kills it like 'hcom kill <own name>'; no identity -> error, nothing killed; lowercase self stays a name. Verified with cli_smoke kill_self_kills_only_the_calling_instance, unit + help tests, agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

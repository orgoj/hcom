---
id: TASK-81
title: 'send: --spawn-as on own name launches a clone instead of the self-target error'
status: To Do
assignee: []
created_date: '2026-11-02 15:19'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom send @<own catalog name> --as <other name> -- 'task' fails with '@hcom_dev is you (hcom_dev); agents don't receive their own messages': the self-target check runs before --spawn-as autostart. Agents clone themselves for a side task this way (user rule in the hcom skill); today only 'hcom agent <name> --as <clone> --hcom-prompt' works.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom send @<own name> --as <different name> autostarts the catalog agent under that name and delivers the message to it
- [ ] #2 hcom send @<own name> without --as, or with --as equal to the sender's name, still fails with the self-target error
- [ ] #3 cli_smoke regression test covers both cases; help and skill text describe self-clone via send --as
<!-- AC:END -->

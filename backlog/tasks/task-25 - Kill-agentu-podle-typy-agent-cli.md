---
id: TASK-25
title: 'hcom kill tool:<cli>: kill all agents of one CLI type'
status: To Do
assignee: []
created_date: '2026-10-15 14:44'
updated_date: '2026-10-16 04:53'
labels: []
dependencies: []
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Kill all running agents of a given CLI (e.g. all Antigravity agents) in one command. Syntax follows the existing tag:<name> target and the 'tool' field of 'hcom list': 'hcom kill tool:antigravity'. The prefix '#' proposed originally does not work, because the shell treats '#agy' as a comment. hcom-gui offers the same in its Kill menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 'hcom kill tool:<cli>' kills every running instance whose tool is <cli> and reports the count, like tag:
- [ ] #2 Known CLI aliases (e.g. agy for antigravity) resolve to the same tool; an unknown tool fails with an error listing valid tools
- [ ] #3 A tool with no running instances reports that nothing was killed and exits non-zero, like an empty tag:
- [ ] #4 hcom-gui Kill menu lists the CLIs of running instances and kills them after confirmation listing their names
- [ ] #5 Built-in help, README.md, docs/, skills/hcom-agent-messaging/, contrib/README.md and tests (cli_smoke, contrib/test-hcom-gui) cover tool:
<!-- AC:END -->

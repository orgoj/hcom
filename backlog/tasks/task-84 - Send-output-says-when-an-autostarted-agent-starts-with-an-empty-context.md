---
id: TASK-84
title: Send output says when an autostarted agent starts with an empty context
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 20:13'
updated_date: '2026-11-02 20:13'
labels: []
dependencies: []
type: feature
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When hcom send autostarts a catalog agent (e.g. a stopped roaming instance) with a clean session, the new agent remembers nothing from earlier sessions. The sender must see that in the send output so it does not expect the agent to remember. Requested by the user.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When hcom send autostarts a catalog agent with a clean session (no resume, no continue), the send output contains a note naming the agent, saying it started with an empty context and remembers no earlier session
- [ ] #2 An autostart with resume or continue prints no such note
- [ ] #3 Sends that start no agent print no such note
- [ ] #4 cli_smoke test covers the clean-start note and its absence for a resume start; README/docs describe the note
<!-- AC:END -->

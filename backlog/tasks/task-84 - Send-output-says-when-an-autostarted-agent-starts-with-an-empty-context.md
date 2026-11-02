---
id: TASK-84
title: Send output says when an autostarted agent starts with an empty context
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 20:13'
updated_date: '2026-11-02 20:25'
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
- [x] #1 When hcom send autostarts a catalog agent with a clean session (no resume, no continue), the send output contains a note naming the agent, saying it started with an empty context and remembers no earlier session
- [x] #2 An autostart with resume or continue prints no such note
- [x] #3 Sends that start no agent print no such note
- [x] #4 cli_smoke covers the clean-start note and its absence for a running agent and a resume start; a unit test (clean_start) also covers continue and dry-run; help, README, docs and skill describe the note
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Send autostart of a clean session prints 'Note: <name> started with an empty context; it does not remember earlier sessions.'; the flag comes from launch_named_with_roaming (false for already running, resume, continue, dry-run). Help, README, docs, skill updated. Verified: fmt, clippy 0, cargo test --bin hcom 2911 ok, cli_smoke roaming/send_/agent_dry_run/autostart/agent_group/lifecycle/handoff ok; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

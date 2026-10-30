---
id: TASK-66
title: >-
  Herdr launch: place an agent in the workspace named after its own directory,
  not the launcher's
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-30 17:11'
updated_date: '2026-10-30 17:49'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed: hcom_dev (~/projects/hcom) and knowledge run in the Herdr workspace 'ansible-wdt', although each has its own working directory and the Herdr workspace should be named after the agent's session directory. Likely the workspace is taken from the launching agent's context (e.g. send autostart from an ansible-wdt agent) instead of the target's cwd. Root cause not yet analyzed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A roaming agent started for a project lands in that project's workspace
- [x] #2 Regression test in tests/cli_smoke.rs covers a launch from a caller in a different directory
- [x] #3 A catalog agent launched into Herdr (hcom send autostart, hcom agent, group launch) uses its catalog session as workspace; without session, the workspace is named after its dir; a catalog agent without dir uses the caller's project name
- [x] #4 Plain hcom <cli> launches are unchanged (user: scope limited to catalog agents)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause: apply_herdr_placement defaulted the workspace to catalogs.project_root, found from the caller's CWD. Roaming: dir = its project root, so the workspace is that project (unchanged code path). Plain launches: no code change. Reviewed by agent_review (OK).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Effective.dir_explicit marks a dir from catalog or --dir; without session the Herdr workspace is that dir's name, else the caller's project. Verified with unit tests, cli_smoke agent_herdr_default_space_is_the_agent_directory_not_the_caller_project (fails without the fix), fmt, clippy, cargo test --bin hcom; help, README and skill updated.
<!-- SECTION:FINAL_SUMMARY:END -->

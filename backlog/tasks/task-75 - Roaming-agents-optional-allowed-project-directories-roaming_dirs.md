---
id: TASK-75
title: 'Roaming agents: optional allowed project directories (roaming_dirs)'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 10:16'
updated_date: '2026-11-01 10:17'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user via bin (spec: /home/michael/bin/tmp/hcom-roaming-dirs-request.md). A roaming catalog agent materializes <name>_<project>_<n> in any sender's project root; an agent meant for a few repos (e.g. pyznap_dev) gets useless instances started from other directories. New optional catalog field roaming_dirs restricts the project roots it may materialize in.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Catalog field roaming_dirs (list of paths, ~ and $VAR expanded, relative to the catalog file) is accepted on roaming agents; absent keeps today's behavior unchanged
- [ ] #2 A roaming agent with roaming_dirs materializes only for a project root equal to or inside one listed directory
- [ ] #3 hcom send @<role> from a disallowed directory, or with --project-dir outside the list, exits non-zero, names the agent, lists the allowed directories, suggests --project-dir <dir>, and starts nothing and queues no message
- [ ] #4 Catalog launch paths that materialize a roaming name for the current directory (hcom agent <name>, hcom agent attach, send autostart incl. restart of a stopped thread member) fail the same way
- [ ] #5 Sends from an allowed directory and with --project-dir <allowed dir> keep working, including to an already running instance
- [ ] #6 roaming_dirs on a non-roaming agent is rejected with a clear error
- [ ] #7 hcom agent list (table and --json) and hcom agent show display the allowed directories; show works from any directory
- [ ] #8 Field documented in hcom agent --help (Agent fields), docs/agent.md, README and the skill named-agents reference; tests cover every case above
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
See tmp plan sent to agent_review: single check in materialize_roaming_definition (+ retarget_roaming_route), show/list display, docs, cli_smoke tests. Implement after agent_review OK.
<!-- SECTION:PLAN:END -->

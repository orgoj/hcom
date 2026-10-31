---
id: TASK-68
title: 'Restore and update named-agent docs (docs/agent.md, skill reference)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 07:49'
updated_date: '2026-10-31 07:54'
labels: []
dependencies: []
priority: medium
type: docs
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
orgoj had docs/agent.md and skills/hcom-agent-messaging/references/named-agents.md describing catalogs, bundles (SOUL.md/NOTES.md/MEMORY/), imports, roaming, instruction transport. orgoj-rebuild dropped them; README keeps only a compressed Agent catalog section.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/agent.md exists and matches current orgoj-rebuild behavior
- [x] #2 skills/hcom-agent-messaging/references/named-agents.md exists, matches current behavior and is linked from SKILL.md
- [x] #3 README links both documents and has no statements contradicting the fork
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Restored docs/agent.md and skills/hcom-agent-messaging/references/named-agents.md from orgoj and updated them to orgoj-rebuild behavior (roaming <name>_<project>_<hash>, --project-dir, Note line, Herdr workspace from agent dir, agy, sidecar, disabled, idle_kill, status columns, inherit_env, Qoder/Grok instruction transport, nested-CLI guard, handoff log path); dropped Windows/macOS notes and the internal open-question section. README links docs/agent.md, describes bundle memory, fixes handoff log path; SKILL.md links the reference. Verified claims against hcom agent/send/handoff --help and src.
<!-- SECTION:FINAL_SUMMARY:END -->

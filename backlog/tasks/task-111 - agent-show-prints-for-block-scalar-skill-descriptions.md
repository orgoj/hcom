---
id: TASK-111
title: agent show prints '>-' for block-scalar skill descriptions
status: To Do
assignee: []
created_date: '2026-11-13T20:09:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 220000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reported by agent_coach__audit (hcom #358926): hcom agent show nuntora_dev prints 'skill: egui — >- (.../skills/egui/SKILL.md)' and the same for egui-kittest; both SKILL.md files use a valid YAML folded block scalar 'description: >-' (lines 3-8). The frontmatter parser takes the line value literally instead of parsing YAML block scalars.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom agent show renders the folded text of a 'description: >-' skill description
- [ ] #2 Literal block scalars ('|', '|-') render too
- [ ] #3 A regression test covers both block styles
<!-- AC:END -->

---
id: TASK-15
title: Definovat prompt pri start agenta v hcom gui
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:01'
updated_date: '2026-10-15 13:12'
labels: []
dependencies: []
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The hcom-gui Start dialog gets an optional Prompt field. A filled prompt is passed as hcom agent --hcom-prompt, the agent's first user message, and replaces the catalog prompt for that launch. The catalog system_prompt/SOUL.md stay as they are. An empty field drops the flag, so a catalog prompt (none is used today) still applies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Start with a filled Prompt runs hcom agent <agent> ... --hcom-prompt <text>, covered by contrib/test-hcom-gui
- [x] #2 Start with an empty Prompt omits --hcom-prompt, covered by contrib/test-hcom-gui
- [x] #3 contrib/README.md documents the Prompt field and that it overrides the catalog prompt but not system_prompt
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add --hcom-prompt {ask:prompt} to the Start button cmd (existing ask/flag-drop logic handles empty); test both cases by patching App.ask; document in README.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
agent_review_hcom: no blocking findings; README rewrap applied. Prompt field is single-line (Return confirms the dialog).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Start dialog has a Prompt field passed as --hcom-prompt (overrides the catalog prompt, not system_prompt; empty omits the flag). Verified with contrib/test-hcom-gui (29 OK, both cases tested).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-102
title: >-
  External answering of agent dialogs (AskUserQuestion, plan approval) for
  connectors such as Nuntora
status: Idea
assignee: []
created_date: '2026-11-12T18:03:00Z'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 209000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Request from nuntora_dev (#349281, Nuntora TASK-13, design /home/michael/work/nuntora/docs/design.md): show agent TUI questions that hooks cannot answer (AskUserQuestion choice/text, ExitPlanMode plan approval, elicitation, idle prompt) in an external decision hub and deliver the answer via hcom.

Current state (verified 2026-11-12):
- State: hcom events --follow --type status (resumable --after-id) or hcom list --json (status, status_context, status_detail, status_age_seconds, transcript_path).
- Claude PermissionRequest hook (src/hooks/claude.rs handle_permission_request) sets blocked/approval with tool_name and status_detail from family::extract_tool_detail; for AskUserQuestion and ExitPlanMode the detail is empty, so questions/options/plan are not stored. Notification elicitation_dialog -> blocked/approval without detail; idle_prompt -> listening. Codex blocked comes from PTY OSC9, no detail.
- hcom send is gated on listening (src/delivery.rs gate), so it answers idle prompts reliably but never a blocked dialog.
- Dialogs can only be answered with hcom term inject (raw keys, no gate, CLI/version-specific key sequences, confirmation only indirect via status leaving blocked).
- Full answer sent to nuntora_dev: tmp/nuntora-hcom-connector.md (not committed).

Suggested direction: store the pending tool_input (questions/options/plan) with the blocked status so connectors need not read transcripts; answer through Claude's hook decision (PermissionRequest/PreToolUse) instead of keystrokes; define a CLI command for an external program to submit the answer and get confirmation. Must stay a fork fallback with a minimal upstream touch. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

---
id: TASK-79
title: >-
  Codex: blocking non-approval dialogs (rate-limit model switch) must set status
  blocked
status: Idea
assignee: []
created_date: '2026-11-02 14:41'
updated_date: '2026-11-02 15:13'
labels: []
dependencies: []
priority: high
type: feature
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Problem: a Codex modal dialog (any screen with the footer 'enter <action> · esc <action>') blocks the agent, but hcom reports it as listening, so nobody sees that the agent is stuck.

Live case (agent_review_hcom_884, Codex 0.166.0, after turn end):
  Approaching rate limits
  Switch to gpt-6-luna for lower credit usage?
  › 1. Switch to gpt-6-luna / 2. Keep current model / 3. Keep current model (never show again)
  enter select · esc back
hcom showed status listening, context tui:not-ready ('prompt not visible'), unread 1; the waiting request was never delivered and the GUI did not highlight the row.

Cause: is_codex_modal_footer (src/pty/screen.rs:271) makes is_ready() false, so the delivery gate only holds messages (gate context tui:not-ready). Blocked status comes only from the approval scrape in update_delivery_state (src/pty/shared.rs:193: OSC title 'Action Required' or is_codex_approval_visible strings), which this dialog does not match. Codex has no hook for it.

User decision: every open Codex modal dialog is blocked, including pickers the user opened (/model, resume, ...) and then forgot; no exception for user activity. Scope is only the blocked status; alerting (reqwatch, notifications) is out of scope.

Direction: after the launch phase, a Codex modal footer without an approval -> blocked with its own context (e.g. pty:dialog), same latch as the approval scrape; on close restore the status from before the dialog (listening here, not active); PostToolUse/turn-end must also clear it. Delivery gating stays unchanged. Analysis: notes below.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decision: scope is only the missing blocked status; once the row shows blocked, the user handles it (GUI highlight is enough). Alerting via reqwatch or notifications is out of scope. Next step when specified: capture the exact dialog text, then extend Codex PTY detection so this dialog (and similar modal dialogs) sets blocked.

Live capture (agent_review_hcom_884, Codex 0.166.0): dialog 'Approaching rate limits / Switch to gpt-6-luna for lower credit usage?' with options 1-3 and footer 'enter select · esc back', shown after turn end. hcom showed listening / tui:not-ready ('prompt not visible'), unread 1: is_codex_modal_footer (src/pty/screen.rs:271) makes is_ready false, so the delivery gate only holds messages, while blocked comes only from the approval scrape (src/pty/shared.rs:193). Proposed: in update_delivery_state, post-launch Codex modal footer without approval -> blocked/pty:dialog (same latch), on release restore the pre-dialog status (listening here, not active). Details: tmp/task-79-blocked-analysis.md (move into the task when specifying).
<!-- SECTION:NOTES:END -->

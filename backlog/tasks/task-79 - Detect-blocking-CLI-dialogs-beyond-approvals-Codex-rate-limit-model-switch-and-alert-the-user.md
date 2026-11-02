---
id: TASK-79
title: >-
  Codex: blocking non-approval dialogs (rate-limit model switch) must set status
  blocked
status: Done
assignee:
  - '@hcom_dev__clone'
created_date: '2026-11-02 14:41'
updated_date: '2026-11-02 15:37'
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

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Codex modal dialog without approval (bottom line 'enter <action> · esc <action>') shown after the launch phase sets status blocked with context pty:dialog and the dialog title as detail (live fixture: 'Approaching rate limits')
- [x] #2 Pickers the user opened (e.g. /model) are blocked the same way; no exception for user activity
- [x] #3 When the dialog closes, the status from before the dialog is restored (listening stays listening, active stays active), only if the row is still blocked/pty:dialog
- [x] #4 Approval scrape keeps precedence: an approval dialog stays blocked/pty:approval, never pty:dialog; launch-phase dialogs (trust, update) keep the launch_blocked path
- [x] #5 Codex PostToolUse releases a pty:dialog block to active like a pty:approval block
- [x] #6 hcom list describes the state as 'blocked: dialog open'; delivery gating is unchanged
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. screen.rs: ScreenTracker::codex_modal_dialog_title() -> Option<String> (footer present; title = top line of the text block above the numbered options, else empty). 2. shared.rs update_delivery_state: Codex && !launch_phase && footer && !approval -> latched dialog (own negative_since/latched fields, same latch fn/APPROVAL_SCRAPE_CLEAR_MS); returns the dialog edge (Option<Option<String>>), signature otherwise unchanged so win.rs still compiles (Windows gets no dialog status). 3. shared.rs publish_dialog_status(edge, prior, instance, current_status) via lifecycle::set_status: rising -> blocked/pty:dialog detail=title, remembers prior status (skip if already blocked); falling -> restore prior if row still blocked/pty:dialog. 4. mod.rs: wrap the 3 update_delivery_state calls in one method that publishes the edge, prior status kept on the proxy; poll wake also while dialog latched. 5. hooks/codex.rs is_approval_block includes pty:dialog; instance_lifecycle description. 6. Tests: screen fixture with the live text, shared edge test (listening -> blocked -> listening, approval precedence, launch phase ignored), description test.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decision: scope is only the missing blocked status; once the row shows blocked, the user handles it (GUI highlight is enough). Alerting via reqwatch or notifications is out of scope. Next step when specified: capture the exact dialog text, then extend Codex PTY detection so this dialog (and similar modal dialogs) sets blocked.

Live capture (agent_review_hcom_884, Codex 0.166.0): dialog 'Approaching rate limits / Switch to gpt-6-luna for lower credit usage?' with options 1-3 and footer 'enter select · esc back', shown after turn end. hcom showed listening / tui:not-ready ('prompt not visible'), unread 1: is_codex_modal_footer (src/pty/screen.rs:271) makes is_ready false, so the delivery gate only holds messages, while blocked comes only from the approval scrape (src/pty/shared.rs:193). Proposed: in update_delivery_state, post-launch Codex modal footer without approval -> blocked/pty:dialog (same latch), on release restore the pre-dialog status (listening here, not active). Details: tmp/task-79-blocked-analysis.md (move into the task when specifying).

Specified from tmp/task-79-blocked-analysis.md (cause chain: is_codex_modal_footer makes is_ready false, blocked only from approval scrape in update_delivery_state). Out of scope: alerting (reqwatch, notifications), delivery gating, Windows (win.rs ignores the new return value), cursor/agy. Risk: a Stop hook during an open dialog sets listening while the dialog stays latched; not re-asserted (live case shows the dialog after turn end).

Review round 1 (agent_review): Windows excluded by standing user rule (fork is Linux-only; no changes to variants that cannot be compiled here): win.rs ignores the new return value and keeps today's behavior. Docs: hcom list help legend 'blocked' becomes 'needs human approval or a dialog answer' (src/commands/help.rs) with a help test; README/docs/skills carry no status-context list, so nothing else changes there.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Codex modal dialogs without approval (enter <action> · esc <action> footer) after the launch phase now set blocked/pty:dialog with the dialog title as detail ('blocked: dialog open'); closing restores the pre-dialog status (listening or active/pty:dialog_closed) only while the row is still blocked/pty:dialog; approval and launch_blocked keep precedence; PostToolUse releases pty:dialog like pty:approval; list help legend updated. Unix PTY only (Windows ignores the edge, fork is Linux-only). Verified: screen fixture from the live capture, update_delivery_state edge test, dialog_status_edge restore test, PostToolUse hook test, description and help tests; fmt, clippy 0, cargo test --bin hcom (only the known idle_kill float flake), cli_smoke help+lifecycle; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-79
title: >-
  Codex: blocking non-approval dialogs (rate-limit model switch) must set status
  blocked
status: Idea
assignee: []
created_date: '2026-11-02 14:41'
updated_date: '2026-11-02 15:08'
labels: []
dependencies: []
priority: high
type: feature
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Incident: an awaited Codex delegate hung on the interactive dialog 'usage limit running out, switch model?'. hcom messages were not delivered, status questions could never arrive and nobody noticed.

Findings (current code):
- Codex blocked state comes only from the PTY approval scrape (src/pty/shared.rs, src/pty/screen.rs): OSC title 'Action Required' or is_codex_approval_visible() strings (allow command?, 'press enter to confirm or esc to cancel', enter to submit answer, [y/n], yes (y), do you want to). The rate-limit/model-switch nudge is not an approval: Codex sets no 'Action Required' title for it and its footer (likely 'esc to go back') matches none of the strings, so the row stays active/listening instead of blocked (exact dialog text unverified; capture it).
- Even a blocked row alerts nobody: reqwatch (create_request_watches, src/db/subscriptions.rs) fires only on status listening or life stopped, never on blocked. hcom-gui only highlights blocked rows (orange) for a user who is looking. No user notification exists.
So it is unsupported, not misconfigured.

Suggested direction: (1) generic Codex modal detection (e.g. is_codex_modal_footer-style footer 'enter ... esc' outside the composer) mapped to blocked with its own context such as pty:dialog, without breaking delivery gating or the auto-review fix; (2) reqwatch/sender notification when an awaited target turns blocked for longer than a grace period; (3) a user-facing alert path (decide: hcom message to bigboss, desktop notification, GUI). Needs analysis and user decisions before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decision: scope is only the missing blocked status; once the row shows blocked, the user handles it (GUI highlight is enough). Alerting via reqwatch or notifications is out of scope. Next step when specified: capture the exact dialog text, then extend Codex PTY detection so this dialog (and similar modal dialogs) sets blocked.

Live capture (agent_review_hcom_884, Codex 0.166.0): dialog 'Approaching rate limits / Switch to gpt-6-luna for lower credit usage?' with options 1-3 and footer 'enter select · esc back', shown after turn end. hcom showed listening / tui:not-ready ('prompt not visible'), unread 1: is_codex_modal_footer (src/pty/screen.rs:271) makes is_ready false, so the delivery gate only holds messages, while blocked comes only from the approval scrape (src/pty/shared.rs:193). Proposed: in update_delivery_state, post-launch Codex modal footer without approval -> blocked/pty:dialog (same latch), on release restore the pre-dialog status (listening here, not active). Details: tmp/task-79-blocked-analysis.md (move into the task when specifying).
<!-- SECTION:NOTES:END -->

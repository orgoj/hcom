---
id: TASK-79
title: >-
  Detect blocking CLI dialogs beyond approvals (Codex rate-limit model switch)
  and alert the user
status: Idea
assignee: []
created_date: '2026-11-02 14:41'
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

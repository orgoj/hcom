---
id: TASK-115
title: Claude stays blocked while an auto-approved tool runs
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-18T16:36:00Z'
updated_date: '2026-11-18T18:18:00Z'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 226000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed 2026-11-18 on nuntora_dev__remote (claude, auto mode, Herdr pane): the dashboard showed status blocked/approval for ~10 min while the screen showed the Bash tool running ('Running… (2m 50s · timeout 10m)'), no approval dialog. Events: #392032 16:23:15 active tool:Bash (PreToolUse, src/hooks/common.rs:1871) -> #392033 same second blocked context=approval (handle_permission_request, src/hooks/claude.rs:1941) -> #392053 16:33:15 active approved:Bash (handle_posttooluse clears blocked, src/hooks/claude.rs:1670) when the 10 min command ended. Cause: the PermissionRequest hook marks blocked unconditionally, but the request was resolved without a visible dialog (auto-mode classifier or another PermissionRequest hook such as Dippy approved it), and nothing clears blocked until PostToolUse. Long commands therefore show as blocked for their whole runtime. Fix direction (analyse first): treat PermissionRequest as blocked only once a dialog is really shown (Notification permission_prompt in handle_notify, or a screen check), or clear blocked when the tool is observed running; check how Claude orders PermissionRequest hooks vs the auto-mode classifier. Upstream behavior: check whether upstream has the same handler and keep the change mergeable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An auto-approved tool call (auto mode or a PermissionRequest hook that allows) never leaves the instance in status blocked while the tool runs, covered by a test
- [ ] #2 A real approval dialog still sets blocked/approval and clears on approval or denial, covered by a test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Analysis 2026-11-18 (hcom_dev): premise 'auto-approved' is wrong. Claude transcripts carry permissionDecision per tool_result. Classifier/config/hook (Dippy PreToolUse allow) approvals fire NO PermissionRequest (nuntora_dev__remote 15:00:58, 15:54:01: only PostToolUse in hcom.log). PermissionRequest fires only when a dialog is shown (Dippy PreToolUse 'ask'). Incident 16:23:15: Dippy ask 'pkill -x' -> PermissionRequest -> Notification at 16:23:21 (permission_prompt arrives ~6 s after the dialog, absent when answered faster: wdt_main 10:19:28/43, answered in 4-5 s) -> tool_result 16:33:15 permissionDecision source=user_temporary. So the user answered the dialog, the 10 min command ran, and Claude fires no hook between the answer and PostToolUse; blocked/approval stays for the tool runtime. Upstream main (0.7.31) has the identical handler. Candidate fixes: (A) Claude PTY: a user or injected keystroke that answers the dialog (Enter, Esc, digit) while the row is hook-blocked context=approval releases to active (no screen scraping; answers outside the hcom PTY, e.g. Claude remote/mobile, stay uncovered); (B) Claude PTY screen scrape of the permission dialog, authoritative release when it disappears (fragile across Claude versions); (C) keep upstream behavior. AC #1 needs rewording to 'an answered dialog' once the option is chosen.

User decision 2026-11-18: variant B. Approval may come from any channel (window, Claude remote control, phone), so no keystroke heuristic (variant A rejected: user approves dialogs in many ways, hcom must not rely on an Enter). Plan: (1) find a stable marker of Claude's permission dialog in the PTY screen or OSC output (title/notification), verified on a live dialog via hcom term; (2) verify that a remote-control approval removes the dialog from the local screen; (3) in the Claude PTY, while the row is hook-blocked context=approval and the dialog is absent for a debounce period, release to active; PostToolUse/PermissionDenied stay as fallback; minimal upstream touch; tests. Then reword AC #1 to 'a dialog answered through any channel'.
<!-- SECTION:NOTES:END -->

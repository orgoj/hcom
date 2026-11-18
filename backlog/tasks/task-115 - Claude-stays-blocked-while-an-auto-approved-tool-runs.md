---
id: TASK-115
title: Claude stays blocked while an auto-approved tool runs
status: To Do
assignee: []
created_date: '2026-11-18T16:36:00Z'
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

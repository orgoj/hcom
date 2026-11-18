---
id: TASK-117
title: 'contrib/agent-commands: report and last-reply scripts for agent user commands'
status: To Do
assignee: []
created_date: '2026-11-18T17:01:00Z'
updated_date: '2026-11-18T17:01:00Z'
labels: []
dependencies:
  - TASK-116
priority: medium
type: feature
ordinal: 228000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generic scripts for the user-command system (design approved 2026-11-18), launched from hcom-dashboard commands or a herdr keybinding. Each takes an agent name, or --pane <id> resolved to the agent through pane_id in hcom list --json. They read data only through the hcom CLI: hcom term <agent> --json (screen), hcom list <agent> --json (status, status_detail), hcom events --agent <agent> --type status --last 20, hcom transcript <agent> --last 1. report [--to <agent>] [--note <text>]: writes screen, list JSON and status events to ~/.hcom/tmp/reports/<ts>-<agent>.md and sends hcom send @<to> --intent request with a short text, the note and the absolute report path (no dump in the message). last-reply: prints the agent's last reply. Machine-specific commands (e.g. dippy approval of status_detail) stay outside the repo; the README shows how to write one and how to bind it in dashboard.json and herdr. Depends on the dashboard keys menu/input/output for the menu entries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 report <agent> --to <x> --note <t> writes the report file with screen, status and status events and sends one request message to x that names the absolute path
- [ ] #2 last-reply <agent> prints the agent's last reply; --pane <id> resolves the agent from pane_id, an unknown pane exits nonzero with a message
- [ ] #3 contrib/agent-commands/README.md documents the scripts, the dashboard.json and herdr bindings and a custom-command example
<!-- AC:END -->

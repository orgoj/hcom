---
id: TASK-116
title: 'Dashboard: user commands in the agent context menu with input and output'
status: To Do
assignee: []
created_date: '2026-11-18T17:01:00Z'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 227000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Universal user commands for an agent (design approved 2026-11-18): a command is an executable that takes the agent; dashboard and herdr only launch it, the binary is unchanged. dashboard.json already has commands (CustomCommand in contrib/hcom-dashboard/src/config.rs: label, cmd, confirm, cwd, key; placeholders {dir} {instance} {agent} {terminal} {pane_id} {herdr_workspace} {herdr_tab} via replace_placeholders_str). Add optional keys, defaults keep current behavior: menu (bool) shows the command in the node and table-row context menu; input (prompt label) asks for a text before running and substitutes {input} (one argv token, no shell); output (none|show, default none) shows the command's stdout/stderr in a window. Out of scope: hcom-gui, hcom binary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A command with menu: true appears in the context menu of a graph node and of a table row and runs for that agent
- [ ] #2 A command with input asks for the text first, substitutes {input} as one argument, and Cancel runs nothing
- [ ] #3 A command with output: show displays its stdout and stderr after it ends; without output nothing is shown, as before
- [ ] #4 Existing commands without the new keys behave unchanged, covered by tests; README documents the keys with an example
<!-- AC:END -->

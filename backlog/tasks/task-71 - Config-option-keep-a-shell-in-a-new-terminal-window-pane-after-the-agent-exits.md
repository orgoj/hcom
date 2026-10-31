---
id: TASK-71
title: >-
  Config option: keep a shell in a new terminal window/pane after the agent
  exits
status: To Do
assignee: []
created_date: '2026-10-31 09:11'
labels: []
dependencies: []
priority: medium
type: enhancement
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Launch scripts for new windows/panes (tmux, kitty, herdr, ...) end with 'exec bash -l' (src/terminal.rs, create_bash_script, opens_new_window branch; upstream behavior since v0.7.0), so after the agent exits (e.g. /exit in a Herdr pane) a bash stays open. 'hcom kill' already closes the Herdr pane. Add an hcom config option (e.g. keep_shell_on_exit). Default on = current upstream behavior unchanged. When off: on exit code 0 the script removes itself and exits so the window/pane closes; on a non-zero exit code it still opens the shell so the error stays readable. Keep the existing unset of hcom identity vars and instruction transports before any shell.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With the option unset or on, the generated new-window bash script is unchanged (ends with unset, rm, exec bash -l)
- [ ] #2 With the option off, a tool exit code 0 ends the script without starting a shell, so the window/pane closes
- [ ] #3 With the option off, a non-zero tool exit code still unsets hcom vars and instruction transports and starts exec bash -l
- [ ] #4 Background and run-here launches are unaffected
- [ ] #5 The option is documented in hcom config help, README/docs and the hcom-agent-messaging skill, with tests for both settings
<!-- AC:END -->

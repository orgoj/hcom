---
id: TASK-71
title: >-
  Config option: keep a shell in a new terminal window/pane after the agent
  exits
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 09:11'
updated_date: '2026-11-02 18:30'
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
- [x] #1 With the option unset or on, the generated new-window bash script is unchanged (ends with unset, rm, exec bash -l)
- [x] #2 With the option off, a tool exit code 0 ends the script without starting a shell, so the window/pane closes
- [x] #3 With the option off, a non-zero tool exit code still unsets hcom vars and instruction transports and starts exec bash -l
- [x] #4 Background and run-here launches are unaffected
- [x] #5 The option is documented in hcom config help, README/docs and the hcom-agent-messaging skill, with tests for both settings
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added config keep_shell_on_exit (terminal.keep_shell_on_exit, HCOM_KEEP_SHELL_ON_EXIT, default true). launch_terminal resolves it from the effective launch env (HcomConfig::load(Some(&final_env))). When false, the new-window bash script captures the tool status, unsets hcom vars and instruction transports, removes itself, exits on status 0 and otherwise runs exec bash -l; on true the script is unchanged. Background/run-here scripts and PowerShell untouched. Documented in config --info/--help, README, docs/agent.md, skill. Verified with unit tests (script tails for both settings, executed tail for status 0 vs 3, background/run-here identical, config default/env), cli_smoke keep_shell_on_exit_config_and_help; fmt, clippy 0, cargo test --bin hcom 2909 ok. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

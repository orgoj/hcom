---
id: TASK-28
title: Config option to disable headless launches
status: To Do
assignee: []
created_date: '2026-10-16 05:30'
labels: []
dependencies: []
type: feature
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An agent sometimes launches another agent with --headless; when that agent then asks a question (permission, trust), the human cannot answer it without hcom term inject. Add a config key (proposed: allow_headless, default true, env HCOM_ALLOW_HEADLESS) that, when false, turns a --headless launch into a normal launch in the configured terminal and prints a notice, so the calling agent still gets its result. Print mode (claude -p) is out of scope: it is non-interactive and asks nothing. Note: bundled scripts that rely on --headless (e.g. hcom run fatcow) then open visible terminals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With allow_headless=false, 'hcom <tool> --headless' launches in the configured terminal (not a background PTY) and prints a notice that headless was disabled by config
- [ ] #2 The same applies to 'hcom r' and 'hcom f' with --headless and to headless launches from the TUI
- [ ] #3 Default (true) keeps current behavior; claude -p print mode is unaffected
- [ ] #4 'hcom config allow_headless --info', config help, README.md, skills/hcom-agent-messaging/ and tests (unit + cli_smoke dry-run) cover the key
<!-- AC:END -->

---
id: TASK-6
title: 'hcom-gui: git status column for agent directories'
status: To Do
assignee: []
created_date: '2026-10-14 20:11'
labels: []
dependencies: []
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
contrib/hcom-gui shows each agent's working directory in the Dir column. Add a Git column that shows, for directories inside a git repository, whether the worktree is dirty and how many commits the branch is ahead of/behind its upstream, so the user sees unfinished or unpushed work per agent at a glance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Git column shows dirty state and ahead/behind counts (e.g. '* ↑2 ↓1') for agent dirs inside a git repo
- [ ] #2 Dirs outside a git repo or without an upstream show an empty or neutral value, without errors
- [ ] #3 Git queries do not block the GUI refresh (cached or run off the UI thread)
- [ ] #4 contrib/test-hcom-gui covers the new column
<!-- AC:END -->

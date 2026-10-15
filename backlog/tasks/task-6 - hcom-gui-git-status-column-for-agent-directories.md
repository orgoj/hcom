---
id: TASK-6
title: 'hcom-gui: git status and branch columns for agent directories'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-14 20:11'
updated_date: '2026-10-15 09:47'
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
- [x] #1 A Git column shows dirty state and ahead/behind counts (e.g. '* ↑2 ↓1') for agent dirs inside a git repo
- [x] #2 Dirs outside a git repo or without an upstream show an empty or neutral value, without errors
- [x] #3 Git queries do not block the GUI refresh (cached or run off the UI thread)
- [x] #4 contrib/test-hcom-gui covers the new column
- [x] #5 A Branch column shows the current branch of agent dirs inside a git repo, read live (not the launch-time snapshot)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Branch and Git columns to contrib/hcom-gui: one git --no-optional-locks status --porcelain=v2 --branch per distinct dir in the refresh worker thread; README updated. Verified with contrib/test-hcom-gui (14 OK, real temp repos for dirty/ahead/behind/no repo) and a live run on this repo; reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

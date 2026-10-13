---
id: TASK-2
title: Resolve stopped agents before scanning Claude transcripts in resume
status: To Do
assignee: []
created_date: '2026-10-13 12:45'
updated_date: '2026-10-13 12:45'
labels: []
dependencies: []
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
For a stopped agent the instance row is gone, so resolve_name_to_plan (src/commands/resume.rs:289) first calls resolve_thread_name, which reads every ~/.claude/projects/*/*.jsonl line by line looking for a custom-title (here 1379 files, 1.3 GB) before it falls back to the stopped-instance lookup. Warm cache: `hcom r <stopped> --dry-run` takes ~2.2 s; cold cache it can take much longer, and `hcom agent agent_coach --resume` appeared to hang before launching (no launch logged) until the user interrupted it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resuming a known stopped hcom agent by name does not read Claude transcripts or the Codex session index
- [ ] #2 Resuming by a Claude custom title or Codex thread name that is not an hcom agent name still works
- [ ] #3 A test covers that a stopped agent name wins without the thread-name scan
<!-- AC:END -->

---
id: TASK-2
title: Resolve stopped agents before scanning Claude transcripts in resume
status: Done
assignee: []
created_date: '2026-10-13 12:45'
updated_date: '2026-10-17 08:05'
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
- [x] #1 Resuming a known stopped hcom agent by name does not read Claude transcripts or the Codex session index
- [x] #2 Resuming by a Claude custom title or Codex thread name that is not an hcom agent name still works
- [x] #3 A test covers that a stopped agent name wins without the thread-name scan
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
resolve_name_to_plan skips the Claude transcript / Codex index thread-name scan when the name has a stopped life event (has_stopped_event), so a known stopped agent resumes from its stored snapshot session id; snapshot errors are still reported by prepare_resume_plan. Thread names that are not hcom agent names resolve as before. Test test_stopped_agent_name_wins_over_thread_name covers a valid and a snapshot-less stopped agent colliding with a Claude thread title. Live 'hcom r skills --dry-run': 4.49 s -> 0.02 s.
<!-- SECTION:FINAL_SUMMARY:END -->

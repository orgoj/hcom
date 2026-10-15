---
id: TASK-8
title: 'hcom-gui: context window fill column for Claude and Codex'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 09:46'
updated_date: '2026-10-15 10:28'
labels: []
dependencies: []
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom does not track token usage, but every instance has transcript_path. Claude JSONL: last assistant message usage (input + cache_creation + cache_read tokens) is the context size; the window is not in the transcript and must come from the model (200k, 1M for [1m]). Codex rollout: last token_count event has last_token_usage.input_tokens and model_context_window. Read only the transcript tail, off the UI thread.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Context column shows fill as a percentage (and tokens) for running Claude and Codex instances
- [x] #2 Other CLIs, missing or unreadable transcripts show an empty value without errors
- [x] #3 Reading transcripts does not block the GUI and reads only the file tail
- [x] #4 contrib/test-hcom-gui covers Claude and Codex fixtures
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Context column (tokens + % of window) to contrib/hcom-gui, read from the 1 MiB transcript tail and cached by mtime/size: Claude latest non-sidechain assistant usage with an assumed 1M window, Codex latest token_count. Verified with contrib/test-hcom-gui (21 OK) and a live run on 13 instances (27 ms); reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

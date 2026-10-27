---
id: TASK-51
title: 'hcom-gui: volba modelu a effort pri startu agenta'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 16:12'
updated_date: '2026-10-27 19:19'
labels: []
dependencies: []
type: feature
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Start dialog of contrib/hcom-gui cannot choose a model or reasoning effort; hcom agent already supports --model and --reasoning (Claude/Antigravity --effort, Codex model_reasoning_effort), and agent list --json exposes the catalog model and reasoning.

In scope: two editable comboboxes in the Start dialog (model, reasoning), prefilled from the catalog; README/docstring.
Out of scope: clearing a catalog model back to the CLI default; per-CLI filtering of offered values; any binary change.
Risk: hcom ignores reasoning for CLIs other than claude/agy/codex (hcom behavior, unchanged).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Start dialog shows a model field: editable combobox prefilled with the agent's catalog model, offering every distinct model used in the catalog
- [x] #2 Start dialog shows a reasoning field: editable combobox prefilled with the agent's catalog reasoning, offering minimal, low, medium, high, xhigh, max
- [x] #3 A changed model or reasoning is passed as --model / --reasoning to hcom agent; a value equal to the prefill or empty passes no flag
- [x] #4 contrib/test-hcom-gui covers the flags (changed, unchanged, empty) and the offered combobox values; contrib/README.md documents the fields
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
BUTTONS Start: add --model {ask:model} --reasoning {ask:reasoning} after --cli. build_rows: catalog_model/catalog_reasoning on catalog and instance rows. run_button prefill adds them. ask(): editable Combobox for model (distinct catalog models) and reasoning (fixed levels). Tests, README, docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Start dialog gains editable model and reasoning comboboxes prefilled from the catalog (suggestions: catalog models, minimal..max); only a changed value passes --model/--reasoning. Verified with contrib/test-hcom-gui (120 tests OK at this commit), reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

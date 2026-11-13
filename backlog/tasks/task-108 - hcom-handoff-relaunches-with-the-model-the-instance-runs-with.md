---
id: TASK-108
title: hcom handoff relaunches with the model the instance runs with
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-13T15:33:00Z'
updated_date: '2026-11-13T15:40:00Z'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 217000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom handoff must restart exactly the agent that runs now: same name, CLI and model (user, 2026-11-13). Name (--as) and CLI (--cli from instances.tool, TASK-85) are already passed; the model is not, so the successor takes the catalog model. The running model is recorded in instances.launch_args (JSON array, e.g. ["--model","gemini-4-hydrogen-medium",...]). Out of scope: a model switched inside the session (/model), which hcom does not record.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom handoff passes --model <value> taken from the instance's recorded launch_args (--model X, --model=X, -m X) to the relaunch
- [x] #2 Without a recorded model, handoff passes no --model and the catalog choice applies
- [x] #3 Explicit --cli on handoff to another CLI does not carry the old CLI's model
- [x] #4 Regression test covers the dry-run relaunch with the recorded model
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
handoff passes --model from instances.launch_args (--model X, --model=X, -m X; last wins), not when --cli switches to another CLI. Unit test recorded_model_*, cli_smoke handoff_keeps_the_model_the_instance_was_launched_with; help, README, docs/agent.md, skill reference updated. Review agent_review: help finding fixed.
<!-- SECTION:FINAL_SUMMARY:END -->

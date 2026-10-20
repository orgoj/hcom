---
id: TASK-40
title: 'Roaming instance names: replace the hex path hash with a 3-digit number'
status: To Do
assignee: []
created_date: '2026-10-20 15:09'
updated_date: '2026-10-20 15:48'
labels: []
dependencies: []
priority: medium
type: enhancement
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Roaming instance names are <name>_<project>_<hash>, where <hash> is the first 6 hex digits of SHA-256 of the canonical project root path (roaming_project_slug in src/commands/agent.rs, introduced by TASK-37). Hex strings tokenize into several meaningless fragments, so agents read and type these names poorly. A 3-digit decimal number is a single token in common BPE tokenizers (GPT cl100k/o200k and Llama 3 split digits into groups of at most 3, so 4 digits would be 2 tokens).

Goal: <hash> becomes a deterministic 3-digit number 100-999 derived from the same SHA-256 of the root path, e.g. agent_review_hcom_482.

The number only has to separate roots that share a basename (~/.hcom vs ~/projects/hcom), so 900 values are enough; the existing collision check in materialize_roaming_definition stays as the safety net and still refuses a name owned by another root.

In scope: roaming_project_slug and every consumer of its output (materialize, retarget_roaming_route, unroutable-name hints), built-in help (agent topic and top-level overview in src/commands/help.rs / agent.rs), README.md roaming section, docs/, skills/hcom-agent-messaging/, unit and cli_smoke tests.

Out of scope: migrating or renaming existing hex-named roaming instances (user decision: they keep running until they end on their own; new sends address the new numeric names); changing project root detection or the <project> basename slug; a word dictionary instead of numbers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 roaming_project_slug returns <project>_<n> where n is a 3-digit decimal in 100..=999 computed deterministically from SHA-256 of the root path bytes, and the same root always yields the same n
- [ ] #2 Unit tests assert the new slug for /home/u/.hcom and /home/u/projects/hcom, that the two differ, and that the suffix is exactly 3 digits in 100..=999
- [ ] #3 When two different roots map to the same instance name, routing still fails with the existing collision error instead of reusing the other root's instance (covered by a test)
- [ ] #4 Built-in help, README.md, docs/ and skills/hcom-agent-messaging/ describe the suffix as a 3-digit number and no longer mention hex digits; help tests updated
- [ ] #5 Existing hex-named roaming instances are not renamed or killed by the change; a send from their project addresses the new numeric name
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Change the suffix in roaming_project_slug to (u32 from first digest bytes % 900) + 100.
2. Update unit tests in agent.rs and the cli_smoke/send.rs expectations; add the 3-digit range and collision tests.
3. Update help text (agent topic, top-level overview), README.md:415, docs/, skills/hcom-agent-messaging/.
4. Run cargo fmt --check, cargo clippy --bin hcom --all-targets, cargo test --bin hcom, cargo test --test cli_smoke.
<!-- SECTION:PLAN:END -->

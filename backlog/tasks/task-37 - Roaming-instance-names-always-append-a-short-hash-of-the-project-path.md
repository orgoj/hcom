---
id: TASK-37
title: 'Roaming instance names: always append a short hash of the project path'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-17 10:54'
updated_date: '2026-10-17 11:07'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Roaming instance name was <agent>_<slug>, where roaming_project_slug (src/commands/agent.rs) kept only the project root basename, lowercased, non-alphanumerics folded to _. Different roots with the same basename collided: ~/.hcom and ~/projects/hcom both gave agent_review_hcom, so roaming_route refused (reported by agent_coach, msg #172496). User decision: always append the first 6 hex of SHA-256 of the canonical project root path, e.g. agent_review_hcom_3f2a9c; keep the existing collision check as a safety net. Existing instances with old names are not migrated; after the fix is installed, kill all old-name roaming instances (user request).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A roaming instance name is <name>_<basename slug>_<h>, where <h> is the first 6 lowercase hex characters of SHA-256 of the canonical project root path; the same root always gives the same name
- [x] #2 Two project roots with the same basename (e.g. ~/.hcom and ~/projects/hcom) route to different instances and a send from either starts/addresses its own instance without the uniqueness error
- [x] #3 The existing collision check stays: an existing instance with the computed name but a different recorded root still refuses the send
- [x] #4 retarget_roaming_route and the unroutable-name list use the same hashed name
- [x] #5 Agent help (show note and Roaming agents), send help roaming line, README Roaming agents and help tests describe <name>_<project>_<hash>
- [x] #6 cargo fmt --check, cargo clippy --bin hcom --all-targets (0 warnings), cargo test --bin hcom and cargo test --test cli_smoke pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. roaming_project_slug appends _ + 6 hex of Sha256(root path bytes). 2. Update error text 'basenames must be unique' to a hash-collision wording only if needed (keep the check). 3. Unit test: slug for a fixed path, different roots with the same basename differ. 4. cli_smoke: roaming tests compute the expected hashed name via a helper (sha2 in the test); collision tests insert an instance under the second root's hashed name with the first root's directory. 5. Help, README, help tests; skill has no roaming mention. 6. bump-fork-version, verify, review by agent_review_hcom, commit, release build, then kill old-name roaming instances after user confirmation. Out of scope: migration/renaming of existing instances, name length limits.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Analysis: slug from roaming_project_slug is used in materialize_roaming_definition (instance name + collision check), retarget_roaming_route (expected name) and catalog_message_targets (unroutable names). sha2 is already a dependency (resume.rs, subscriptions.rs). Hash length decided by the user: 6 hex.

Review by agent_review_hcom: hash raw OsStr bytes (non-UTF-8 parents), top-level help + skill; both fixed, re-review OK (#172633).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Roaming instance names are now <name>_<project>_<hash> (6 hex of SHA-256 of the root path's raw bytes), computed in roaming_project_slug and used by materialize, retarget and unroutable names; collision check kept with new wording. Help (top-level, agent, send), README, skill and tests updated. Verified: cargo fmt --check, clippy 0 warnings, cargo test --bin hcom 2814 passed, cargo test --test cli_smoke 127 passed.
<!-- SECTION:FINAL_SUMMARY:END -->

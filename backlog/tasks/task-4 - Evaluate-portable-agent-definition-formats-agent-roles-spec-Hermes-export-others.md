---
id: TASK-4
title: >-
  Evaluate portable agent definition formats (agent-roles-spec, Hermes export,
  others)
status: To Do
assignee: []
created_date: '2026-10-13 14:09'
labels: []
dependencies: []
references:
  - 'https://github.com/SeemSeam/agent-roles-spec'
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Check https://github.com/SeemSeam/agent-roles-spec (only ~18 stars): is anything worth adopting, or could it serve as the hcom agent definition format? Also research whether a more widely used, portable agent definition format exists; Hermes agent export is a likely stronger candidate. Compare against the current hcom catalog (agents.json + agent bundles).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 agent-roles-spec reviewed: concrete ideas worth adopting listed, or a stated reason to reject it
- [ ] #2 Research lists the portable agent definition formats in use (including Hermes agent export) with adoption and maturity signals
- [ ] #3 Each candidate is mapped to the hcom catalog/bundle model: what maps, what is missing, import/export feasibility
- [ ] #4 States a recommendation: adopt a format, add import/export, borrow specific ideas, or keep the hcom format
<!-- AC:END -->

---
id: TASK-70
title: 'hcom-gui: Restart keeps the restarted agent''s row selected'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 08:33'
updated_date: '2026-10-31 08:35'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After Restart the selection lands on another agent: the killed instance row vanishes, render() moves the selection to the agent's catalog row and consumes keep_agent; when the instance reappears the catalog row vanishes and the index fallback picks an unrelated row. Stop already works.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After Restart, once the instance reappears under the same name, its row is selected, even when the agent's catalog row was selected in between
- [x] #2 Choosing a row of another agent during the restart cancels the pending reselection
- [x] #3 contrib/test-hcom-gui covers the restart sequence (instance gone, catalog row, instance back)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Restart sets select_next to agent|instance; _on_select clears select_next only when a row of another agent is chosen; add test in contrib/test-hcom-gui.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Restart (when=live) sets select_next to agent|instance, so the relaunched instance is reselected; _on_select drops select_next only when a row of another agent is chosen, so render's interim move to the catalog row no longer cancels it. Verified with contrib/test-hcom-gui (168 OK), new cases test_selection_follows_agent_while_restarting (extended) and test_selecting_another_agent_while_restarting_cancels_the_jump.
<!-- SECTION:FINAL_SUMMARY:END -->

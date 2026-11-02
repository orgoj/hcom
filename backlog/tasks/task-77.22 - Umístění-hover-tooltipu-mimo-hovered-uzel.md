---
id: TASK-77.22
title: Umístění hover tooltipu mimo hovered uzel
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:40'
updated_date: '2026-11-02 19:34'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F6/S6: u okraje plátna se tooltip (TASK-77.9) posune přes uzel, který popisuje; šířka neodpovídá obsahu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tooltip zkouší pozice vpravo, vlevo, pod a nad uzlem a použije první, kde se vejde do plátna bez překrytí kruhu hovered uzlu
- [x] #2 Uzel do 150 px od okraje plátna není tooltipem zakryt
- [x] #3 Šířka tooltipu se řídí obsahem do maxima; dir zkracuje domovský adresář na ~ a dlouhé cesty elipsou uprostřed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tooltip placed by canvas::tooltip_placement (right, left, below, above; first that fits the canvas without covering the node), measured in the same frame by an invisible Area so placement never uses a stale size. Width follows content up to 360 px: ui.separator() took the full max width and is replaced by a line painted across the content. dir shortened by table_data::shorten_path (home -> ~, middle ellipsis over 48 chars; home_dir from HOME at start). README updated. Verified with model_tests for placement, nodes 0-150 px from each edge, content width, shorten_path, tooltip_content dir with/without home, and a node-switch plus same-node refresh regression (mutants fail). Plan and diff reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

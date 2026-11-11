---
id: TASK-99
title: 'Review TASK-98: hcom recover --json při resume'
status: Done
assignee: []
created_date: '2026-11-11T09:27:00Z'
labels:
  - review:rejected
dependencies: []
priority: low
type: task
ordinal: 202000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
agent_review upozornil, že hcom recover --json se při <jména>/--all neuplatní.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Zamítnuto: --json je zdokumentovaný jen pro výpis kandidátů; resume běží přes hcom r a jeho textový výstup stačí, dashboard výsledek resume neparsuje.
<!-- SECTION:FINAL_SUMMARY:END -->

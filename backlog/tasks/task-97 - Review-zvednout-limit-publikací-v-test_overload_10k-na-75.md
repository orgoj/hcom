---
id: TASK-97
title: 'Review: zvednout limit publikací v test_overload_10k na <=75'
status: Done
assignee: []
created_date: '2026-11-09T09:29:00Z'
labels:
  - review:rejected
dependencies: []
ordinal: 195000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Nález agent_review k TASK-77.97: test_overload_10k_events_throttled_coalescing hlásí při loadavg ~10 až 65 publikací místo <=60. Odmítnuto: test je známě nestabilní při zátěži stroje i bez této změny (NOTES), po opravě sync.rs prošel 5/5 při běžné zátěži a celá sada 249/249; uvolnění záměrné hranice do opravy indikátoru nepatří.
<!-- SECTION:DESCRIPTION:END -->

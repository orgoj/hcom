---
id: TASK-128
title: Cut CPU cost of hcom-dashboard and per-session hcom polling
status: Idea
assignee: []
created_date: '2026-11-19T15:45:00Z'
labels: []
dependencies: []
priority: medium
type: enhancement
ordinal: 239000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measured 2026-11-19 on a 4-core i5-4690 with 14 Claude sessions: hcom-dashboard averaged ~33 % of one core over 4h24m (8 % own, 25 % reaped children). Children: idle_check.py --json -- <agent> per agent, each running hcom list --json; plus a long-running hcom events --follow --full --type message --after-id 0. Reads: 236 GB rchar / 168 GB read_bytes over its life, ~16 MB/s live, almost all from ZFS ARC, so the cost is CPU (sys time), not disk. Per Claude session: frequent short-lived hcom claude-mod-poll --session-id ... and hcom list processes; hcom was 353 of 624 short-lived processes in a ps sample; machine-wide ~200 new processes/s and 15-30 % CPU in kernel. Suggested direction: fewer process spawns (batch idle_check over all agents with one hcom list, longer or event-driven poll intervals in claude-mod-poll), and avoid re-reading the full event log (--after-id 0 --full). User: record only, do not implement now. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

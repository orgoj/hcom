---
id: TASK-118
title: >-
  Scheduled agent messages: cron runs a parameterless script that sends the
  prompt
status: Idea
assignee: []
created_date: '2026-11-18T17:07:00Z'
updated_date: '2026-11-18T17:16:00Z'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 229000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User need (2026-11-18): several times a day send the knowledge agent, as instance knowledge_vulncheck, a prompt to check current vulnerabilities (it has its own procedure and scripts); ideally configured from hcom-dashboard. User decisions: no separate catalog entry (knowledge_vulncheck is the knowledge agent under another name); each scheduled job is a script in ~/.hcom so the crontab line is only the command without parameters; the script uses send, which delivers whether the instance runs or not. Works with the current binary (docs/agent.md, send --spawn-as/--as): 'hcom send @knowledge --as knowledge_vulncheck --intent request -- <prompt>' delivers to a running knowledge_vulncheck, otherwise autostarts the catalog agent knowledge under that name and delivers once ready (output 'Queued; delivery pending' while it starts). No scheduler exists in hcom or upstream. To verify: send autostart from the cron environment (PATH from mise, Herdr server; herdr_autostart starts it headless if missing). Remaining hcom scope (open): how hcom-dashboard shows or sets up these scheduled scripts (list of jobs, crontab times, last run). Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Installed 2026-11-18 by bin: user crontab line '0 11,16,21 * * * /home/michael/.hcom/cron/knowledge-vulncheck >>/home/michael/.local/state/knowledge-vulncheck.log 2>&1' with PATH (~/.cargo/bin, ~/.local/bin, mise shims) and XDG_RUNTIME_DIR=/run/user/1000 above it; verified under env -i (knowledge_vulncheck autostarted in Herdr, message delivered). Requirements from bin if hcom/dashboard manages cron entries: (1) edit only its own marked block (begin/end markers), never other crontab lines; (2) set PATH and XDG_RUNTIME_DIR inside the block or command (Herdr autostart fails without them under cron); (3) take over or skip the existing knowledge-vulncheck line so it never runs twice, and tell bin on takeover so bin removes its line.
<!-- SECTION:NOTES:END -->

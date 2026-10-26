---
id: TASK-47
title: >-
  Send to a roaming agent with no instance in the caller's project: error
  instead of autostart
status: Idea
assignee: []
created_date: '2026-10-26 20:34'
updated_date: '2026-10-26 21:02'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User order (via agent_coach). When someone sends to a roaming agent (e.g. @sphere_dev) from a directory where no instance of that roaming agent runs, hcom should fail instead of starting a new instance: the error says the agent is roaming, that the sender must address the right running instance, and lists the running instances (e.g. sphere_dev_ansible_proweb_505).

Incident: cca_manager sent @sphere_dev from its own directory at 20:29 and spawned sphere_dev_cca_manager_369 instead of reaching the instance working in the proweb clone.

Open question for the user: today a send to a roaming agent deliberately starts it in the caller's project (global SYSTEM_PROMPT.md, line ~59: a roaming agent materializes for your project). The new error contradicts that rule. How do the two fit? Options to decide: error only when another instance of the role already runs elsewhere (autostart stays when none runs), error always (explicit start command needed), or an opt-in/opt-out flag or catalog setting. The global prompt rule must be updated to match whatever is chosen.

Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

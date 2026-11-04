---
id: TASK-88
title: 'hcom agent --restart přijme i jméno instance, nejen katalogové jméno'
status: Idea
assignee: []
created_date: '2026-11-04 08:00'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek od herdr_dev (pro uživatele): hcom agent <name> --restart (nebo ekvivalentní příkaz pro restart) má přijmout i jméno instance (např. roaming agent_review_hcom_884 nebo jméno z --as), ne jen katalogové jméno agenta. Dnes volající mapuje instance -> katalogový agent sám (hcom agent list --all --json, instances) a předává --as <instance>. Použití: Herdr zkratka Ctrl+Alt+S (~/.config/herdr/scripts/hcom-restart-active-pane.sh) restartuje instanci ve fokusovaném pane. Navrhovaný směr: rozlišit jméno instance a odvodit katalogového agenta a --as. Před implementací je potřeba analýza (izolace katalogu, kolize jmen instance a katalogu).
<!-- SECTION:DESCRIPTION:END -->

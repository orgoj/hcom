---
id: TASK-46
title: 'hcom-gui: start dialog prefills the clone name prefix on focus'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-26 13:42'
updated_date: '2026-10-26 13:44'
labels: []
dependencies: []
type: feature
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User convention: a clone's name starts with '<agent name>__'. In the Start agent dialog the Name field stays empty, but when it receives focus (click or keyboard) while empty, it is prefilled with the agent name followed by '__'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Name field is empty when the Start agent dialog opens
- [x] #2 When the empty Name field gets focus by click or keyboard, it contains '<agent name>__' with the cursor at the end
- [x] #3 A Name field that already contains text is not changed on focus
- [x] #4 A Name left at exactly '<agent name>__' counts as empty, so the agent starts under its default name
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Start dialog Name entry gets '<agent>__' on FocusIn when empty, cursor at end; an unchanged prefix counts as empty (Tab through the field must not start an agent named 'x__'). Verified with test_empty_name_gets_clone_prefix_on_focus, test_clone_prefix_by_keyboard_and_typed_name, test_name_with_text_is_not_changed_on_focus.
<!-- SECTION:FINAL_SUMMARY:END -->

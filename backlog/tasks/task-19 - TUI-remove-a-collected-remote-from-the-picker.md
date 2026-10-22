---
id: TASK-19
title: 'TUI: remove a collected remote from the picker'
status: Done
assignee: []
created_date: '2026-10-22T02:59:00Z'
updated_date: '2026-10-22T04:53:00Z'
labels: []
dependencies: []
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press d on a collected row in the remote picker to remove it, after a y/n confirmation. Real remotes (from git config) are refused with a status-line note. Calls the same code as 'git fetchview remove', so every registered file that has it is edited. The row disappears at once. Part of the add-from-the-TUI work; design is being brainstormed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 d on a collected row asks for confirmation, then removes it from the files and the list
- [x] #2 d on a real remote does nothing but show a note
- [x] #3 State-machine and render tests cover confirm, cancel and refusal
- [x] #4 Keys, README and docs/usage.md are updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: d on a collected row shows a status-line prompt 'remove <label>? y/n'; y removes, anything else cancels. No dd, no unconfirmed delete.
<!-- SECTION:NOTES:END -->

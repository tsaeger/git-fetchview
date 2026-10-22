---
id: TASK-20
title: 'TUI: add a URL from the picker'
status: Done
assignee: []
created_date: '2026-10-22T02:59:00Z'
updated_date: '2026-10-22T04:57:00Z'
labels: []
dependencies: []
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press n (new) in the remote picker to open a one-line prompt styled like the search box. The URL goes through validate_url and is written to the default remotes file (the same file 'add url' uses without --file); --file and --global stay CLI-only. On success the row appears, the cursor moves to it, and the picker stays on the remote list. A rejected URL shows the reason in the status line and keeps the prompt open. The key is n, not a, because a toggles every row in the add multi-select.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Invalid URLs show the rejection reason and are not written
- [x] #2 An existing URL is not duplicated and the cursor goes to the existing row
- [x] #3 Tests cover the prompt state machine and the render
- [x] #4 Keys, README and docs/usage.md are updated
- [x] #5 n opens the prompt; Enter adds, Esc cancels
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: after a successful add, stay in the remote list with the cursor on the new row (not drilling into refs, not ticking it).
<!-- SECTION:NOTES:END -->

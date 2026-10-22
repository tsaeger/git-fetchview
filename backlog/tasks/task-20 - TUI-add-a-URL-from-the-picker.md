---
id: TASK-20
title: 'TUI: add a URL from the picker'
status: To Do
assignee: []
created_date: '2026-10-22T02:59:00Z'
labels: []
dependencies: []
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press a in the remote picker to open a one-line prompt styled like the search box. The URL goes through validate_url and is written to the default remotes file (the same file 'add url' uses without --file); --file and --global stay CLI-only. On success the row appears and the cursor moves to it; a rejected URL shows the reason in the status line and keeps the prompt open. Brainstorm decides what happens right after an add.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 a opens the prompt; Enter adds, Esc cancels
- [ ] #2 Invalid URLs show the rejection reason and are not written
- [ ] #3 An existing URL is not duplicated and the cursor goes to the existing row
- [ ] #4 Tests cover the prompt state machine and the render
- [ ] #5 Keys, README and docs/usage.md are updated
<!-- AC:END -->

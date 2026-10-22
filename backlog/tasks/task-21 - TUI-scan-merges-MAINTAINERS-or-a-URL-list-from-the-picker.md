---
id: TASK-21
title: 'TUI: scan merges, MAINTAINERS or a URL list from the picker'
status: Done
assignee: []
created_date: '2026-10-22T02:59:00Z'
updated_date: '2026-10-22T05:12:00Z'
labels: []
dependencies: []
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press A in the remote picker to open a menu of sources: merges, maintainers, URL file. Choosing one runs the scan and opens the existing multi-select; Enter writes and refreshes the list. The merges source asks for a range in a prompt, defaulting to 'HEAD' with -n 500, editable. This replaces leaving the TUI to run 'git fetchview add'. Should be done with or after TASK-15 so the CLI and TUI agree on picker behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A opens the source menu and each source runs its scan
- [x] #2 The merges range prompt defaults to HEAD with -n 500 and can be edited
- [x] #3 The multi-select returns to the remote picker with the list refreshed
- [x] #4 Scan errors show in the status line without leaving the picker
- [x] #5 Tests, README and docs/usage.md are updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The scan runs inline, not on the worker thread, so a huge unbounded merges scan blocks the screen. The -n 500 default keeps it short; moving scans to the worker with a cancel is a possible follow-up.
<!-- SECTION:NOTES:END -->

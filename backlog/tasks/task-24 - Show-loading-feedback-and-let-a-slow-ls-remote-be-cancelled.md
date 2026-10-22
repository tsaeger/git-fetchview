---
id: TASK-24
title: Show loading feedback and let a slow ls-remote be cancelled
status: Done
assignee: []
created_date: '2026-10-22T03:16:00Z'
updated_date: '2026-10-22T03:35:00Z'
labels: []
dependencies: []
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
While git ls-remote runs, the picker does not redraw and ignores keys. A remote that stalls (seen with the macOS keychain helper on a URL with a password) freezes the screen with no hint. Run the git call off the UI thread, show 'loading <remote>…' on the status line with a spinner, and let Esc or Ctrl-C cancel and return to the remote list. The same applies to the hidden-refs fetch. Keep the state machine pure: the loader reports back through a channel, and the cancel kills the child process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The status line shows loading while ls-remote or the hidden-ref fetch runs
- [x] #2 Esc or Ctrl-C during a load cancels it, kills the git process, and returns to the remote list
- [x] #3 A load that fails still shows its error as today
- [x] #4 Tests cover loading, cancel and failure in the state machine, with a fake slow loader
- [x] #5 AGENTS.md keybind notes and docs/usage.md mention the cancel
<!-- AC:END -->

---
id: TASK-36
title: 'Multi-select: space selects and advances, shift/ctrl with arrows or j/k extend'
status: To Do
assignee: []
created_date: '2026-10-22T14:57:00Z'
labels: []
dependencies: []
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In the multi-select (tui::select), space toggles the row under the cursor. Change it so space selects the current row and moves to the next, and space on an already selected row deselects it and also moves on. Shift and ctrl combined with the arrow keys and with j/k should also work. The intended meaning is not settled: likely shift extends a range from where it started, and ctrl moves the cursor without changing the selection, but the maintainer wants to try it. Prototype the interaction in a throwaway web page first (not committed), settle the behavior with the maintainer, then implement it in the TUI. Note that terminals often cannot tell shift+j from J or report ctrl+j as newline, so check what crossterm actually delivers in the supported terminals (Terminal.app, iTerm2, kitty, a Linux console) before promising any key.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 space selects the current row and advances; space on a selected row deselects it and advances
- [ ] #2 shift with arrows and j/k, and ctrl with arrows and j/k, do what the prototype settled on
- [ ] #3 Which key combinations arrive in the supported terminals is recorded, and unreliable ones are not promised
- [ ] #4 The behavior is covered by unit tests in select.rs and keys.rs
- [ ] #5 The status-line hints, README, docs/usage.md and AGENTS.md describe the new keys
<!-- AC:END -->

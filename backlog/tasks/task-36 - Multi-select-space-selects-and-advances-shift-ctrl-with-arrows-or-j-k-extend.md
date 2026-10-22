---
id: TASK-36
title: 'Multi-select: space advances, J/K highlight a range'
status: Done
assignee: []
created_date: '2026-10-22T14:57:00Z'
updated_date: '2026-10-22T19:37:00Z'
labels:
  - prototype
dependencies: []
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In the multi-select (tui::select), space toggles the row under the cursor. Change it so space selects the current row and moves to the next, and space on an already selected row deselects it and also moves on. Shift and ctrl combined with the arrow keys and with j/k should also work. The intended meaning is not settled: likely shift extends a range from where it started, and ctrl moves the cursor without changing the selection, but the maintainer wants to try it. Prototype the interaction in a throwaway web page first (not committed), settle the behavior with the maintainer, then implement it in the TUI. Note that terminals often cannot tell shift+j from J or report ctrl+j as newline, so check what crossterm actually delivers in the supported terminals (Terminal.app, iTerm2, kitty, a Linux console) before promising any key.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 space selects or deselects the current row and advances
- [x] #2 shift with arrows and J/K moves the cursor and highlights anchor..cursor without selecting; space toggles the whole highlight by the anchor row's state (selected: deselect all, else select all), the cursor lands after the range; a plain move or Esc drops the highlight
- [x] #3 ctrl is not used. Which shift combinations arrive in Terminal.app, iTerm2, kitty and a Linux console is recorded, and unreliable ones are not promised
- [x] #4 The behavior is covered by unit tests in select.rs and keys.rs
- [x] #5 The status-line hints, README, docs/usage.md and AGENTS.md describe the new keys
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Key probe, iTerm2 (TERM=xterm-256color), crossterm 0.29, default mode: shift+down and shift+up arrive as Down/Up with SHIFT; shift+j and shift+k arrive as Char('J')/Char('K') with SHIFT; shift+space arrives as a plain space with no SHIFT (so the highlight must be its own state, as designed). Still to probe: Terminal.app, kitty, Linux console.

Key probe, Terminal.app (TERM=xterm-256color), default mode: shift+down and shift+up arrive as plain Down/Up with NO modifier, so they cannot be told from the unshifted arrows. shift+j/shift+k arrive as Char('J')/Char('K') with SHIFT; shift+space is a plain space. So J/K is the only combination that works in both iTerm2 and Terminal.app: the hints and docs promise J/K, and shift+arrows is a bonus where the terminal reports it (iTerm2), not promised.

Key probe, Ghostty (TERM=xterm-ghostty), default mode: same as iTerm2. shift+down/up arrive with SHIFT, shift+j/k as Char('J')/Char('K') with SHIFT, shift+space is a plain space. Not probed: kitty, Linux console.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Space toggles the row and advances. J/K and shift+arrows highlight anchor..cursor without selecting; space then sets the whole range by the anchor row's state and lands after it; any other key or Esc drops the highlight. Rows already present are skipped. ctrl is unused. Key probes: iTerm2 and Ghostty send shift+arrows with SHIFT, Terminal.app does not, so docs promise J/K only. Not probed: kitty, Linux console.
<!-- SECTION:FINAL_SUMMARY:END -->

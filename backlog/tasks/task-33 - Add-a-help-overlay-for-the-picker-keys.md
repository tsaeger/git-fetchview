---
id: TASK-33
title: Add a ? help overlay for the picker keys
status: To Do
assignee: []
created_date: '2026-10-22T06:03:00Z'
updated_date: '2026-10-22T14:57:00Z'
labels: []
dependencies:
  - TASK-35
  - TASK-36
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The remote list's key hints are full at 80 columns (l open, / i search, s sort, n new, o browse, A scan, p promote, d remove, q quit), and j/k, c clear, h/esc back and the sort keys had to be dropped from them. Add a ? key that opens an overlay listing every key for the current level and mode, closed by any key, and shorten the status-line hints to the few most useful plus '? help'. The overlay text should come from one table next to the keymap so it cannot drift from it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ? opens an overlay with every key for the current level and mode
- [ ] #2 The status-line hints end with '? help' and fit 80 columns
- [ ] #3 The table the overlay draws from sits with the keymap, and a test checks every Action in the keymap appears
- [ ] #4 README, docs/usage.md and AGENTS.md mention it
<!-- AC:END -->

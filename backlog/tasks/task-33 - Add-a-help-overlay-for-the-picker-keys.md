---
id: TASK-33
title: Add a ? help overlay for the picker keys
status: Done
assignee: []
created_date: '2026-10-22T06:03:00Z'
updated_date: '2026-10-22T20:01:00Z'
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
- [x] #1 ? opens an overlay listing every key in sections, over any level and over the add picker; any key closes it
- [x] #2 The status-line hints are shortened to the most useful keys plus '? help' and fit 80 columns
- [x] #3 The overlay draws from the HELP table in keys.rs, and tests check each line against the keymap and that no mapped key is missing
- [x] #4 README, docs/usage.md and AGENTS.md mention it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
? opens one overlay with every key in sections (two columns at 80x24), over the remote list, refs list and add picker. It draws from HELP in keys.rs; tests check each sample key against map() and fail if a mapped key is not in the table. Hints are now short and include '? help' (p promote is gone from the remote hints; c clear and h/l col from all).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-35
title: 'Sort with one key: s cycles ascending, descending, none'
status: Done
assignee: []
created_date: '2026-10-22T14:54:00Z'
updated_date: '2026-10-22T15:50:00Z'
labels: []
dependencies: []
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
There is no sort mode any more. h/l and the left/right arrows move a column cursor at every level, and s cycles the sort on the column under the cursor: ascending, descending, none (back to the default order). s on a different column starts that column at ascending and clears the old one. Level changes are Enter/Esc only; h/l and the arrows no longer drop in or out. r no longer reverses. The pseudo-column 'default' goes away: no sort means the default order, and the cursor starts on the first real column (label). Do this before TASK-33 so the help text describes the final keys.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 s cycles ascending, descending, none on the column under the cursor; none restores the default order
- [x] #2 s on a different column starts it at ascending and the old column's sort clears
- [x] #3 h/l and left/right move the column cursor at every level and never change the sort or the level
- [x] #4 Enter and Esc are the only keys that drop in and out; Mode::Sort, Action::Reverse and the 'default' column are removed
- [x] #5 The column cursor is visible in the header at every level, starting on the first real column (label)
- [x] #6 The sorted column's header shows an up arrow (ascending) or a down arrow (descending), and nothing when it is not sorted
- [x] #7 The bottom status line no longer reports the sort
- [x] #8 Sorting keeps its group rules and natural name order in every state
- [x] #9 keys.rs, app.rs and ui.rs tests cover the three states, the cursor and the arrows
- [x] #10 README, docs/usage.md, AGENTS.md (including the 'hjkl and arrows both navigate' rule) and the sorting demo text match the new keys
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Maintainer's rules (chat, 2026-10-22): a sorted column carries an indicator at the top: up arrow, down arrow, or nothing. Columns keep their sort state while h/l move the column cursor. Only s on a different column changes which column is sorted. This replaces today's behavior where moving the sort column re-sorts the list live. Still to settle: when s lands on a new column, does it start at ascending, and is the old column's state cleared (assumed yes, one sorted column at a time).

Confirmed: s on a new column starts at ascending.

Settled in chat 2026-10-22: no sort mode; h/l move the column cursor; default column removed; sort indicators at the bottom of the screen are dropped because the eye is on the cursor/header.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sorting has no mode. h/l and the arrows move a column cursor (underlined header) at every level; s cycles ascending, descending, none on that column, and a new column starts ascending. Level changes are Enter/Esc only. Removed: Mode::Sort, Action::Reverse, the default column, the bottom-line sort display. Hints and docs updated. Commit 24b601f. The hint text dropped j/k and the remote-level open hint, and o browse became o once, to fit 80 columns; TASK-33 should settle the hint line with a ? help entry.
<!-- SECTION:FINAL_SUMMARY:END -->

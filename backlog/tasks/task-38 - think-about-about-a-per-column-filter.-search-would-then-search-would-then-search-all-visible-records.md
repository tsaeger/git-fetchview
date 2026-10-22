---
id: TASK-38
title: Per-column filters
status: Done
assignee: []
created_date: '2026-10-22T17:49:00Z'
updated_date: '2026-10-22T19:58:00Z'
labels:
  - prototype
dependencies: []
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today the one search box matches across all columns (fzf-style or regex tried per column). Idea: a filter per column, with the search box then searching only what the column filters left visible. Needs a prototype first: how a column filter is entered (key on the column cursor? 'f'?), how it shows (header marker, a second filter line), how it composes with search and sort, and whether it applies to the refs list, which has one column. Prototype in a throwaway web page (not committed), settle the design with the maintainer, then write acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 f edits a filter box under the header of the column under the column cursor, in the remote list and in add; Esc or Enter leaves it
- [x] #2 A blank box has a darker background and a dim 'filter' placeholder that goes away when it is edited; a box with text is tinted
- [x] #3 The column cursor underlines its box; the refs list has no boxes and underlines its header
- [x] #4 Column filters take the search syntax and Ctrl-r regex; the search box searches the rows they left; c clears all
- [x] #5 Unit tests cover the list, app, select and drawing; README, usage, AGENTS and architecture docs say so
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Prototype: filters go in a row of input boxes under the header, blank boxes with a darker background. Still to settle: placeholder or not, and the key (f).

Settled in prototype: blank boxes show a dim 'filter' placeholder on a darker background. 'f' enters the box with the placeholder removed and the cursor at the start. Key stays 'f' unless a conflict turns up.

Column cursor underline goes under the filter input box of that column (where the eyes are), not on the header text. The header keeps only the sort arrow. When a list has no filter row (refs list, one column), the underline stays on the header. This changes the TASK-35 header underline.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
f edits the filter box under the column cursor's header (remote list and add). Blank boxes are darker with a dim 'filter' placeholder that clears on edit; filled boxes are tinted; the cursor column is underlined on its box. Same matcher as search; search then runs over what the column filters left; c clears all. The refs list has one column and no boxes. Not in the status-line hints (80 columns are full); the placeholder and TASK-33's help cover discovery.
<!-- SECTION:FINAL_SUMMARY:END -->

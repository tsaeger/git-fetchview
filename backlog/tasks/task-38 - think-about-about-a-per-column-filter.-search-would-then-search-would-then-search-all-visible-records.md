---
id: TASK-38
title: Per-column filters
status: To Do
assignee: []
created_date: '2026-10-22T17:49:00Z'
updated_date: '2026-10-22T19:21:00Z'
labels:
  - prototype
dependencies: []
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today the one search box matches across all columns (fzf-style or regex tried per column). Idea: a filter per column, with the search box then searching only what the column filters left visible. Needs a prototype first: how a column filter is entered (key on the column cursor? 'f'?), how it shows (header marker, a second filter line), how it composes with search and sort, and whether it applies to the refs list, which has one column. Prototype in a throwaway web page (not committed), settle the design with the maintainer, then write acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Prototype: filters go in a row of input boxes under the header, blank boxes with a darker background. Still to settle: placeholder or not, and the key (f).

Settled in prototype: blank boxes show a dim 'filter' placeholder on a darker background. 'f' enters the box with the placeholder removed and the cursor at the start. Key stays 'f' unless a conflict turns up.

Column cursor underline goes under the filter input box of that column (where the eyes are), not on the header text. The header keeps only the sort arrow. When a list has no filter row (refs list, one column), the underline stays on the header. This changes the TASK-35 header underline.
<!-- SECTION:NOTES:END -->

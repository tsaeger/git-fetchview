---
id: TASK-37
title: '''c'' clears the sort as well as the search'
status: Done
assignee: []
created_date: '2026-10-22T17:48:00Z'
updated_date: '2026-10-22T19:52:00Z'
labels: []
dependencies: []
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
'c' clears the search box. It should also drop the sort (back to none), so one key returns the list to how it opened. The 'default' column is gone since TASK-35, so 'none' is the only reset. The column cursor stays where it is: it is a cursor, not a filter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 c clears the search and the sort, in the picker and in the multi-select
- [x] #2 The column cursor stays where it is
- [x] #3 Unit tests in list.rs cover it; docs/usage.md, README and AGENTS.md say so
<!-- AC:END -->

---
id: TASK-4
title: align picker columns with ratatui Table
status: To Do
assignee: []
created_date: '2026-10-21T19:21:00Z'
labels:
  - tui
dependencies: []
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Picker rows are one joined string, so name, url and tags do not line up. Draw them as columns with ratatui::widgets::Table instead of List.

Design:
- Rows carry cells (Vec<String>) next to the joined label. The joined label stays the filter and sort input, so `/` matching and SortKeys do not change.
- Column widths come from the widest visible cell, capped, with the last column taking the rest. Recomputed per draw from the visible rows.
- Header row shows the column names; the active sort column is marked (and the arrow when reversed), which could replace the sort bar in the status line.
- Same Table for remotes, refs, and the add multi-select (with the check box as the first cell).
- No new dependency. Decided against ratagrid, see TASK-2.

The README demo (TASK-1) should be recorded after this lands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Name, url and tags are aligned in the remotes, refs and add pickers
- [ ] #2 Filter and sort behave exactly as before (existing tests pass unchanged)
- [ ] #3 Narrow terminals truncate the last column instead of wrapping
<!-- AC:END -->

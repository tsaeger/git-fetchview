---
id: TASK-2
title: consider using Ratagrid for table layouts
status: Done
assignee: []
created_date: '2026-10-21T17:47:00Z'
updated_date: '2026-10-21T19:21:00Z'
labels: []
dependencies: []
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
https://github.com/kahwee/ratagrid

current tables do not have column alignment, consider using ratagrid and evaluate if using would allow for more advanced sorting filtering etc.  Recommend design spec.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Evaluated kahwee/ratagrid 0.3.1. Not adopting it.

- It needs Rust 1.99. Our MSRV is 1.88.0, so adopting it means raising the MSRV in Cargo.toml, rust-toolchain.toml, the msrv shell and the CI job.
- It owns input: `Grid::handle_event` takes crossterm events and keeps sort, search and selection state. Our keymap (`KeyParser`: counts, gg, G, sort mode, h/l column pick) and the `List` state machine would be replaced or bent around it. It is mouse-first; we are keyboard-first.
- Its sorting and search are what we already have. Single selection only, no multi-select marks like `tui::select`. No natural sort built in; we would pass comparators we already wrote.
- It pins crossterm 0.29 as a direct dependency next to ratatui's own.

The alignment complaint does not need it. Rows are already pre-joined strings drawn by `ratatui::widgets::List`. Ratatui's own `Table` is already a dependency, costs no MSRV bump, and keeps our key handling. Follow-up: TASK-4.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-41
title: 'Remote list layout: gutter, column widths, info panel'
status: Done
assignee: []
created_date: '2026-10-22T22:23:00Z'
updated_date: '2026-10-22T22:31:00Z'
labels:
  - tui
dependencies: []
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Findings from looking at the remote list against ~/dev/linux (about 1000 remotes). The dot gutter, column widths and truncation make the list hard to read.

Current problems:
- The dot sits in its own column plus the 2-char column gap, so names start too far from the cursor.
- The name filter box spans the gutter, so its text is not aligned with the names.
- Every column but the last is padded to its widest cell (capped at 60) and the last takes all the rest, so source is stretched across the screen.
- A long URL is cut at the end, which hides the part that differs between rows (the tail) and keeps the part they share (git://git.kernel.org/pub/scm/linux/...).

Decisions so far:
- Keep the dot in a gutter, a fixed 2 characters wide, with no extra column gap after it. Name cells start right after it.
- The name filter box starts at the names, not at the gutter. The gutter has no header and no filter box.
- Each column is only as wide as its widest content (up to the cap). A fourth, empty, elastic column soaks up the leftover width once every real column fits. Source is no longer the stretchy one.
- When a URL (or any cell) does not fit, elide the middle or the shared prefix with an ellipsis so the part that differs between rows stays visible. Needs a decision on how: common-prefix elision across visible rows, or plain middle elision.
- An info panel at the bottom shows the cursor row as <column name>: <value>, one line per column, names right-justified and values left-justified, so the full URL is always readable. A hotkey toggles it off and on. Its key goes in HELP and the hints.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The dot gutter is 2 characters wide with no column gap after it; names start immediately after it
- [x] #2 The name filter box and header align with the names, not the gutter
- [x] #3 Columns are sized to their widest content (capped); a trailing empty elastic column takes the leftover width
- [x] #4 Cells that do not fit are elided so the part that differs between rows stays visible
- [x] #5 A bottom info panel shows the cursor row as right-justified column name and left-justified value; a hotkey toggles it
- [x] #6 The toggle is in the HELP table and the docs; tests cover widths, elision and the panel
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Gutter is a fixed 2-wide column with no gap after it and no filter box. Columns are sized to their widest cell, shrunk widest-first on a narrow screen, with a blank elastic column last. Cells that do not fit lose their middle (ui::elide). v toggles an info panel under the list (remotes, refs, add) that shows the cursor row as right-justified name and left-justified value; it hides itself on short screens. Gaps are real blank cells because ratatui's Cell::default has a column span of 0.
<!-- SECTION:FINAL_SUMMARY:END -->

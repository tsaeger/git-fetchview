---
id: TASK-39
title: Show real git remotes differently from collected ones in the remote list
status: To Do
assignee: []
created_date: '2026-10-22T17:57:00Z'
updated_date: '2026-10-22T19:17:00Z'
labels:
  - prototype
dependencies: []
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In the remote list, remotes already configured in git (real) should look subtly different from collected entries (a faint background or text tone), so you can tell them apart without opening them. Needs a prototype first: try two or three treatments against all themes in tui/theme.rs and a 16-color terminal, since a background that works in kanagawa may vanish elsewhere. Settle with the maintainer, then implement and add acceptance criteria. Interacts with TASK-36's selected-row styling and TASK-33's legend.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Prototype settled (2026-10-22): gutter dot for real remotes.
<!-- SECTION:NOTES:END -->

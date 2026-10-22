---
id: TASK-40
title: Call the tags column 'source' in the picker
status: To Do
assignee: []
created_date: '2026-10-22T19:23:00Z'
labels: []
dependencies: []
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The remote list's third column is headed 'tags', but it holds where an entry came from (maintainers, a merge scan, a file). In git, 'tags' means something else, so it confuses. Head the column 'source' in the TUI, the multi-select and the docs. My guess: the remotes file format keeps its third field as is (renaming it breaks existing files), and the code keeps the name internally unless it is cheap to rename. Do it alongside TASK-38, which reworks the header.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The remote list and the add multi-select head the column 'source'
- [ ] #2 Sort and column-cursor behavior is unchanged; the tests that name the column follow
- [ ] #3 README, docs/usage.md and docs/architecture.md call it the source; the file format description says what the third field means
- [ ] #4 The decision on the file format field name is recorded
<!-- AC:END -->

---
id: TASK-40
title: Call the tags column 'source' in the picker
status: Done
assignee: []
created_date: '2026-10-22T19:23:00Z'
updated_date: '2026-10-22T19:53:00Z'
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
- [x] #1 The remote list and the add multi-select head the column 'source'
- [x] #2 Sort and column-cursor behavior is unchanged; the tests that name the column follow
- [x] #3 README, docs/usage.md and docs/architecture.md call it the source; the file format description says what the third field means
- [x] #4 The decision on the file format field name is recorded
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: the remotes file keeps its third field named tags (renaming breaks existing files). The Rust side of the picker (SortColumn::Source, SortKeys.source, select::Row.source) was renamed; entry.tags in the registry keeps its name.
<!-- SECTION:NOTES:END -->

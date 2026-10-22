---
id: TASK-12
title: Push main and watch the first CI run of the docs check
status: Done
assignee: []
created_date: '2026-10-22T02:54:00Z'
updated_date: '2026-10-22T11:50:00Z'
labels: []
dependencies: []
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
main is ahead of origin by several commits, including the demo GIFs (Git LFS) and the new CI step that runs npm ci and cargo xtask docs. That step has only run locally. Push, then confirm the ci jobs pass on ubuntu and macOS, and that LFS objects uploaded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 main is pushed and the LFS objects are on the remote
- [x] #2 All four CI jobs pass, including npm ci and the docs check
- [x] #3 The demo GIFs render on the GitHub README
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CI run 39769424867: all five jobs passed. GIFs render on GitHub in the README and the demos page (confirmed by the user).
<!-- SECTION:NOTES:END -->

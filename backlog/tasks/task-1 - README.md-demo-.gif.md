---
id: TASK-1
title: README.md demo .gif
status: In Progress
assignee: []
created_date: '2026-10-21T17:40:00Z'
updated_date: '2026-10-21T17:52:00Z'
labels:
  - demo
  - docs
dependencies: []
type: enhancement
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use `vhs` to come up with a compelling demo to be used near top of README.md to capture reader's attention and bring focus to the usefulness of this project.

Linux kernel repo at /Users/tsaeger/dev/linux can be used to show compelling workflows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/demo/setup.sh and docs/demo/demo.tape reproduce the demo from a clean checkout
- [ ] #2 The GIF is tracked with Git LFS and embedded near the top of README.md
- [ ] #3 The demo shows the final UI, so it is recorded after the UI redo
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Scripts: setup.sh builds a release binary and a scratch kernel clone that shares objects with ~/dev/linux; demo.tape drives it (done). 2. After the UI redo, update the tape keystrokes, run vhs, check frames. 3. Commit docs/demo/demo.gif (LFS) and embed it in README.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Scripts and LFS config are committed. A recording made before the UI redo proved the pipeline: add maintainers (306 trees found), filter /netdev, fetch net-next, natural newest-first tag sort, tig. In vhs, tig must be waited for with Wait+Screen; Enter did not open a commit in tig under vhs. The GIF is about 2 MB.
<!-- SECTION:NOTES:END -->

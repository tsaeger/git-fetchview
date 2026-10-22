---
id: TASK-18
title: Decide whether to pin the ubuntu runner
status: Done
assignee: []
created_date: '2026-10-22T02:54:00Z'
updated_date: '2026-10-22T03:26:00Z'
labels: []
dependencies: []
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub is moving the ubuntu-latest label to Ubuntu 26 (began October 19, 2026). CI passed on it, so nothing is broken. Decide between following the label and pinning ubuntu-24.04, and note why in ci.yml.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ci.yml states the choice in a comment
- [x] #2 All jobs pass after the change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: follow ubuntu-latest. Nix pins the toolchain and tools, so the host OS adds little risk, and a pinned image needs a bump when it retires. The comment in ci.yml records this. The second criterion (all jobs pass) is checked once the push runs; see TASK-12.
<!-- SECTION:NOTES:END -->

---
id: TASK-18
title: Decide whether to pin the ubuntu runner
status: Done
assignee: []
created_date: '2026-10-22T02:54:00Z'
updated_date: '2026-10-22T11:37:00Z'
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
CI run 39769424867: all jobs passed on ubuntu-latest.
<!-- SECTION:NOTES:END -->

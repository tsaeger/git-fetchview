---
id: TASK-18
title: Decide whether to pin the ubuntu runner
status: To Do
assignee: []
created_date: '2026-10-22T02:54:00Z'
labels: []
dependencies: []
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub is moving the ubuntu-latest label to Ubuntu 26 (began October 19, 2026). CI passed on it, so nothing is broken. Decide between following the label and pinning ubuntu-24.04, and note why in ci.yml.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ci.yml states the choice in a comment
- [ ] #2 All jobs pass after the change
<!-- AC:END -->

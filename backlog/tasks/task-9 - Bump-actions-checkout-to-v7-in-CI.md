---
id: TASK-9
title: Bump actions/checkout to v7 in CI
status: To Do
assignee: []
created_date: '2026-10-22T00:58:00Z'
labels: []
dependencies: []
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every CI run annotates: 'Node.js 20 is deprecated. The following actions target Node.js 20 but are being forced to run on Node.js 24: actions/checkout@v4'. GitHub is already forcing Node 24, so nothing fails today, but the warning will turn into a hard break when Node 20 support is removed from runners. v5 through v7 of actions/checkout all run on Node 24 natively; v7 is the latest. The workflow uses checkout in three jobs (ci, msrv, flake).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All three actions/checkout uses in .github/workflows/ci.yml are on v7
- [ ] #2 The next CI run passes on ubuntu and macOS, msrv and flake check
- [ ] #3 The Node.js 20 deprecation annotation for checkout no longer appears
<!-- AC:END -->

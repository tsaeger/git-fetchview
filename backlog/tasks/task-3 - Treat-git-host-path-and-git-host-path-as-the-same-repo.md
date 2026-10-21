---
id: TASK-3
title: Treat git://host:/path and git://host/path as the same repo
status: To Do
assignee: []
created_date: '2026-10-21T17:52:00Z'
labels:
  - bug
dependencies: []
priority: medium
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The kernel's own history has merge subjects like 'Merge tag xfs-merge-7.4 of git://git.kernel.org:/pub/scm/fs/xfs/xfs-linux'. A colon after the host with a slash path gives a different dedupe key from git://git.kernel.org/pub/scm/fs/xfs/xfs-linux, so one tree is listed twice. Normalize an empty port in remotes::entry::key and derive_label, with a test, and check that validate_url still accepts it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 key() gives the same key for the two spellings
- [ ] #2 Test uses the real xfs-linux URL from the kernel history
<!-- AC:END -->

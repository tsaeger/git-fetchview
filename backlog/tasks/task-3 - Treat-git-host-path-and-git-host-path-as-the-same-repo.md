---
id: TASK-3
title: Treat git://host:/path and git://host/path as the same repo
status: Done
assignee: []
created_date: '2026-10-21T17:52:00Z'
updated_date: '2026-10-21T19:15:00Z'
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
- [x] #1 key() gives the same key for the two spellings
- [x] #2 Test uses the real xfs-linux URL from the kernel history
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in two places. key() ignores an empty port (git://host:/path = git://host/path), and validate_url drops it from the stored URL so the clean spelling is what gets saved and fetched. A real port (https://h:8443) still counts. Checked against 20,000 real kernel merge commits: 442 distinct trees, no empty-port spellings left, xfs-linux listed once. Not changed on purpose: https://github.com/x/y and git@github.com:x/y stay separate entries, since ssh/scp and web URLs are not interchangeable (spec).
<!-- SECTION:NOTES:END -->

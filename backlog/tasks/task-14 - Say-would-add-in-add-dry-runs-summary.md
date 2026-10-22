---
id: TASK-14
title: Say 'would add' in add --dry-run's summary
status: To Do
assignee: []
created_date: '2026-10-22T02:54:00Z'
labels: []
dependencies: []
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
'git fetchview add --all --dry-run url <url>' ends with 'added 1, tags updated 0, ...' although nothing is written. The summary should read 'would add 1' (and 'would update') on a dry run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A dry run's summary uses the conditional wording
- [ ] #2 A normal run's summary is unchanged
- [ ] #3 An integration test covers the dry-run wording
<!-- AC:END -->

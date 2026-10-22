---
id: TASK-27
title: Start the picker where you left off
status: Done
assignee: []
created_date: '2026-10-22T03:16:00Z'
updated_date: '2026-10-22T06:01:00Z'
labels: []
dependencies: []
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remember the last remote used and put the cursor on it the next time, and, when the refs list of the same remote is opened again, on the last ref. Store it in git config or a file under the git common dir (not an env var or a new config file format), per repository. It must degrade quietly when the remembered remote is gone, and be easy to turn off.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The remote list opens with the cursor on the last-used remote
- [x] #2 The refs list opens on the last-used ref when it still exists
- [x] #3 A missing remote or ref falls back to the top with no error
- [x] #4 A setting turns it off; tests cover remembered, missing and disabled; docs updated
<!-- AC:END -->

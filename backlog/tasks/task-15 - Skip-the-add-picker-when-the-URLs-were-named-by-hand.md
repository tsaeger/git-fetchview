---
id: TASK-15
title: Skip the add picker when the URLs were named by hand
status: Done
assignee: []
created_date: '2026-10-22T02:54:00Z'
updated_date: '2026-10-22T05:10:00Z'
labels: []
dependencies: []
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
'git fetchview add url <urls>' opens the picker even though the user typed the URLs and there is nothing to choose, and fails with 'no terminal' in scripts unless --all is passed. Consider skipping the picker for 'add url' (and when nothing new was found for any source) and printing the summary. Decide whether merges and maintainers keep the picker. Update README and docs/usage.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 add url with explicit URLs writes without the picker
- [x] #2 Sources that search (merges, maintainers) still show the picker
- [x] #3 Docs and the collect demo are updated if the behaviour changes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The collect demo uses --all throughout, so it still shows the same thing.
<!-- SECTION:NOTES:END -->

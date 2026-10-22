---
id: TASK-15
title: Skip the add picker when the URLs were named by hand
status: To Do
assignee: []
created_date: '2026-10-22T02:54:00Z'
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
- [ ] #1 add url with explicit URLs writes without the picker
- [ ] #2 Sources that search (merges, maintainers) still show the picker
- [ ] #3 Docs and the collect demo are updated if the behaviour changes
<!-- AC:END -->

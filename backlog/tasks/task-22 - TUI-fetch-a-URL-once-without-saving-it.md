---
id: TASK-22
title: 'TUI: fetch a URL once without saving it'
status: To Do
assignee: []
created_date: '2026-10-22T02:59:00Z'
labels: []
dependencies: []
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the add prompt, offer 'browse only': type a URL, skip the registry, and go straight to its refs. This is the one-off PR-link workflow from the README. Needs a design decision on the key (for example Ctrl-o in the prompt) and on how the remote shows in the title and trail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A typed URL can be browsed without being written to any file
- [ ] #2 The trail shows the full URL, as for any remote
- [ ] #3 Invalid URLs are rejected the same way as in add
- [ ] #4 Tests, README and docs/usage.md are updated
<!-- AC:END -->

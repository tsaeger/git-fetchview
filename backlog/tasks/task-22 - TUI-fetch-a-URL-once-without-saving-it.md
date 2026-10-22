---
id: TASK-22
title: 'TUI: fetch a URL once without saving it'
status: To Do
assignee: []
created_date: '2026-10-22T02:59:00Z'
updated_date: '2026-10-22T03:03:00Z'
labels: []
dependencies: []
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press o (open) in the remote picker to open a one-line prompt, like the one n opens for adding. Type a URL, press Enter, and go straight to its refs without writing any file. This is the one-off PR-link workflow from the README. The URL is validated like any other, and invalid ones show the reason and keep the prompt open. The trail prints the full URL as for any remote, and the refs title shows the redacted URL since the remote has no name.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 o opens the prompt; Enter goes to the URL's refs, Esc cancels
- [ ] #2 Nothing is written to any remotes file or git config
- [ ] #3 The trail shows the full URL, as for any remote, with a password left out
- [ ] #4 Invalid URLs are rejected the same way as in n
- [ ] #5 Tests cover the prompt and the hand-off to the refs list
- [ ] #6 Keys, README and docs/usage.md are updated
<!-- AC:END -->

---
id: TASK-16
title: Treat https://github.com/x/y and git@github.com:x/y as one repo
status: To Do
assignee: []
created_date: '2026-10-22T02:54:00Z'
labels: []
dependencies: []
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The same GitHub repo can be written as an https URL, an ssh:// URL or an scp-style address. They currently count as different remotes, so collecting from several sources can add the same repo twice. TASK-3 fixed git://host:/path against git://host/path only. Decide a normal form for the key (entry::key) that folds these together, without changing the stored URL.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 entry::key gives one key for https, ssh and scp-style forms of the same host and path
- [ ] #2 Unit tests cover the forms, including a .git suffix and a trailing slash
- [ ] #3 Two different hosts or paths still get different keys
<!-- AC:END -->

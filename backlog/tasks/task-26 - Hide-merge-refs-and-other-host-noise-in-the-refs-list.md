---
id: TASK-26
title: Hide merge refs and other host noise in the refs list
status: Done
assignee: []
created_date: '2026-10-22T03:16:00Z'
updated_date: '2026-10-22T05:15:00Z'
labels: []
dependencies: []
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A big GitHub repo lists refs/pull/N/head and refs/pull/N/merge for every PR, which doubles the list and the /merge refs are rarely wanted. GitLab does the same with refs/merge-requests/N/head and /merge, and also advertises refs/pipelines/*, refs/environments/*, refs/keep-around/* and similar. Gitea and Forgejo use refs/pull/N/head and /merge like GitHub. Add a setting (for example the multi-valued fetchview.hideRefs, patterns like refs/pull/*/merge) with a default that hides the merge refs and GitLab's internal namespaces, and show the count in the status line next to the existing 'N hidden'. Decide how it interacts with fetchview.hideRefsFrom (which hides refs another repo carries) and give a one-key way to show everything. Check the behaviour against a real GitLab (gitlab.freedesktop.org is already in the demo data) and a real GitHub repo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 refs/pull/*/merge and refs/merge-requests/*/merge are hidden by default and counted in the status line
- [x] #2 GitLab's pipelines, environments and keep-around refs are hidden by default
- [x] #3 fetchview.hideRefs replaces the default, and an empty value turns it off, like hideRefsFrom
- [x] #4 A key toggles showing the hidden refs without leaving the list
- [x] #5 Checked against a real GitHub repo and a real GitLab repo; tests cover the patterns
- [x] #6 README and docs/usage.md are updated
<!-- AC:END -->

---
id: TASK-25
title: Run git fv with a URL and optional ref, skipping the picker
status: To Do
assignee: []
created_date: '2026-10-22T03:16:00Z'
labels: []
dependencies: []
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
'git fv <url>' opens the refs list of that URL, and 'git fv <url> <ref>' fetches that ref and opens the viewer with no TUI. A ref may be given in full (refs/pull/998/head) or as a short form that is matched against ls-remote, and an ambiguous match is an error that lists the candidates. Shortcuts worth supporting: a bare number or #998 means the pull request (refs/pull/998/head on GitHub-style hosts, refs/merge-requests/998/head on GitLab-style hosts, chosen by what the remote advertises). The trail is printed as always, and the URL goes through validate_url. Builds on TASK-22 for the URL-in-the-TUI case; do not duplicate the validation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 git fv <url> opens that URL's refs list
- [ ] #2 git fv <url> <ref> fetches and views without a TUI, printing the trail
- [ ] #3 A short ref or #N is resolved against ls-remote; no match or several matches is a clear error
- [ ] #4 Both GitHub-style and GitLab-style pull request refs resolve from a number
- [ ] #5 Tests cover resolution, ambiguity and the URL validation; README and docs/usage.md are updated
<!-- AC:END -->

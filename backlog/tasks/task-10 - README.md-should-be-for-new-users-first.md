---
id: TASK-10
title: README.md should be for new users first
status: Done
assignee: []
created_date: '2026-10-22T01:10:00Z'
updated_date: '2026-10-22T01:18:00Z'
labels: []
dependencies: []
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reflow README.md for new-users first point of view.

Hook interest with the teaser.
Then a few of the workflows.  Most as written are kernel focused which is ok, but this can also be used for pulling down PRs from github or any other place.  Make sure PRs work actually because sometimes they do not show up in the default fetch refspecs and that might require another task.

advanced topics like development should go in docs/... and only have README.md reference that doc.

It might be nice to add an "install" prompt users can give to install this to their ai agents.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
README rewritten for new users; reference moved to docs/usage.md, install.md, development.md. PR refs checked against BurntSushi/ripgrep: ls-remote lists refs/pull/N/head and 'git fetch <url> refs/pull/998/head' works with no fetch refspec, so no follow-up task is needed. Hosts that hide pull refs from ls-remote cannot be listed by the picker.
<!-- SECTION:NOTES:END -->

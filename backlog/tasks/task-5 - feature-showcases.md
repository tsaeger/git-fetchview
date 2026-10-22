---
id: TASK-5
title: feature showcases
status: Done
assignee: []
created_date: '2026-10-21T19:40:00Z'
updated_date: '2026-10-22T02:52:00Z'
labels: []
dependencies: []
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
add vhs demos for the various features.  The README.md can point to showcase but should only include the teaser demo.

Features should have their own demo that clearly teaches, inspires, and showcases that feature.  `vhs` can be use to "record" demo.

All demos should have a consistent style and match the default kanagawa theme where applicable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each demo below is a vhs tape in docs/demo/ that starts with Output and Source docs/demo/style.tape, so all share one look
- [x] #2 Sorting: s, h/l column, r reverse; natural version order (v6.9 before v6.10); sorted-column arrow
- [x] #3 Collecting from merges: add merges over a range, multi-select with space and a
- [x] #4 Collecting by hand and managing: add url (args, --from-file or stdin, --dry-run, --all), list, remove, shared remotes file with --file and --global
- [x] #5 Hiding common refs: net-next with mainline and stable tags hidden and the hidden count, then mainline itself showing everything, and --refresh
- [x] #6 The trail: fetch, then paste the printed commands into a plain shell; password redaction shown
- [x] #7 Viewers: the same fetch in tig, git-log, fzf and a custom {} command
- [x] #8 Themes: the same screen in kanagawa, gruvbox, nord and catppuccin
- [x] #9 docs/demo/README.md indexes and embeds every demo; the main README keeps only the teaser and links to the index
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Order: sorting, hiding common refs, then the rest. Vim-style navigation is not a showcase. Completions and the man page stay README text. Demos must not need the network at record time where a cached or local fetch will do (viewers, themes, trail). setup.sh is shared. Each GIF is tracked with Git LFS.
<!-- SECTION:PLAN:END -->

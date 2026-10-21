---
id: TASK-6
title: filter-out refs
status: Done
assignee: []
created_date: '2026-10-21T21:10:00Z'
updated_date: '2026-10-21T21:26:00Z'
labels: []
dependencies: []
ordinal: 2500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
when working with linux repos I want to filter-out refs that appear in mainline and stable trees, since these are common in most git trees.  If I want to grab those refs, I will from mainline or stable tree.  so for fetchview I need to configure what gets filtered-out by default, 

normally I filter out tags that appear in these repos:
git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
git://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added fetchview.hideRefsFrom (multi-valued: a URL then any number of ref patterns), defaulting to mainline and stable tags. Hidden means same name and same commit in the listed repo. Lists are cached in .git/fetchview/hidden for a day, --refresh forces a fetch, and a failed refresh falls back to the old copy. The status line shows the hidden count. src/hide.rs holds the logic; checked against git.kernel.org on net-next (950 tags hidden).
<!-- SECTION:FINAL_SUMMARY:END -->

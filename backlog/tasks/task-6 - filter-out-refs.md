---
id: TASK-6
title: filter-out refs
status: To Do
assignee: []
created_date: '2026-10-21T21:10:00Z'
labels: []
dependencies: []
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
when working with linux repos I want to filter-out refs that appear in mainline and stable trees, since these are common in most git trees.  If I want to grab those refs, I will from mainline or stable tree.  so for fetchview I need to configure what gets filtered-out by default, 

normally I filter out tags that appear in these repos:
git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
git://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git
<!-- SECTION:DESCRIPTION:END -->

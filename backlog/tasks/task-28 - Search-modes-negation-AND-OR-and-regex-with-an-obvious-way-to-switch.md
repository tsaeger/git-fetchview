---
id: TASK-28
title: 'Search modes: negation, AND, OR and regex, with an obvious way to switch'
status: Done
assignee: []
created_date: '2026-10-22T03:16:00Z'
updated_date: '2026-10-22T05:56:00Z'
labels: []
dependencies: []
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today the search box is a case-insensitive substring match (tui/list.rs::matches). Extend it. Terms are separated by spaces and ANDed. A term starting with ! excludes (so '!rc v6.1' means v6.1 and not rc). An explicit OR is also wanted (for example a | between terms or a separate syntax; decide in design). A regex mode is wanted too. If more than one mode is needed, switching must be intuitive and visible: show the mode in the search line (the 'search:' label could become 'regex:'), toggle it with one key while typing (for example Ctrl-r), and keep the match count 'N of M'. A regex that does not compile must show an error in the box, not crash or silently match nothing. Keep the case-insensitive default. Decide whether to depend on the regex crate (watch MSRV 1.88.0 and cargo-deny) or write a small matcher; the parser should stay free of I/O so it can be fuzzed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Space-separated terms are ANDed and a leading ! excludes
- [x] #2 An explicit OR works and is documented
- [x] #3 A regex mode exists, shown in the search line, switched with one documented key
- [x] #4 An invalid regex shows an error in the box and does not change the list
- [x] #5 Unit tests cover each syntax and edge cases (empty term, lone !, escaped characters); a fuzz target covers the parser
- [x] #6 The search applies to every picker, including the add multi-select
- [x] #7 AGENTS.md, README keys table and docs/usage.md are updated, and the demos that show search are re-checked
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Demos that show search (sorting, merges, teaser) still use plain terms, which behave as before, so none needed re-recording.
<!-- SECTION:NOTES:END -->

---
id: TASK-32
title: Re-record the demos for the new keys and status lines
status: To Do
assignee: []
created_date: '2026-10-22T06:02:00Z'
updated_date: '2026-10-22T20:02:00Z'
labels: []
dependencies:
  - TASK-35
  - TASK-36
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Several changes since the demos were recorded show on screen: the remote hints now read 'l open  / i search  s sort  n new  o browse  A scan  p promote  d remove  q quit', the refs status line says 'N hidden (H)' and the refs hints are shorter, a loading spinner shows while refs load, and the search box has a regex mode. The teaser, sorting, hide and viewers GIFs show those lines. Re-record the ones that differ (docs/demo/setup.sh, then vhs), review them in Safari before committing, and consider new short demos for adding and promoting from the picker, and for the fzf-style search (! and |) and regex mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 demo.gif, sorting.gif, hide.gif and viewers.gif match the current status lines
- [ ] #2 A demo shows n, o, p and d in the remote list
- [ ] #3 A demo shows the search syntax and Ctrl-r regex
- [ ] #4 docs/demo/README.md embeds the new demos
- [ ] #5 The maintainer has reviewed each in a browser before it is committed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-35 changed the keys: demo.tape, hide.tape and sorting.tape still send h/l/r the old way and must be rewritten (h/l move the column cursor, s cycles ascending/descending/none, Enter/Esc change level). The hint lines also changed.

Since this was written: the multi-select highlights ranges with J/K, remote rows have a gutter dot, the third column is headed source, f filters a column in input boxes under the header (the cursor underline is on the box), c clears search, filters and sort, and ? opens a help overlay. The tapes, the demos README and the GIF captions need to match, and a demo of ? and f is worth adding.
<!-- SECTION:NOTES:END -->

---
id: TASK-42
title: Find out why a backspace was dropped in the pty regex test on CI
status: To Do
assignee: []
created_date: '2026-10-23T04:15:00Z'
labels:
  - deferred
dependencies: []
priority: low
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
On the v0.2.0 CI run (ubuntu-latest), the_search_box_takes_not_and_or_and_switches_to_regex_with_ctrl_r sent 16 backspaces to empty the regex box and one was lost, leaving a stray 't' so the pattern matched nothing and the wait timed out. It passes locally, including 12 copies in parallel. The test was changed to clear with c instead of counting backspaces (commit 873d0cc), so CI is green, but the cause is unknown: it could be the pty harness or an input race in the app. Deferred until a pty test flakes again.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause is found, or shown to be in the test harness
- [ ] #2 If the app drops keys under load, it is fixed with a test
<!-- AC:END -->

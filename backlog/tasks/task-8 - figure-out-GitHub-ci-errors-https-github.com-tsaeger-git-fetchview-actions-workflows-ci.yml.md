---
id: TASK-8
title: >-
  figure out GitHub ci errors
  https://github.com/tsaeger/git-fetchview/actions/workflows/ci.yml
status: Done
assignee: []
created_date: '2026-10-21T23:59:00Z'
updated_date: '2026-10-22T00:02:00Z'
labels: []
dependencies: []
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The one failed run (39677008823) was the macOS job: 'The job was not acquired by Runner of type hosted even after multiple attempts', a GitHub runner capacity problem, not a code or workflow error. The other three jobs passed. The next push (39697034104) passed all four jobs: ci on ubuntu and macOS, msrv, and flake check. Left as is: the Node.js 20 deprecation warning for actions/checkout@v4.
<!-- SECTION:DESCRIPTION:END -->

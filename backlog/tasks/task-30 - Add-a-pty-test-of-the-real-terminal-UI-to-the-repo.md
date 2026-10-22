---
id: TASK-30
title: Add a pty test of the real terminal UI to the repo
status: To Do
assignee: []
created_date: '2026-10-22T03:17:00Z'
labels: []
dependencies: []
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The layout checks I run by hand (a pty with a pyte screen, in /tmp) catch things the ratatui TestBackend tests cannot: raw-mode handling, alternate screen, resizing, and how the trail looks after the picker exits. Put a small version in tests/ that runs the built binary against a local git repository (no network), drives a few keys, and asserts on the final screen text. Decide how to do it in Rust without a Python dependency (a pty crate as a dev dependency, subject to cargo-deny and MSRV) or whether a Python test under cargo xtask is acceptable. It must be skipped cleanly where no pty is available.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test drives the real binary in a pty through remote pick, ref pick and quit
- [ ] #2 It asserts on screen text, including the status line and the printed trail
- [ ] #3 It needs no network and passes on Linux and macOS in CI
- [ ] #4 cargo xtask ci runs it, and cargo deny passes with any new dev dependency
<!-- AC:END -->

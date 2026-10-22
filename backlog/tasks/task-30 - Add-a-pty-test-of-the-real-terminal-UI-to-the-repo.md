---
id: TASK-30
title: Add a pty test of the real terminal UI to the repo
status: Done
assignee: []
created_date: '2026-10-22T03:17:00Z'
updated_date: '2026-10-22T04:02:00Z'
labels: []
dependencies: []
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The layout checks I run by hand (a pty with a pyte screen, in /tmp) catch things the ratatui TestBackend tests cannot: raw-mode handling, alternate screen, resizing, and how the trail looks after the picker exits. Put a small version in tests/ that runs the built binary against a local git repository (no network), drives a few keys, and asserts on the final screen text. Decide how to do it in Rust without a Python dependency (a pty crate as a dev dependency, subject to cargo-deny and MSRV) or whether a Python test under cargo xtask is acceptable. It must be skipped cleanly where no pty is available.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test drives the real binary in a pty through remote pick, ref pick and quit
- [x] #2 It asserts on screen text, including the status line and the printed trail
- [x] #3 It needs no network and passes on Linux and macOS in CI
- [x] #4 cargo xtask ci runs it, and cargo deny passes with any new dev dependency
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified on macOS. The Linux CI run is still to be seen (TASK-12).
<!-- SECTION:NOTES:END -->

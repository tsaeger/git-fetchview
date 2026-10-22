---
id: TASK-31
title: Add cargo-llvm-cov coverage to cargo xtask
status: Done
assignee: []
created_date: '2026-10-22T03:29:00Z'
updated_date: '2026-10-22T05:50:00Z'
labels: []
dependencies: []
ordinal: 17500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure test coverage with cargo-llvm-cov. Add 'cargo xtask coverage' that runs the workspace tests under llvm-cov and prints a summary, with --html for a local report under target/llvm-cov and --lcov for CI. Put cargo-llvm-cov and the llvm-tools component in the Nix dev shell (and rust-toolchain.toml components if needed) so it is pinned like the other tools; keep it optional locally the way cargo xtask docs is, and decide whether CI runs it. Exclude xtask, the fuzz crate, main.rs and the bin shims from the numbers. The TUI state machines and the parsers are the code we most want measured; ui.rs rendering is lower value. Decide whether to publish a number (a README badge or a CI summary) and whether to set a floor that fails CI, and write the decision down. .gitignore may need the report directory; the headers check should not trip on generated files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo xtask coverage prints a summary and --html and --lcov write reports
- [x] #2 cargo-llvm-cov and llvm-tools come from the Nix dev shell, and the task is skipped with a hint when the tool is missing locally
- [x] #3 xtask, fuzz, main.rs and the bin shims are excluded from the figures
- [x] #4 The CI, badge and minimum-coverage questions are decided and written down
- [x] #5 docs/development.md lists the task
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
96% of lines covered locally. The new coverage CI job has not run on GitHub yet (TASK-12).
<!-- SECTION:NOTES:END -->

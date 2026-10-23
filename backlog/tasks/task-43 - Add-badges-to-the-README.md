---
id: TASK-43
title: Add badges to the README
status: Done
assignee: []
created_date: '2026-10-23T05:06:00Z'
updated_date: '2026-10-23T05:06:00Z'
labels: []
dependencies: []
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a short badge row under the title in README.md. Four badges, each true today: CI status (ci.yml, which runs cargo xtask ci including deny and audit), latest GitHub release, MIT license, and the minimum Rust version (1.88, from rust-version in Cargo.toml).

Left out on purpose: crates.io and docs.rs (the crate is not published), coverage (CI runs cargo llvm-cov but does not upload the result anywhere), and stars (reads empty at 0 stars).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README.md has CI, release, license and Rust 1.88+ badges on one line under the title
- [x] #2 Each badge resolves and links to the right place
- [x] #3 cargo xtask docs passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Checked each badge URL: all return 200 as SVG, and the release and license badges read v0.2.0 and MIT. cargo xtask docs passes (markdownlint 0 issues, mermaid 6 blocks, 0 failed). CI already runs cargo deny and cargo audit through cargo xtask ci, so the CI badge covers them.
<!-- SECTION:NOTES:END -->

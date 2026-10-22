---
id: TASK-29
title: Publish GitHub releases with prebuilt binaries
status: To Do
assignee: []
created_date: '2026-10-22T03:17:00Z'
labels: []
dependencies: []
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today the README install is clone plus cargo xtask install, which needs a Rust toolchain. Publish a GitHub release for each version tag: archives of git-fetchview and git-fv with the man pages and shell completions, built by CI for Linux x86_64, Linux aarch64 and macOS arm64 (the targets the Nix shell and CI already cover), with SHA-256 checksums and release notes taken from the changelog (git-cliff, via cargo xtask changelog --tag). Decide the trigger (push of a v* tag), the build matrix, and whether to sign. Keep it in cargo xtask where possible so a release can be rehearsed locally with prep-release. Decide separately whether to also publish to crates.io, a Homebrew tap, or a Nix package, and note the decision. The README install section should then lead with the download.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pushing a v* tag builds archives for Linux x86_64, Linux aarch64 and macOS arm64 and attaches them to a GitHub release
- [ ] #2 The release has checksums and notes from the changelog
- [ ] #3 The workflow can be rehearsed without publishing (a dry-run path or workflow_dispatch)
- [ ] #4 The crates.io, Homebrew and Nix question is decided and written down
- [ ] #5 README install section and docs/install.md are updated
<!-- AC:END -->

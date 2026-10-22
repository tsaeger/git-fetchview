---
id: TASK-34
title: Make prep-release take a version, commit and tag, and print the push commands
status: Done
assignee: []
created_date: '2026-10-22T11:54:00Z'
updated_date: '2026-10-22T12:07:00Z'
labels: []
dependencies: []
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cargo xtask prep-release takes no argument today. It runs ci, dist and a cargo publish dry run, and leaves the version bump, changelog, commit, tag and push to the maintainer. Give it a version argument (v0.1.1 or 0.1.1) so it prepares the release, makes the release commit and the tag, then prints the push commands to copy and paste. It must never run git push: the push is the maintainer's last chance to review, and until then a git reset of the release commit (and deleting the tag) undoes everything. The crates.io dry run is not needed while the crate is unpublished, so drop it or put it behind a flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 prep-release takes a version, with or without a leading v, and rejects a malformed or non-increasing one
- [x] #2 It sets version in Cargo.toml and refreshes Cargo.lock
- [x] #3 It regenerates CHANGELOG.md with git-cliff --tag for that version
- [x] #4 It runs ci and dist, and cargo publish --dry-run only behind a flag
- [x] #5 It refuses to run on a dirty tree or when the tag already exists
- [x] #6 Tests cover the version parsing and the bump of Cargo.toml; docs/development.md describes the new flow
- [x] #7 It makes the release commit (Conventional Commits, signed off, per AGENTS.md) and an annotated tag, and does not push
- [x] #8 It prints the git push command for the branch and tag, ready to paste, and the commands to undo the commit and tag
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done in xtask/src/release.rs. Rehearsed in a clone: one chore(release) commit touching Cargo.toml, Cargo.lock and CHANGELOG.md, an annotated tag, nothing pushed. Unit tests cover version parsing, ordering and the manifest edit. The dirty-tree, existing-tag and failure-restore paths are not unit tested.
<!-- SECTION:NOTES:END -->

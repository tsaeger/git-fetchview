---
id: TASK-17
title: Run the fuzz targets
status: Done
assignee: []
created_date: '2026-10-22T02:54:00Z'
updated_date: '2026-10-22T05:50:00Z'
labels: []
dependencies: []
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The six fuzz targets (parse_ls_remote, parse_maintainers, parse_merge_subject, parse_remotes, parse_remotes_file, validate_url) have never been run in this setup, because cargo-fuzz and a nightly toolchain are not installed. Add them to the Nix dev shell or document how to install them, run each target for a few minutes, and fix any finding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo xtask fuzz runs from a documented setup
- [x] #2 Each target ran for at least 5 minutes with no crash, or findings are fixed with a test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All six targets ran 5 minutes each, about 160 million executions in total, no crash. Setup: nix develop .#fuzz, see docs/development.md.
<!-- SECTION:NOTES:END -->

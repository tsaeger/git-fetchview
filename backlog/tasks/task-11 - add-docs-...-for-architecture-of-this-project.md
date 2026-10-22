---
id: TASK-11
title: add docs/... for architecture of this project
status: Done
assignee: []
created_date: '2026-10-22T01:12:00Z'
updated_date: '2026-10-22T01:18:00Z'
labels: []
dependencies: []
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
add architectural docs to docs/... 

mermaid diagrams are nice.

make sure lints and checks are added for markdown and mermaid to cargo xtask.

dev tools should be optionally installed and tracked.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
docs/architecture.md with 6 Mermaid diagrams. 'cargo xtask docs' runs markdownlint-cli2 and parses Mermaid blocks via mermaid+jsdom (no browser). Tools are npm dev deps; skipped locally if missing, required when CI is set. Not yet seen running in GitHub CI.
<!-- SECTION:NOTES:END -->

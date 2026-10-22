<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add remote and ref picker (ecbd218)
- Use full remote urls in fetch and trail (db378c5)
- **remotes:** Validate and identify remote urls (8f49e9a)
- **remotes:** Read and write the remotes file (f2b2bed)
- **remotes:** Read file pointers from git config (c42f22a)
- **remotes:** Add merges, maintainers and url sources (fd3c0a0)
- List collected remotes in the picker (17d24a2)
- **tui:** Add a multi-select picker (a34741f)
- Add the add, list and remove commands (c3b3587)
- **tui:** Add s to cycle the sort order (3929cf3)
- **tui:** Follow vim motions for g, G, counts and paging (91c2ac0)
- **tui:** Sort refs by name naturally and add a sort mode (36564b3)
- Choose the viewer with fetchview.viewer (97eead5)
- **tui:** Align picker columns with a table and header (12f29a1)
- Hide refs that mainline and stable already carry (26ea37e)
- **tui:** Show the search in a box at the top and edit it with i (725c74a)

### Build

- Add git-cliff changelog and xtask task (a0bd686)
- Make deny and audit tasks usable (b3eb8e7)
- Set minimum supported rust version to 1.88 (a6dba5f)
- Add cargo xtask install (2ca0083)
- **xtask:** Skip backlog files in the header check (9be0004)
- Pin the toolchain with rust-toolchain.toml and a nix flake (ed57147)
- Run cargo xtask ci on GitHub Actions (f60a6e5)
- Track demo gifs with git lfs (d9d2a1b)
- Bump actions/checkout to v7 (9c8fb4f)
- **cliff:** Leave backlog commits out of the changelog (9a94108)

### Documentation

- Add readme and agent instructions (7a00e38)
- Regenerate changelog (47e1958)
- Add design spec for remote sources (4d28ad9)
- Add implementation plan for remote sources (78da3d9)
- Describe collecting remotes (8ba41af)
- Regenerate changelog (59a3ef8)
- Regenerate changelog (bfcbdba)
- Add scripts to record the README demo (e944215)
- Slow down the add step of the demo (85d486b)
- Add the README demo and a shared vhs style (124141a)
- Plan the feature showcase demos (ef88af7)
- Re-record the README demo with the search box (96d71d7)
- Add architecture notes and a Markdown and Mermaid check (243d4d5)
- Rewrite the README for new users (19d1020)
- Add a demo for each feature and an index (91ef380)

### Fixed

- Harden remote sources after review (90e7e09)
- **tui:** Put the cursor at the top after r (b63953e)
- Show already collected entries as (have) in add (65702b2)
- Treat an empty port as no port when comparing urls (6de0cb3)
- Stop list and --dry-run panicking when stdout closes early (354714d)
- **tui:** Hide passwords in remote URLs on screen (cfbc9cf)

### Miscellaneous

- Add license, gitignore and tool config (8ef7682)
- Ignore the _preview directory (547d6bd)
- Ignore the _preview folder under docs (8212388)

### Tests

- Add fuzz targets for the git output parsers (37349d3)
- Fuzz the remote source parsers (ee2ba54)


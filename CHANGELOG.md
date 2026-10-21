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

### Build

- Add git-cliff changelog and xtask task (a0bd686)
- Make deny and audit tasks usable (b3eb8e7)
- Set minimum supported rust version to 1.88 (a6dba5f)

### Documentation

- Add readme and agent instructions (7a00e38)
- Regenerate changelog (47e1958)
- Add design spec for remote sources (4d28ad9)
- Add implementation plan for remote sources (78da3d9)
- Describe collecting remotes (8ba41af)
- Regenerate changelog (59a3ef8)

### Fixed

- Harden remote sources after review (90e7e09)

### Miscellaneous

- Add license, gitignore and tool config (8ef7682)

### Tests

- Add fuzz targets for the git output parsers (37349d3)
- Fuzz the remote source parsers (ee2ba54)


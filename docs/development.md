<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Development

Everything goes through `cargo xtask`:

```sh
cargo xtask ci            # headers, docs, lint, check, test, doc, deny, audit
cargo xtask docs          # markdownlint + Mermaid parse (needs npm ci)
cargo xtask lint          # fmt --check + clippy
cargo xtask test
cargo xtask coverage      # test coverage summary (cargo-llvm-cov)
cargo xtask miri          # needs nightly + miri
cargo xtask fuzz          # needs nightly + cargo-fuzz, see below
cargo xtask dist          # man page + completions under target/
cargo xtask install       # binaries, man pages, completions (see install.md)
cargo xtask changelog     # regenerate CHANGELOG.md (needs git-cliff)
cargo xtask prep-release
```

`docs` uses `markdownlint-cli2` and `mermaid`, which `package.json` pins as dev
dependencies. Run `npm ci` once to install them. Without them `docs` is
skipped on your machine, and fails when `CI` is set. The Mermaid check only
parses the diagrams, so it needs no browser.

## Coverage

`cargo xtask coverage` runs the workspace tests under
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) and prints a
summary. `--html` writes a report under `target/llvm-cov/html` and `--lcov`
writes `target/llvm-cov/lcov.info`. The figures leave out `xtask`, the fuzz
crate, `main.rs` and the bin shims. The pty tests count too: they pass
`LLVM_PROFILE_FILE` on to the program they start. At the time of writing about
96% of lines are covered. The Nix dev shell has the tool and the
`llvm-tools` component. Without them the task is skipped on your machine, and
fails when `CI` is set.

CI runs it in its own job, not in `cargo xtask ci`, and puts the summary on the
run page and the lcov file in the run's artifacts. There is no minimum and no
badge: the number is there to look at. A floor would fail unrelated changes,
so revisit it once the figure settles.

## Fuzzing

Fuzzing needs a nightly compiler, so it has its own shell:

```sh
nix develop .#fuzz --command cargo xtask fuzz --all -- -max_total_time=300
nix develop .#fuzz --command cargo xtask fuzz parse_remotes_file -- -max_total_time=60
```

`--all` runs every target in turn with the same extra arguments. Without Nix,
install `cargo-fuzz` and a nightly toolchain with rustup, and the task selects
`+nightly` itself. Run it before releases and after changing a parser.

`deny` and `audit` need `cargo install cargo-deny cargo-audit`.

## Toolchain and CI

The toolchain is pinned. `rust-toolchain.toml` sets the Rust version (rustup reads
it) and `flake.nix` reads the same file, so Nix and rustup agree. `flake.lock`
pins nixpkgs and rust-overlay.

```sh
nix develop          # pinned Rust, cargo-deny, cargo-audit, git-cliff, git, node
nix develop .#msrv   # Rust 1.88.0, the minimum supported version
```

GitHub Actions (`.github/workflows/ci.yml`) runs `cargo xtask ci` inside that shell
on Linux and macOS, a `cargo check` on 1.88.0, and `nix flake check`. To move the
pins, edit `channel` in `rust-toolchain.toml`, or run `nix flake update`. The Nix
shell covers Linux and Apple silicon; nixpkgs dropped Intel macOS.

Tasks are tracked with [backlog.md](https://github.com/MrLesk/Backlog.md), installed
locally from `package.json` (nothing global), so you need node and npm
(`nix develop` provides them):

```sh
npm ci              # once: installs backlog.md into node_modules
npx backlog board
```

Tasks live in `backlog/` and are committed.
`changelog` needs `cargo install git-cliff`; `--check` fails if the file is stale,
and `--tag v1.2.3` labels the unreleased commits for a release.

Targets: Linux and macOS.

## Releases

Pushing a tag that starts with `v` runs `.github/workflows/release.yml`. It
builds an archive on Linux x86_64, Linux aarch64 and macOS arm64 with
`cargo xtask package`, and publishes a GitHub release with the archives, their
SHA-256 files and the changelog section for the tag as the notes. The
archives are built with the toolchain `rust-toolchain.toml` pins, outside the
Nix shell, because a binary linked inside it points at `/nix/store`.

To cut a release:

```sh
cargo xtask prep-release                  # ci, man page, completions, publish --dry-run
cargo xtask changelog --tag v0.2.0        # label the unreleased commits
# bump version in Cargo.toml, commit, then:
git tag v0.2.0 && git push origin main v0.2.0
```

To rehearse without publishing, run the workflow by hand from the Actions tab.
It builds the same archives and notes, uploads them as workflow artifacts, and
publishes nothing. Locally, `cargo xtask package` writes this machine's archive
to `dist/`, and `cargo xtask release-notes --unreleased` prints the notes.

Decided, and open to change:

- **Signing:** none. Each archive has a SHA-256 file. GitHub's artifact
  attestations are the next step if signatures are wanted.
- **crates.io:** not published yet. `cargo xtask prep-release` already runs a
  publish dry run, so it is one `cargo publish` away once the name is settled.
- **Homebrew and Nix:** no tap and no package. The flake only provides
  development shells. A tap that downloads these archives is the likely
  first step, and a `packages.default` in the flake the second.

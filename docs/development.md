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

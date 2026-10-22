<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Development

Everything goes through `cargo xtask`:

```sh
cargo xtask ci            # headers, docs, lint, check, test, doc, deny, audit
cargo xtask docs          # markdownlint + Mermaid parse (needs npm ci)
cargo xtask lint          # fmt --check + clippy
cargo xtask test
cargo xtask miri          # needs nightly + miri
cargo xtask fuzz          # needs nightly + cargo-fuzz
cargo xtask dist          # man page + completions under target/
cargo xtask install       # binaries, man pages, completions (see install.md)
cargo xtask changelog     # regenerate CHANGELOG.md (needs git-cliff)
cargo xtask prep-release
```

`docs` uses `markdownlint-cli2` and `mermaid`, which `package.json` pins as dev
dependencies. Run `npm ci` once to install them. Without them `docs` is
skipped on your machine, and fails when `CI` is set. The Mermaid check only
parses the diagrams, so it needs no browser.

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

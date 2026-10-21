<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# git-fetchview

Pick a remote, pick a ref, fetch it, and browse it in [tig](https://jonas.github.io/tig/) —
without checking anything out or touching your branches.

```
git fetchview        # or the short form: git fv
```

1. Choose a remote.
2. Choose one of that remote's refs (from `git ls-remote`).
3. It runs `git fetch <remote> <ref>`, then `tig FETCH_HEAD`.

It prints the plain commands as it goes, so you can paste them and do the same
thing without this tool:

```
git ls-remote git@github.com:someone/project.git
git fetch git@github.com:someone/project.git refs/pull/123/head
tig FETCH_HEAD
```

The trail uses the remote's full URL, not its name, so it works for anyone.
A password embedded in a URL is left out of the printed commands.

## Keys

| key                     | action                                      |
|-------------------------|---------------------------------------------|
| `j` `k` / `↓` `↑`       | move                                        |
| `g` `G`                 | top / bottom                                |
| `l` / `→` / `Enter`     | drop in: select and go to the next level    |
| `h` / `←` / `Esc`       | drop out (does nothing at the top level)    |
| `/`                     | search (substring, case-insensitive)        |
| `c`                     | clear the search                            |
| `q`                     | quit, from anywhere                         |

While typing a search, `Enter` confirms it and `Esc` abandons it.

## Configuration

Everything lives in `git config`, so the usual system → global → local hierarchy applies:

```
git config --global fetchview.theme kanagawa
git config fetchview.theme gruvbox      # this repository only
```

Themes: `kanagawa` (default), `gruvbox`, `nord`, `catppuccin`.
Flags override config: `git fetchview --theme nord`.

To use `git fv`, install both binaries (`git-fetchview` and `git-fv`).

## Development

Everything goes through `cargo xtask`:

```
cargo xtask ci            # lint, check, test, doc, deny, audit
cargo xtask lint          # fmt --check + clippy
cargo xtask test
cargo xtask miri          # needs nightly + miri
cargo xtask fuzz          # needs nightly + cargo-fuzz
cargo xtask dist          # man page + completions under target/
cargo xtask changelog     # regenerate CHANGELOG.md (needs git-cliff)
cargo xtask prep-release
```

`deny` and `audit` need `cargo install cargo-deny cargo-audit`.
`changelog` needs `cargo install git-cliff`; `--check` fails if the file is stale,
and `--tag v1.2.3` labels the unreleased commits for a release.

Targets: Linux and macOS.

## License

MIT

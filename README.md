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

## Collecting remotes

A kernel tree has far more interesting repos than any clone has remotes.
`git fetchview add` collects them into files, and the picker lists them after
your real remotes.

```
git fetchview add merges -n 500          # pull URLs from merge commits in HEAD
git fetchview add merges v6.11..v6.12    # or from a range
git fetchview add maintainers            # T: git lines of ./MAINTAINERS
git fetchview add url https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git
git fetchview add url --from-file urls.txt
git fetchview list
git fetchview remove netdev/net
```

`add` shows a picker of what it found: `Space` toggles a row, `a` toggles every
visible row, `/` filters, `Enter` writes, `q` writes nothing. Rows already
collected show as `(have)`. `--all` adds everything new without the picker, and
`--dry-run` prints what would be written.

Entries are written to `.git/fetchview.remotes` (shared by linked worktrees) and
that path is added to `fetchview.remotes` in the local config. To keep one set
for every clone:

```
git fetchview add --file ~/.local/share/fetchview/kernel.remotes --global merges
```

`fetchview.remotes` can list any number of files, at any config level. A
relative path is relative to the config file that names it.

Each line of a remotes file is `url<TAB>label<TAB>tags`. Lines starting with `#`
are comments and are kept when the file is rewritten. The label defaults to the
last two path segments (`netdev/net`), and the tags record where the entry came
from, so `/maintainers` in the picker shows only those.

URLs from merge subjects and `MAINTAINERS` are untrusted. Only `https`, `http`,
`git`, `ssh` and scp-style URLs are accepted. Anything else, such as `ext::`
helpers or `file:` URLs, is rejected and counted in the summary.

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

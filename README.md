<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# git-fetchview

Pick a remote, pick a ref, fetch it, and browse it in [tig](https://jonas.github.io/tig/) —
without checking anything out or touching your branches.

![Collecting kernel trees from MAINTAINERS, then fetching net-next into tig](docs/demo/demo.gif)

```
git fetchview        # or the short form: git fv
```

1. Choose a remote.
2. Choose one of that remote's refs (from `git ls-remote`).
3. It runs `git fetch <remote> <ref>`, then opens `FETCH_HEAD` in a viewer (`tig` unless you choose another).

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
| `gg` `G`                | top / bottom                                |
| `5j` `5k` `5G` `5gg`    | counts: move 5 rows, or go to row 5         |
| `Ctrl-d` `Ctrl-u`       | half a page down / up                       |
| `Ctrl-f` `Ctrl-b`       | a page down / up (also `PageDown` `PageUp`) |
| `l` / `→` / `Enter`     | drop in: select and go to the next level    |
| `h` / `←` / `Esc`       | drop out (does nothing at the top level)    |
| `/`                     | search (substring, case-insensitive)        |
| `c`                     | clear the search                            |
| `s`                     | sort mode: `h` `l` pick the column, `r` reverses, `s` `Esc` `Enter` finish |
| `r`                     | reverse the current order                   |
| `q`                     | quit, from anywhere                         |

Keys typed but not finished (`5`, `g`) show at the left of the status line, and `Esc` drops them. While typing a search, `Enter` confirms it and `Esc` abandons it. In sort mode the status line shows the columns (`default`, `label`, `url`, `tags`; refs have `default` and `label`) with the active one in brackets, and the list re-sorts as you move. Labels sort naturally, so `v6.9` comes before `v6.10`, and `r` after `s` puts the newest tags first. Sorting keeps real remotes above collected ones, and in `add` keeps rows you already have at the bottom. Moving the sort column keeps the cursor on the same row, and `r` puts it at the top of the list.

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

### Viewer

`fetchview.viewer` chooses what browses `FETCH_HEAD`. It is a preset name or a
command:

```
git config --global fetchview.viewer git-log
git config fetchview.viewer 'gitk --all'               # FETCH_HEAD is appended
git config fetchview.viewer 'git show --stat {}'       # {} is FETCH_HEAD
git fetchview --viewer fzf                             # one-off
```

| preset | runs |
|--------|------|
| `tig` (default) | `tig FETCH_HEAD` |
| `gitk` | `gitk FETCH_HEAD` |
| `git-log` | `git log --graph --decorate --stat FETCH_HEAD` |
| `fzf` | `git log --oneline --color=always FETCH_HEAD \| fzf --ansi --no-sort --preview 'git show --color=always {1}'` |
| `fugitive` | `nvim -c 'Git log --graph --oneline FETCH_HEAD'` |

A command runs through `sh -c`, so pipes work. The printed trail shows the exact
command line that ran. A viewer's exit status is ignored, since `fzf` exits 130
on `Esc`, except 127 (command not found), which is reported. The value comes from
your own git config and is trusted the way `core.pager` is. lazygit and gitui
take no revision, so they cannot be used directly.

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
cargo xtask install       # binaries, man pages, completions (see below)
cargo xtask changelog     # regenerate CHANGELOG.md (needs git-cliff)
cargo xtask prep-release
```

`deny` and `audit` need `cargo install cargo-deny cargo-audit`.

### Toolchain and CI

The toolchain is pinned. `rust-toolchain.toml` sets the Rust version (rustup reads
it) and `flake.nix` reads the same file, so Nix and rustup agree. `flake.lock`
pins nixpkgs and rust-overlay.

```
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

```
npm ci              # once: installs backlog.md into node_modules
npx backlog board
```

Tasks live in `backlog/` and are committed.
`changelog` needs `cargo install git-cliff`; `--check` fails if the file is stale,
and `--tag v1.2.3` labels the unreleased commits for a release.

Targets: Linux and macOS.

## Installing

```
cargo xtask install                  # per-user
cargo xtask install --prefix /opt/fv # or under a prefix
cargo xtask install --dry-run        # show what it would do
```

This installs `git-fetchview` and `git-fv` with `cargo install` (to `~/.cargo/bin`,
or `<prefix>/bin`), the man pages `git-fetchview.1` and `git-fv.1`, and
completions for bash, zsh and fish. Without a prefix, man pages go in
`~/.local/share/man`, bash and zsh completions under `~/.local/share`, and fish
completions in `~/.config/fish/completions`.

`git fetchview --help` is answered by git, which runs `man git-fetchview`, so the
man page has to be on your `MANPATH`. The command prints a hint if `man` can't
find it. Until then, `git fetchview -h` works.

## License

MIT

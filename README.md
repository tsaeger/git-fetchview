<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# git-fetchview

Look at someone else's branch, tag or pull request without adding a remote,
checking anything out, or touching your own branches.

![Collecting kernel trees from MAINTAINERS, then fetching net-next into tig](docs/demo/demo.gif)

```sh
git fetchview        # or the short form: git fv
```

1. Pick a remote.
2. Pick one of its refs.
3. It runs `git fetch <url> <ref>` and opens `FETCH_HEAD` in [tig](https://jonas.github.io/tig/)
   (or another viewer, see below).

It prints the plain commands as it goes, so you can paste them and do the same
thing without this tool:

```sh
git ls-remote git@github.com:someone/project.git
git fetch git@github.com:someone/project.git refs/pull/123/head
tig FETCH_HEAD
```

The trail uses the remote's full URL, not its name, so it works on anyone's
machine. A password embedded in a URL is left out of the trail and the picker.

## Install

You need git, a Rust toolchain (1.88 or newer), and a viewer. The default
viewer is `tig`. Linux and macOS are supported.

```sh
git clone https://github.com/tsaeger/git-fetchview
cd git-fetchview
cargo xtask install
```

That installs `git-fetchview` and `git-fv` into `~/.cargo/bin`, plus man pages
and shell completions. Make sure `~/.cargo/bin` is on your `PATH`. Details,
including `--prefix`, are in [docs/install.md](docs/install.md).

### Have an AI agent do it

Paste this into your coding agent:

```sh
Install git-fetchview from https://github.com/tsaeger/git-fetchview.
Clone it, run `cargo xtask install` inside the clone, and make sure
~/.cargo/bin is on my PATH. Check that `tig` is installed and tell me if it
is not. Then run `git fetchview -h` and show me the output. Do not change my
git config.
```

## Workflows

### Look at a pull request

GitHub, Gitea and similar hosts advertise pull requests as refs, so they show
up in the ref list. Inside any repository:

```sh
git fetchview add url https://github.com/BurntSushi/ripgrep
git fetchview
```

Pick `BurntSushi/ripgrep`, press `/`, type `pull/998/head` and press `Enter`
twice (once to leave the search box, once to pick the ref). You are in tig on
that PR's commits. Your checkout is unchanged. If the repository is already one of your remotes, skip the `add`.
You can also press `n` in the picker and type the URL to keep it, or `o` to
browse it once and save nothing.
Other hosts use other names (GitLab has `refs/merge-requests/N/head`, Gerrit
`refs/changes/...`), and the picker lists whatever the remote advertises.

### Browse a tree you have no remote for

Collect URLs once, then pick from them whenever you want:

```sh
git fetchview add url https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git
git fetchview add maintainers            # every T: git line in ./MAINTAINERS
git fetchview add merges -n 500          # every tree merged into HEAD recently
```

`add` shows what it found. `Space` marks a row, `a` marks all visible rows,
`Enter` writes them. Collected remotes show up in `git fetchview` after your
real ones.

### Compare what a tree carries beyond mainline

Most tags in a kernel tree are ones mainline already has. `git fetchview`
hides refs that torvalds' tree and the stable tree carry under the same name at
the same commit, so what is left is what that tree adds. The status line says
how many it hid, and opening mainline itself shows everything. See
[Hiding refs](docs/usage.md#hiding-refs-other-trees-carry).

## Keys

| key                 | action                                           |
|---------------------|--------------------------------------------------|
| `j` `k` / `↓` `↑`   | move (counts work: `5j`, `5G`)                   |
| `Enter` / `l`       | pick this one and go to the next level           |
| `Esc` / `h`         | go back (does nothing at the top)                |
| `/` or `i`          | edit the search box; `Esc` or `Enter` leaves it  |
| `Ctrl-r`            | in the search box: switch between plain and regex |
| `c`                 | clear the search                                 |
| `s` then `h` `l`    | choose the sort column; `r` reverses             |
| `H`                 | in the refs list: show or hide the hidden refs   |
| `n`                 | add a URL to your collected remotes              |
| `A`                 | collect remotes from merges, MAINTAINERS or a file |
| `o`                 | browse a URL once, without saving it             |
| `p`                 | turn a collected remote into a real git remote   |
| `d`                 | remove a collected remote, after `y`             |
| `q`                 | quit, from anywhere                              |

The full list, including paging and how sorting works, is in
[docs/usage.md](docs/usage.md#keys).

## Configuration

Everything lives in `git config`, so system, global and per-repository levels
all work:

```sh
git config --global fetchview.viewer git-log
git config --global fetchview.theme nord
```

Viewers are `tig` (default), `gitk`, `git-log`, `fzf`, `fugitive`, or any
command with `{}` standing for `FETCH_HEAD`. Themes are `kanagawa` (default),
`gruvbox`, `nord` and `catppuccin`. Flags beat config: `git fetchview --viewer
fzf`. All settings are described in [docs/usage.md](docs/usage.md#configuration).

## More

- [docs/demo/README.md](docs/demo/README.md): a short recording of each feature
- [docs/usage.md](docs/usage.md): keys, collecting remotes, hiding refs,
  viewers and themes
- [docs/install.md](docs/install.md): install options
- [docs/architecture.md](docs/architecture.md): how the code is laid out
- [docs/development.md](docs/development.md): building, testing, CI and
  releases

## License

MIT

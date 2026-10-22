<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Usage

The README covers the first steps. This is the reference.

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
| `/` / `i`               | edit the search box at the top (substring, case-insensitive) |
| `Esc` / `Enter`         | in the search box: leave it, keeping the text |
| `c`                     | clear the search                            |
| `s`                     | sort mode: `h` `l` pick the column, `r` reverses, `s` `Esc` `Enter` finish |
| `r`                     | reverse the current order                   |
| `n`                     | remote list: add a URL. `Enter` adds it to the default remotes file and puts the cursor on its row, `Esc` cancels |
| `o`                     | remote list: type a URL and go straight to its refs without saving it anywhere. A URL that is already listed opens that remote |
| `p`                     | remote list: make the collected remote a real git remote. The prompt suggests a name (`netdev/net` becomes `netdev-net`) and runs `git remote add`; the collected entry stays in its file |
| `d`                     | remote list: remove the collected remote under the cursor; `y` confirms, any other key cancels |
| `q`                     | quit, from anywhere                         |

While a remote's refs are loading, the status line shows a spinner and the
remote's name. `Esc`, `h` or `Ctrl-C` give up, kill the `git ls-remote`, and
return to the remote list. A slow or stalled remote never freezes the screen.

Keys typed but not finished (`5`, `g`) show at the left of the status line, and `Esc` drops them. The search box is always on the top line and shows how many rows match. While you edit it, characters are text (so `q` types a `q`) and the list filters as you type. `Esc` or `Enter` leaves the box and the filter stays; `c` clears it. In sort mode the status line shows the columns (`default`, `label`, `url`, `tags`; refs have `default` and `label`) with the active one in brackets, and the list re-sorts as you move. Labels sort naturally, so `v6.9` comes before `v6.10`, and `r` after `s` puts the newest tags first. Sorting keeps real remotes above collected ones, and in `add` keeps rows you already have at the bottom. Moving the sort column keeps the cursor on the same row, and `r` puts it at the top of the list.

## Collecting remotes

A kernel tree has far more interesting repos than any clone has remotes.
`git fetchview add` collects them into files, and the picker lists them after
your real remotes.

```sh
git fetchview add merges -n 500          # pull URLs from merge commits in HEAD
git fetchview add merges v6.11..v6.12    # or from a range
git fetchview add maintainers            # T: git lines of ./MAINTAINERS
git fetchview add url https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git
git fetchview add url --from-file urls.txt
git fetchview list
git fetchview remove netdev/net
```

`add merges` and `add maintainers` show a picker of what they found: `Space`
toggles a row, `a` toggles every visible row, `/` filters, `Enter` writes, `q`
writes nothing. Rows already collected show as `(have)`. `--all` adds everything
new without the picker, and `--dry-run` prints what would be written.

`add url` writes the URLs you named without asking, and any `add` with nothing
new to choose skips the picker too, so neither needs a terminal. A search with
something new still needs one, or `--all`.

Entries are written to `.git/fetchview.remotes` (shared by linked worktrees) and
that path is added to `fetchview.remotes` in the local config. To keep one set
for every clone:

```sh
git fetchview add --file ~/.local/share/fetchview/kernel.remotes --global merges
```

`fetchview.remotes` can list any number of files, at any config level. A
relative path is relative to the config file that names it.

Each line of a remotes file is `url<TAB>label<TAB>tags`. Lines starting with `#`
are comments and are kept when the file is rewritten. The label defaults to the
last two path segments (`netdev/net`), and the tags record where the entry came
from, so `/maintainers` in the picker shows only those.

Two URLs name the same repo when the host and path match, so the `https`,
`ssh://` and `git@host:path` forms of one repo count once, and a collected
entry is hidden when a real remote is the same repo. Only the host and path
are compared; a non-default port makes a different repo.

URLs from merge subjects and `MAINTAINERS` are untrusted. Only `https`, `http`,
`git`, `ssh` and scp-style URLs are accepted. Anything else, such as `ext::`
helpers or `file:` URLs, is rejected and counted in the summary.

## Configuration

Everything lives in `git config`, so the usual system → global → local hierarchy applies:

```sh
git config --global fetchview.theme kanagawa
git config fetchview.theme gruvbox      # this repository only
```

Themes: `kanagawa` (default), `gruvbox`, `nord`, `catppuccin`.
Flags override config: `git fetchview --theme nord`.

### Viewer

`fetchview.viewer` chooses what browses `FETCH_HEAD`. It is a preset name or a
command:

```sh
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

## Hiding refs other trees carry

Most tags in a kernel tree are ones mainline and stable already have, and you
would fetch those from mainline or stable. `fetchview.hideRefsFrom` lists
repositories whose refs are left out of the refs list. Each value is a URL
followed by any number of ref patterns:

```sh
git config --add fetchview.hideRefsFrom 'git://git.example.org/base.git refs/tags/* refs/heads/master'
```

With nothing set, the refs of mainline and stable are hidden:
`git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git` and
`.../stable/linux.git`, both with `refs/tags/*`. Setting the key at any level
replaces that default, and a single empty value turns hiding off.

A ref is hidden only if the listed repository has the same name at the same
commit, so a tag that reuses a name for different work still shows. Opening one
of the listed repositories shows all of its refs. The status line says how many
were hidden.

The lists are cached in `.git/fetchview/hidden` and fetched again once they are a
day old. If a refresh fails, the old copy is used and the status line says so.
`git fetchview --refresh` fetches them now.

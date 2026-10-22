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
| `Enter`                 | drop in: select and go to the next level    |
| `Esc`                   | drop out (does nothing at the top level)    |
| `h` `l` / `←` `→`       | move the column cursor (the underlined filter box) |
| `v`                     | show or hide the info panel under the list (on by default) |
| `?`                     | show every key in sections over the screen; any key closes it |
| `f`                     | remote list and `add`: filter the column under the cursor (same syntax as search); `Esc` / `Enter` leaves the box |
| `/` / `i`               | edit the search box at the top (substring, case-insensitive) |
| `Esc` / `Enter`         | in the search box: leave it, keeping the text |
| `c`                     | clear the search, the column filters and the sort |
| `s`                     | sort the column under the cursor: ascending, descending, off |
| `n`                     | remote list: add a URL. `Enter` adds it to the default remotes file and puts the cursor on its row, `Esc` cancels |
| `A`                     | remote list: collect remotes without leaving the picker. `m` scans merges (the prompt starts as `HEAD -n 500`; give a range and `-n N` as you like), `t` scans `MAINTAINERS`, `f` reads a file of URLs. What it finds opens in the `add` multi-select, and `Enter` writes your choice to the default file |
| `o`                     | remote list: type a URL and go straight to its refs without saving it anywhere. A URL that is already listed opens that remote |
| `p`                     | remote list: make the collected remote a real git remote. The prompt suggests a name (`netdev/net` becomes `netdev-net`) and runs `git remote add`; the collected entry stays in its file |
| `d`                     | remote list: remove the collected remote under the cursor; `y` confirms, any other key cancels |
| `q`                     | quit, from anywhere                         |

While a remote's refs are loading, the status line shows a spinner and the
remote's name. `Esc` or `Ctrl-C` give up, kill the `git ls-remote`, and
return to the remote list. A slow or stalled remote never freezes the screen.

Keys typed but not finished (`5`, `g`) show at the left of the status line, and `Esc` drops them. The search box is always on the top line and shows how many rows match. While you edit it, characters are text (so `q` types a `q`) and the list filters as you type. `Esc` or `Enter` leaves the box and the filter stays; `c` clears it and the sort too (the column cursor stays). There is no sort mode. `h` and `l` (or the arrows) move a column cursor, shown by an underline under that column's filter box (`name`, `url`, `source`; the refs list has one sortable column and no boxes, so there it underlines the header). `f` types into the box under the cursor: a blank box shows a dim `filter` and loses it as you start, and a box with text is tinted. Column filters take the same syntax as search, including `Ctrl-r` for regex, and the search box then searches only the rows the column filters left. `c` empties all of them. `s` sorts the column under it: the first press is ascending (`↑` in the header), the second descending (`↓`), the third turns the sort off and restores the order git gave. Moving the cursor never changes the sort, which stays on its column until you press `s` on another one. Names sort naturally, so `v6.9` comes before `v6.10`, and descending puts the newest tags first. A `●` in a two-character gutter before the name marks the real git remotes; collected ones have none. Each column is only as wide as its widest cell, and a blank column at the end takes the leftover width. When a cell does not fit, its middle is replaced by `…` so the start and end of a URL stay visible. The info panel under the list shows the row under the cursor in full, one `column: value` line per column; `v` hides it, and it hides itself when the screen is short. Sorting keeps real remotes above collected ones, and in `add` keeps rows you already have at the bottom. The cursor stays on its row when a sort changes, except that it goes to the top when the sort turns descending.

## Going straight to a ref

```sh
git fetchview <url-or-remote>          # open that repository's refs
git fetchview <url-or-remote> <ref>    # fetch it and open the viewer, no picker
```

The first argument is a URL or the name of a remote. The same checks apply as
for collected URLs: only `https`, `http`, `git`, `ssh` and scp-style URLs, and
a password in the URL is dropped. The ref is matched against what the remote
advertises, so what you type never reaches git as an option:

| you type | it means |
|----------|----------|
| `refs/tags/v6.12` | that ref, as advertised |
| `v6.12` | the branch or tag of that name (`refs/heads/v6.12`, `refs/tags/v6.12`) |
| `7` or `#7` | pull request 7: `refs/pull/7/head`, or `refs/merge-requests/7/head` |
| `pull/7/merge` | anything under `refs/` that ends the same way |

No match, or more than one (a branch and a tag of one name), is an error that
says so and lists the candidates. The trail is printed as usual.

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
toggles a row and moves to the next (on a marked row it unmarks it and moves on).
`J` and `K` (or Shift with the arrows) move and highlight the rows from where you
started; nothing is marked yet. `Space` then toggles the whole highlight by the
row it started on: if that row was marked they are all unmarked, otherwise all
marked, and the cursor lands after the range. Any other key, or `Esc`, drops the
highlight. Shift with the arrows works in iTerm2 and Ghostty but not in
Terminal.app, which sends them as plain arrows; `J` and `K` work everywhere. A
terminal cannot tell Shift-Space from Space, so you do not need to hold Shift
for the `Space`. `a` toggles every visible row, `/` filters, `Enter` writes, `q`
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
last two path segments (`netdev/net`), and the third field records where the
entry came from (`maintainers`, a merge range, a file). The picker shows it as
the `source` column, so `/maintainers` shows only those. The field is still
called `tags` in the file, so existing files keep working.

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

### Starting where you left off

The picker puts the cursor on the remote you picked last, and in that remote's
refs list on the ref you picked last, if it is still there. It is remembered
per repository, in `.git/fetchview/last`. To turn it off:

```sh
git config fetchview.remember false
```

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

### Hiding host noise

A big repository on GitHub, Gitea or GitLab advertises more than branches and
tags. Every pull request has a `refs/pull/N/merge` next to its `head`, and
GitLab adds `refs/merge-requests/N/merge`, `refs/pipelines/*`,
`refs/environments/*` and `refs/keep-around/*`. `fetchview.hideRefs` lists
patterns for refs to leave out, where `*` matches any run of characters:

```sh
git config --add fetchview.hideRefs 'refs/pull/*/merge'
```

With nothing set, those five patterns are hidden. Setting the key at any level
replaces them, and a single empty value turns this hiding off. The count joins
the other hidden refs on the status line, and `H` shows them among the rest and
hides them again. `refs/pull/N/head` is not hidden, so a pull request still
shows up.

The lists are cached in `.git/fetchview/hidden` and fetched again once they are a
day old. If a refresh fails, the old copy is used and the status line says so.
`git fetchview --refresh` fetches them now.

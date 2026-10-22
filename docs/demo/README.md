<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Demos

Short recordings of each feature. The first one is the teaser on the main
README. The rest are in the order you are likely to want them.

## Collecting from MAINTAINERS, then fetching

![Collecting kernel trees from MAINTAINERS, then fetching net-next into tig](demo.gif)

`add maintainers` lists every `T: git` tree in `MAINTAINERS`. Filter, mark and
write them, then pick one with `git fetchview`.

## Sorting

![Sorting refs: git's text order, then natural order, then reversed](sorting.gif)

Git lists tags as text, so `v6.10` lands before `v6.2`. `s` sorts the column under
the column cursor (`h` and `l` move it): ascending, descending, then off. The
header shows an arrow on the sorted column.

## Collecting from merges

![Collecting the trees merged in a release](merges.gif)

`add merges v6.11..v6.12` finds the trees Linus pulled. `Space` marks a row and moves on, `J`/`K` highlight a range for `Space`, `a`
marks every row that matches the search, `Enter` writes them.

## Adding by hand and managing the list

![add url, dry run, stdin, list, remove, and a shared remotes file](collect.gif)

`add url` takes URLs as arguments or from a file or stdin (`--from-file -`).
`--dry-run` shows what would be written, `--all` skips the picker, `list` and
`remove` manage what is collected, and `--file` with `--global` keeps one list
for every clone.

## Hiding refs other trees carry

![net-next with mainline's tags hidden, then mainline showing everything](hide.gif)

Tags that mainline and stable already have are left out, and the status line
counts them. Opening mainline itself shows everything. `--refresh` fetches the
lists again.

## The trail

![A URL with a password, the printed trail without it, and the same fetch by hand](trail.gif)

Every run prints the plain git commands. The password in the URL is left out of
the trail and the picker, and the printed fetch works without this tool.

## Viewers

![The same fetch in tig, git log, fzf and a custom command](viewers.gif)

`--viewer` or `fetchview.viewer` picks what browses `FETCH_HEAD`: a preset, or
any command with `{}` for `FETCH_HEAD`.

## Themes

![The picker in kanagawa, gruvbox, nord and catppuccin](themes.gif)

`--theme` or `fetchview.theme` picks one of four palettes.

## Recording

The GIFs are made with [vhs](https://github.com/charmbracelet/vhs) and tracked
with Git LFS. Every tape starts with `Output docs/demo/<name>.gif` and
`Source docs/demo/style.tape`, so they share one look.

Run `docs/demo/setup.sh` before each recording. It builds a release binary and
makes a scratch kernel clone in `/tmp/fv-demo` that shares objects with
`~/dev/linux`, so nothing touches your own repository. Then, from the repo root:

```sh
nix shell nixpkgs#ttyd --command vhs docs/demo/sorting.tape
```

Some tapes need network access to git.kernel.org. Each tape says so in its
header.

<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Installing

```sh
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

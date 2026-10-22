<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Installing

## From a release

Each release has an archive per platform: Linux x86_64, Linux aarch64 and macOS
arm64, named `git-fetchview-vX.Y.Z-<target>.tar.gz`. Next to each is a
`.sha256` file. They are not signed, so the checksum only tells you the
download is intact.

```sh
shasum -a 256 -c git-fetchview-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
tar xzf git-fetchview-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

The archive holds `bin/` (`git-fetchview` and `git-fv`), `share/man/man1/`,
the completions for bash, zsh and fish under `share/`, the license and the
README. Copy `bin` and `share` under any prefix on your `PATH`, for example
`~/.local` or `/usr/local`. The Linux binaries are built on the CI runner's
glibc, so a distribution older than that may not run them; build from source
there.

## From source

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

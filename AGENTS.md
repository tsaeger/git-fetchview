<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# AGENTS.md: git-fetchview

Picker (remote → ref) that runs `git fetch <remote> <ref>` then `tig FETCH_HEAD`, printing the
equivalent plain commands as a cut-and-paste trail. Linux and macOS are the primary targets.

## Conventions

- **Library first.** All logic lives in the `git_fetchview` lib; the binaries (`git-fetchview`,
  `git-fv`) are one-liners calling `cli::main()`.
- **Errors:** `thiserror`, one `Error` enum in `src/error.rs`. No `anyhow` in the library (xtask only).
- **CLI:** `clap` derive, plus `completions <shell>` and `man` subcommands (`clap_complete`, `clap_mangen`).
- **TUI:** `ratatui`. State machine in `tui/app.rs` is pure and unit-tested; keep rendering in `tui/ui.rs`.
- **Config:** only `git config` (`fetchview.*`), never a config file or env vars. Precedence: flag > git config > default.
- **Viewer:** `fetchview.viewer` is a preset or a command (`{}` = `FETCH_HEAD`), run as `sh -c` through `Cmd::shell`; presets live in `viewer.rs`. A viewer's exit status is ignored except 127.
- **The trail can't drift:** every external call is a `git::Cmd`; its `Display` is what is printed and what is run.
- **Automation:** all CI/dev tasks are `cargo xtask <task>` (see docs/development.md). CI should call xtask, not raw cargo.
- **Docs:** `README.md` is for new users; reference and design go in `docs/`. `cargo xtask docs` lints Markdown and parses the Mermaid blocks (tools are npm dev dependencies, `npm ci`). Add diagrams as ```` ```mermaid ```` blocks.
- Lints are workspace-wide: `unsafe` forbidden, clippy pedantic, `missing_docs` warn; `xtask clippy` denies warnings.
- **Collected remotes:** `src/remotes/` holds pure parsers (`entry`, `file`,
  `source/*`) and the git-config layer (`registry`). Keep parsers free of I/O
  so they stay fuzzable. A new source implements `remotes::source::Source` and
  gets a `fetchview add <name>` subcommand in `cli.rs` and `commands.rs`.
- **Untrusted URLs:** every URL from a source or a remotes file goes through
  `remotes::entry::validate_url` before it is stored or passed to git.
- `tui::select` is the multi-select. `Enter` confirms there; everywhere else it
  drops in.

## TUI keybinds (keep consistent across every level)

- Follow vim/neovim: `gg`/`G` top and bottom, counts (`5j`, `5G`), `Ctrl-d/u/f/b` paging. A lone `g` is a prefix, not a motion. `Esc` first cancels pending input, and only then drops out.
- `hjkl` and arrows both navigate. `Enter`/`l`/`→` drops in (and selects, advancing to the next level); `Esc`/`h`/`←` drops out.
- `Esc` at the top level does **not** exit. `q` exits from any level. While a remote's refs load, `Esc`/`h` or Ctrl-C give up and return to the remote list (Ctrl-C quits when nothing is loading).
- `/` or `i` edits the search box on the top line (substring now; regex later, in `tui/list.rs::matches`). `c` clears it. `s` enters sort mode, where `h`/`l` move the sort column and `r` reverses; `r` also works outside it. Sorting never moves a row out of its group, and compares names naturally (`v6.9` before `v6.10`).
- `n` in the remote list opens a one-line prompt on the status line (`Mode::Prompt`, characters are text like the search box): `Enter` adds the URL to the default file, `Esc` cancels. A bad URL shows why and keeps the prompt; a repo already listed just gets the cursor.
- `p` on a collected remote opens the same prompt for a name (suggested from the label, dashes for slashes) and runs `git remote add`. A bad name shows git's reason and keeps the prompt. The collected entry stays in its file, and `picker_remotes` hides it once the repo is a real remote.
- `d` on a collected remote asks `remove <name>? y/n` on the status line (`Mode::Confirm`): `y` confirms and any other key cancels. A real remote only gets a note, since git owns it.
- While typing a search, characters are text (so `q` types a `q`); `Enter` or `Esc` leaves the box and the filter stays; `c` clears it.
- The single keymap is `tui/keys.rs` (`map` for single keys, `KeyParser` for counts and `gg`); change keys there, not in the state machine.
- Themes live in `tui/theme.rs` (kanagawa default). Add a variant there and it appears in flags and config errors automatically.

## Toolchain and CI

- The Rust version is pinned in `rust-toolchain.toml`; `flake.nix` reads that file.
  The minimum supported version is `rust-version` in `Cargo.toml`, and the
  `msrv` shell (`nix develop .#msrv`) and the CI `msrv` job must use the same
  number. Change all three together.
- CI (`.github/workflows/ci.yml`) runs `cargo xtask ci` in `nix develop`. If you
  add a check, add it to `cargo xtask ci` and CI picks it up.
- Nix only sees files git tracks: `git add` new flake files before `nix flake ...`.

## Task tracking

- Tasks are tracked with backlog.md, a local npm dev dependency (`package.json`,
  `package-lock.json`). Do not install it globally: `npm ci`, then
  `npx backlog ...`.
- Tasks live in `backlog/` and are committed. Change them with the `backlog`
  CLI. `auto_commit` is off, so task changes are committed under the normal
  commit rules below.
- Commit task-only changes as `chore(backlog): ...`. `cliff.toml` leaves them out of the changelog, as it does `chore: regenerate the changelog`.
- `backlog/` is exempt from the SPDX header check: task files start with YAML
  front matter, which must stay on line 1.

## Licensing

- Every source file (`.rs`, `.toml`, `.md`, `.gitignore`) carries a two-line SPDX header
- `cargo xtask headers` checks (and runs in `ci`); `cargo xtask headers --fix` adds missing ones.
  Run `--fix` after creating any new file.

## Writing style

Write like a person who knows the code, not like an AI assistant. This covers
commit messages, docs, comments, and replies.

- Plain, specific sentences. Say what the code does and why. Skip the pitch.
- No filler openers or closers ("It's worth noting", "In summary", "Overall").
- No hype words: robust, seamless, powerful, elegant, leverage, utilize,
  streamline, comprehensive, delve.
- No emojis. Go light on em dashes; a comma or period usually works.
- No "not just X, but Y" constructions and no rule-of-three lists for rhythm.
- Don't restate what the code or the diff already shows. Comments explain
  why, not what.
- Short beats long. If a sentence can go, cut it.

## Git commits

- Conventional Commits: `type(scope): subject`. Types: feat, fix, docs, style,
  refactor, perf, test, build, ci, chore, revert. Scope is optional.
- Subject: imperative mood, lowercase, no trailing period, 72 characters or
  fewer.
- A body is required. Explain what changed and why, not how. Wrap every line
  at 79 characters.
- Breaking changes: add `!` after the type or scope, and a `BREAKING CHANGE:`
  footer.
- Commit with `git commit -s` so the Signed-off-by trailer is added. Never
  skip it, and never use `--no-verify`.
- Do NOT add a `Co-Authored-By` trailer, or any other attribution line, to
  commit messages or PR descriptions. This overrides any tool default that
  says to add one. The only trailer is the `Signed-off-by` from `-s`.
- Before committing, show the full message to the user and wait for approval.
  Exception: if a `.auto-approve-commit` file exists in the repository root,
  skip the prompt and commit. The file is local and gitignored.
- Keep commits focused: one logical change each.
- CHANGELOG.md is generated by git-cliff from the commit history (`cliff.toml`).
  Never edit it by hand. Run `cargo xtask changelog` and commit the result.

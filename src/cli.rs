// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Command line: argument parsing and the top-level flow.

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::commands;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::git::{self, Cmd};
use crate::remotes::{self, registry};
use crate::tui::{self, theme::ThemeName};

/// Pick a remote and ref, fetch it, and open it in a viewer (tig by default).
///
/// Settings come from `git config` (system, global, then local):
/// `fetchview.theme`, `fetchview.viewer`. Flags override config.
#[derive(Debug, Parser)]
#[command(name = "git-fetchview", version, about)]
pub struct Cli {
    /// Colour theme (overrides `fetchview.theme`).
    #[arg(long, value_enum)]
    pub theme: Option<ThemeName>,

    /// Viewer for `FETCH_HEAD`: tig, gitk, git-log, fzf, fugitive, or a command
    /// in which `{}` stands for `FETCH_HEAD` (overrides `fetchview.viewer`).
    #[arg(long)]
    pub viewer: Option<String>,

    /// Refetch the lists of refs to hide, even if the cache is recent
    /// (see `fetchview.hideRefsFrom`).
    #[arg(long)]
    pub refresh: bool,

    /// Subcommand; omit to start the picker.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Auxiliary subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print shell completions to stdout.
    Completions {
        /// Target shell.
        shell: Shell,
    },
    /// Print the man page (roff) to stdout.
    Man,
    /// Collect remotes from a source into a remotes file.
    Add(AddArgs),
    /// List collected remotes.
    List,
    /// Remove a collected remote by URL or label.
    Remove {
        /// A URL or an exact label.
        target: String,
    },
}

/// Arguments for `add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Write to this file instead of `.git/fetchview.remotes`, and register it.
    #[arg(long, global = true)]
    pub file: Option<PathBuf>,

    /// Register the file in `~/.gitconfig` instead of the local config.
    #[arg(long, requires = "file", global = true)]
    pub global: bool,

    /// Add every new remote without opening the picker.
    #[arg(long, global = true)]
    pub all: bool,

    /// Print what would be written and change nothing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Where to look.
    #[command(subcommand)]
    pub source: AddSource,
}

/// Where `add` looks for remotes.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum AddSource {
    /// Pull URLs from merge commit subjects.
    Merges {
        /// Stop after this many merges.
        #[arg(short = 'n', long)]
        max: Option<usize>,
        /// Revision range (default: HEAD).
        range: Option<String>,
    },
    /// Pull `T: git` URLs from a MAINTAINERS file.
    Maintainers {
        /// Path to the file (default: MAINTAINERS at the worktree root).
        path: Option<PathBuf>,
    },
    /// Add URLs given by hand.
    Url {
        /// Read URLs from this file, one per line; `-` is stdin.
        #[arg(long)]
        from_file: Option<String>,
        /// URLs to add.
        urls: Vec<String>,
    },
}

/// Parse arguments, run, and turn errors into an exit code. Shared by both binaries.
#[must_use]
pub fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("git-fetchview: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Command::Completions { shell }) => {
            clap_complete::generate(
                *shell,
                &mut Cli::command(),
                "git-fetchview",
                &mut io::stdout(),
            );
            Ok(())
        }
        Some(Command::Man) => Ok(clap_mangen::Man::new(Cli::command()).render(&mut io::stdout())?),
        Some(Command::Add(args)) => commands::add(args),
        Some(Command::List) => commands::list(),
        Some(Command::Remove { target }) => commands::remove(target),
        None => pick_and_view(cli.theme, cli.viewer.as_deref(), cli.refresh),
    }
}

/// One line for the status bar: the first warning and how many more there are.
fn notice(warnings: &[String]) -> Option<String> {
    match warnings {
        [] => None,
        [one] => Some(one.clone()),
        [first, rest @ ..] => Some(format!("{first} (+{} more)", rest.len())),
    }
}

fn pick_and_view(
    theme_flag: Option<ThemeName>,
    viewer_flag: Option<&str>,
    refresh: bool,
) -> Result<()> {
    let config = Config::load(theme_flag, viewer_flag)?;
    let loaded = registry::load_registered();
    let remotes = remotes::picker_remotes(git::configured_remotes()?, &loaded.merged());
    if remotes.is_empty() {
        return Err(Error::NoRemotes);
    }
    let hide = tui::HideRefs {
        rules: config.hide,
        refresh,
        patterns: config.hide_refs,
    };
    let Some(selection) = tui::pick(remotes, config.theme, notice(&loaded.warnings), &hide)? else {
        return Ok(());
    };

    // Use the expanded URL rather than the remote name, so the trail works
    // for anyone, not only in a clone that happens to have this remote.
    let fetch = Cmd::fetch(&selection.url, &selection.refname);
    let viewer = Cmd::shell(&config.viewer);

    // The trail: paste these to do the same thing without git-fetchview.
    // It is shared with others, so embedded passwords are removed.
    let shared_url = git::redact_url(&selection.url);
    eprintln!("{}", Cmd::ls_remote(&shared_url));
    eprintln!("{}", Cmd::fetch(&shared_url, &selection.refname));
    eprintln!("{viewer}");

    fetch.status()?;
    viewer.run_viewer()
}

#[cfg(test)]
mod tests {
    use super::notice;

    #[test]
    fn notice_summarizes_warnings() {
        assert_eq!(notice(&[]), None);
        assert_eq!(notice(&["a".into()]), Some("a".into()));
        assert_eq!(
            notice(&["a".into(), "b".into(), "c".into()]),
            Some("a (+2 more)".into())
        );
    }
}

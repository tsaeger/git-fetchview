// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Command line: argument parsing and the top-level flow.

use std::io;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::git::{self, Cmd};
use crate::remotes::{self, registry};
use crate::tui::{self, theme::ThemeName};

/// Pick a remote and ref, fetch it, and open it in tig.
///
/// Settings come from `git config` (system, global, then local):
/// `fetchview.theme`. Flags override config.
#[derive(Debug, Parser)]
#[command(name = "git-fetchview", version, about)]
pub struct Cli {
    /// Colour theme (overrides `fetchview.theme`).
    #[arg(long, value_enum)]
    pub theme: Option<ThemeName>,

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
        None => pick_and_view(cli.theme),
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

fn pick_and_view(theme_flag: Option<ThemeName>) -> Result<()> {
    let config = Config::load(theme_flag)?;
    let loaded = registry::load(&registry::files()?);
    let remotes = remotes::picker_remotes(git::configured_remotes()?, &loaded.merged());
    if remotes.is_empty() {
        return Err(Error::NoRemotes);
    }
    let Some(selection) = tui::pick(remotes, config.theme, notice(&loaded.warnings))? else {
        return Ok(());
    };

    // Use the expanded URL rather than the remote name, so the trail works
    // for anyone, not only in a clone that happens to have this remote.
    let fetch = Cmd::fetch(&selection.url, &selection.refname);
    let tig = Cmd::tig_fetch_head();

    // The trail: paste these to do the same thing without git-fetchview.
    // It is shared with others, so embedded passwords are removed.
    let shared_url = git::redact_url(&selection.url);
    eprintln!("{}", Cmd::ls_remote(&shared_url));
    eprintln!("{}", Cmd::fetch(&shared_url, &selection.refname));
    eprintln!("{tig}");

    fetch.status()?;
    tig.status()
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

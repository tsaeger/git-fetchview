// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Command line: argument parsing and the top-level flow.

use std::io;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::config::Config;
use crate::error::Result;
use crate::git::{self, Cmd};
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

fn pick_and_view(theme_flag: Option<ThemeName>) -> Result<()> {
    let config = Config::load(theme_flag)?;
    let remotes = git::remotes()?;
    let Some(selection) = tui::pick(remotes, config.theme)? else {
        return Ok(());
    };

    let ls_remote = Cmd::ls_remote(&selection.remote);
    let fetch = Cmd::fetch(&selection.remote, &selection.refname);
    let tig = Cmd::tig_fetch_head();

    // The trail: paste these to do the same thing without git-fetchview.
    eprintln!("{ls_remote}");
    eprintln!("{fetch}");
    eprintln!("{tig}");

    fetch.status()?;
    tig.status()
}

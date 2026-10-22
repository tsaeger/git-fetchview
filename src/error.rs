// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Error type for the library.

use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// Everything that can go wrong in `git-fetchview`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A command could not be started (e.g. `tig` is not installed).
    #[error("failed to run `{command}`: {source}")]
    Spawn {
        /// The command line, as it would be pasted into a shell.
        command: String,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },

    /// A command ran but exited unsuccessfully.
    #[error("`{command}` failed ({status}){}", fmt_stderr(.stderr))]
    Failed {
        /// The command line, as it would be pasted into a shell.
        command: String,
        /// The exit status.
        status: ExitStatus,
        /// Captured standard error, if the command's output was captured.
        stderr: String,
    },

    /// A command printed something that is not UTF-8.
    #[error("`{command}` produced output that is not valid UTF-8")]
    NotUtf8 {
        /// The command line, as it would be pasted into a shell.
        command: String,
    },

    /// The viewer command could not be found (exit status 127).
    #[error(
        "viewer not found: `{command}` (set fetchview.viewer to a command, or one of: {})",
        crate::viewer::preset_names()
    )]
    ViewerNotFound {
        /// The command line that was run.
        command: String,
    },

    /// The repository has no remotes.
    #[error("this repository has no remotes")]
    NoRemotes,

    /// A command was stopped on request before it finished.
    #[error("`{command}` was cancelled")]
    Cancelled {
        /// The command line, as it would be pasted into a shell.
        command: String,
    },

    /// The chosen remote advertises no refs.
    #[error("remote `{0}` has no refs")]
    NoRefs(String),

    /// A command-line flag value could not be understood.
    #[error("invalid value for {flag}: `{value}`: {reason}")]
    BadFlag {
        /// The flag, e.g. `--viewer`.
        flag: &'static str,
        /// The offending value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },

    /// A `git config` value could not be understood.
    #[error("invalid git config `{key}` = `{value}`: {reason}")]
    BadConfig {
        /// The config key.
        key: &'static str,
        /// The offending value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },

    /// Reading or writing a file failed.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },

    /// `add` needs to ask which remotes to add, but there is no terminal.
    #[error("no terminal to choose from; pass --all to add every new remote")]
    NotATerminal,

    /// `remove` matched nothing.
    #[error("no collected remote matches `{0}`")]
    NotFound(String),

    /// `remove` matched more than one repo.
    #[error("`{target}` matches more than one remote:\n{matches}")]
    Ambiguous {
        /// What the user typed.
        target: String,
        /// One `label  url` per line.
        matches: String,
    },

    /// Terminal I/O failed.
    #[error("terminal error: {0}")]
    Terminal(#[from] io::Error),
}

fn fmt_stderr(stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    }
}

/// Library result alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Error type for the library.

use std::io;
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

    /// The repository has no remotes.
    #[error("this repository has no remotes")]
    NoRemotes,

    /// The chosen remote advertises no refs.
    #[error("remote `{0}` has no refs")]
    NoRefs(String),

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

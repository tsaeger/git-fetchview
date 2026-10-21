// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Configuration lives in `git config`, so the usual system, global and local
//! (per-repository) hierarchy applies for free.
//!
//! ```text
//! git config --global fetchview.theme kanagawa
//! git config fetchview.theme gruvbox        # this repository only
//! ```
//!
//! Precedence: command-line flag, then `git config`, then the built-in default.

use crate::error::{Error, Result};
use crate::git::Cmd;
use crate::tui::theme::ThemeName;

/// Config key for the TUI theme.
pub const THEME_KEY: &str = "fetchview.theme";

/// Resolved settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    /// TUI colour theme.
    pub theme: ThemeName,
}

impl Config {
    /// Read settings from `git config`; `theme_flag` wins over config.
    ///
    /// # Errors
    /// Git cannot be run, or a configured value is not recognised.
    pub fn load(theme_flag: Option<ThemeName>) -> Result<Self> {
        let theme = match theme_flag {
            Some(theme) => theme,
            None => match get(THEME_KEY)? {
                Some(value) => value.parse().map_err(|reason| Error::BadConfig {
                    key: THEME_KEY,
                    value,
                    reason,
                })?,
                None => ThemeName::default(),
            },
        };
        Ok(Self { theme })
    }
}

/// Read one key; an unset key is `None`, not an error.
fn get(key: &str) -> Result<Option<String>> {
    match Cmd::config_get(key).output() {
        Ok(value) => Ok(Some(value.trim().to_owned()).filter(|v| !v.is_empty())),
        // `git config --get` exits 1 when the key is unset.
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e),
    }
}

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Configuration lives in `git config`, so the usual system, global and local
//! (per-repository) hierarchy applies for free.
//!
//! ```text
//! git config --global fetchview.theme kanagawa
//! git config --global fetchview.viewer gitk        # or a command; {} is FETCH_HEAD
//! git config fetchview.theme gruvbox        # this repository only
//! ```
//!
//! Precedence: command-line flag, then `git config`, then the built-in default.

use crate::error::{Error, Result};
use crate::git::Cmd;
use crate::hide::{self, Rule};
use crate::tui::theme::ThemeName;
use crate::viewer;

/// Config key for the TUI theme.
pub const THEME_KEY: &str = "fetchview.theme";

/// Config key for the viewer that browses `FETCH_HEAD`.
pub const VIEWER_KEY: &str = "fetchview.viewer";

/// Resolved settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// TUI colour theme.
    pub theme: ThemeName,
    /// The shell command line that browses `FETCH_HEAD`.
    pub viewer: String,
    /// Repositories whose refs the refs picker leaves out.
    pub hide: Vec<Rule>,
}

impl Config {
    /// Read settings from `git config`; the flags win over config.
    ///
    /// # Errors
    /// Git cannot be run, or a configured value is not recognised.
    pub fn load(theme_flag: Option<ThemeName>, viewer_flag: Option<&str>) -> Result<Self> {
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
        let configured = get(VIEWER_KEY)?;
        let viewer =
            viewer::choose(viewer_flag, configured.as_deref()).map_err(
                |reason| match viewer_flag {
                    Some(value) => Error::BadFlag {
                        flag: "--viewer",
                        value: value.to_owned(),
                        reason,
                    },
                    None => Error::BadConfig {
                        key: VIEWER_KEY,
                        value: configured.clone().unwrap_or_default(),
                        reason,
                    },
                },
            )?;
        let hide_values = get_all(hide::KEY)?;
        let hide = hide::parse_rules(hide_values.as_deref()).map_err(|(value, reason)| {
            Error::BadConfig {
                key: hide::KEY,
                value,
                reason,
            }
        })?;
        Ok(Self {
            theme,
            viewer,
            hide,
        })
    }
}

/// Read every value of a multi-valued key, one per line; unset is `None`. A
/// key set to an empty string gives `Some` of a blank line.
fn get_all(key: &str) -> Result<Option<String>> {
    match Cmd::config_get_all(key).output() {
        Ok(values) => Ok(Some(values)),
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e),
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

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! `git-fetchview`: pick a remote and a ref, fetch it, and browse it in tig.
//!
//! Every step is also printed as a plain command, so you can learn the
//! workflow and then do it without this tool.

pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod git;
pub mod remotes;
pub mod tui;

pub use error::{Error, Result};

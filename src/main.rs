// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! `git fetchview` entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    git_fetchview::cli::main()
}

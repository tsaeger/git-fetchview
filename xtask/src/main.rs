// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Project automation: `cargo xtask <task>`.
//!
//! CI runs exactly these tasks, so anything that passes locally passes there.

mod headers;

use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Project automation")]
struct Args {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Everything CI runs: headers, lint, test, docs, deny, audit.
    Ci,
    /// `cargo check` on all targets.
    Check,
    /// Clippy with warnings denied.
    Clippy,
    /// rustfmt check (or rewrite with --fix).
    Fmt {
        /// Rewrite files instead of checking.
        #[arg(long)]
        fix: bool,
    },
    /// fmt check + clippy.
    Lint,
    /// Run the test suite.
    Test,
    /// Build docs with warnings denied.
    Doc,
    /// cargo-deny: licenses, bans, advisories, sources.
    Deny,
    /// cargo-audit: advisory database check.
    Audit,
    /// Tests under Miri (needs nightly + miri).
    Miri,
    /// Run a cargo-fuzz target (needs nightly + cargo-fuzz).
    Fuzz {
        /// Fuzz target name.
        #[arg(default_value = "parse_ls_remote")]
        target: String,
        /// Extra arguments after `--` (e.g. `-max_total_time=60`).
        #[arg(last = true)]
        extra: Vec<String>,
    },
    /// Check SPDX headers on all source files (or add them with --fix).
    Headers {
        /// Insert missing headers instead of failing.
        #[arg(long)]
        fix: bool,
    },
    /// Generate man page and shell completions under target/.
    Dist,
    /// Everything required before cutting a release.
    PrepRelease,
}

fn main() -> ExitCode {
    match run(Args::parse().task) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(task: Task) -> Result<()> {
    match task {
        Task::Ci => {
            run(Task::Headers { fix: false })?;
            run(Task::Lint)?;
            run(Task::Check)?;
            run(Task::Test)?;
            run(Task::Doc)?;
            run(Task::Deny)?;
            run(Task::Audit)
        }
        Task::Check => cargo(&["check", "--workspace", "--all-targets"]),
        Task::Clippy => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        Task::Fmt { fix } => cargo(if fix {
            &["fmt", "--all"]
        } else {
            &["fmt", "--all", "--check"]
        }),
        Task::Lint => {
            run(Task::Fmt { fix: false })?;
            run(Task::Clippy)
        }
        Task::Test => cargo(&["test", "--workspace"]),
        Task::Doc => {
            let mut cmd = Command::new("cargo");
            cmd.args(["doc", "--workspace", "--no-deps"])
                .env("RUSTDOCFLAGS", "-D warnings");
            exec(cmd)
        }
        Task::Deny => cargo(&["deny", "check"]),
        Task::Audit => cargo(&["audit"]),
        Task::Miri => cargo(&["+nightly", "miri", "test", "--workspace"]),
        Task::Fuzz { target, extra } => {
            let mut args = vec!["+nightly", "fuzz", "run", &target];
            if !extra.is_empty() {
                args.push("--");
                args.extend(extra.iter().map(String::as_str));
            }
            cargo(&args)
        }
        Task::Headers { fix } => headers::run(fix),
        Task::Dist => dist(),
        Task::PrepRelease => {
            run(Task::Ci)?;
            run(Task::Dist)?;
            cargo(&["publish", "--dry-run", "--package", "git-fetchview"])
        }
    }
}

fn dist() -> Result<()> {
    std::fs::create_dir_all("target/man")?;
    std::fs::create_dir_all("target/completions")?;
    let man = capture(&["run", "--quiet", "--bin", "git-fetchview", "--", "man"])?;
    std::fs::write("target/man/git-fetchview.1", man)?;
    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        let out = capture(&[
            "run",
            "--quiet",
            "--bin",
            "git-fetchview",
            "--",
            "completions",
            shell,
        ])?;
        std::fs::write(format!("target/completions/git-fetchview.{shell}"), out)?;
    }
    eprintln!("wrote target/man and target/completions");
    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(args);
    exec(cmd)
}

fn capture(args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("cargo")
        .args(args)
        .output()
        .context("failed to run cargo")?;
    if !out.status.success() {
        bail!(
            "cargo {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(out.stdout)
}

fn exec(mut cmd: Command) -> Result<()> {
    eprintln!("$ {cmd:?}");
    let status = cmd
        .status()
        .with_context(|| format!("failed to start {cmd:?}"))?;
    if !status.success() {
        bail!("{cmd:?} exited with {status}");
    }
    Ok(())
}

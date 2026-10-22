// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Project automation: `cargo xtask <task>`.
//!
//! CI runs exactly these tasks, so anything that passes locally passes there.

mod headers;
mod install;

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
    /// Run a cargo-fuzz target (needs nightly + cargo-fuzz; see development.md).
    Fuzz {
        /// Fuzz target name.
        #[arg(default_value = "parse_ls_remote", conflicts_with = "all")]
        target: String,
        /// Run every target in turn, each with the same extra arguments.
        #[arg(long)]
        all: bool,
        /// Extra arguments after `--` (e.g. `-max_total_time=60`).
        #[arg(last = true)]
        extra: Vec<String>,
    },
    /// Lint Markdown and check the Mermaid diagrams in it (needs `npm ci`).
    Docs {
        /// Fail if the tools are not installed, instead of skipping.
        #[arg(long)]
        require: bool,
    },
    /// Test coverage with cargo-llvm-cov: a summary, or --html / --lcov reports.
    Coverage {
        /// Write an HTML report under target/llvm-cov/html.
        #[arg(long)]
        html: bool,
        /// Write target/llvm-cov/lcov.info, for tools that read lcov.
        #[arg(long)]
        lcov: bool,
        /// Fail if the tool is not installed, instead of skipping.
        #[arg(long)]
        require: bool,
    },
    /// Check SPDX headers on all source files (or add them with --fix).
    Headers {
        /// Insert missing headers instead of failing.
        #[arg(long)]
        fix: bool,
    },
    /// Regenerate CHANGELOG.md with git-cliff (or verify it with --check).
    Changelog {
        /// Fail if CHANGELOG.md is out of date instead of writing it.
        #[arg(long)]
        check: bool,
        /// Label unreleased commits as this version, e.g. v0.2.0.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Generate man page and shell completions under target/.
    Dist,
    /// Build and install the binaries, man pages and shell completions.
    Install {
        /// Install under this prefix (`<prefix>/bin`, `<prefix>/share/...`)
        /// instead of the per-user directories.
        #[arg(long)]
        prefix: Option<std::path::PathBuf>,
        /// Show what would be done and change nothing.
        #[arg(long)]
        dry_run: bool,
    },
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
            run(Task::Docs { require: false })?;
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
        Task::Deny => {
            require_subcommand("deny", "cargo-deny")?;
            cargo(&["deny", "--workspace", "check", "--hide-inclusion-graph"])
        }
        Task::Audit => {
            require_subcommand("audit", "cargo-audit")?;
            cargo(&["audit"])
        }
        Task::Miri => cargo(&["+nightly", "miri", "test", "--workspace"]),
        Task::Fuzz { target, all, extra } => {
            let targets = if all { fuzz_targets()? } else { vec![target] };
            for target in targets {
                fuzz(&target, &extra)?;
            }
            Ok(())
        }
        Task::Coverage {
            html,
            lcov,
            require,
        } => coverage(html, lcov, require),
        Task::Docs { require } => docs(require),
        Task::Headers { fix } => headers::run(fix),
        Task::Changelog { check, tag } => changelog(check, tag.as_deref()),
        Task::Dist => dist(),
        Task::Install { prefix, dry_run } => install::run(prefix.as_deref(), dry_run),
        Task::PrepRelease => {
            run(Task::Ci)?;
            run(Task::Dist)?;
            cargo(&["publish", "--dry-run", "--package", "git-fetchview"])
        }
    }
}

/// What the coverage figures leave out: the project's own tooling, the fuzz
/// crate, and the two lines of `main` that only call into the library.
const COVERAGE_IGNORE: &str = r"(^|/)(xtask|fuzz)/|src/main\.rs|src/bin/";

/// Test coverage with `cargo llvm-cov`. Like the docs checks it is optional on
/// a laptop, and required when `CI` is set.
fn coverage(html: bool, lcov: bool, require: bool) -> Result<()> {
    let found = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output()
        .is_ok_and(|out| out.status.success());
    if !found {
        if require || std::env::var_os("CI").is_some() {
            bail!(
                "`cargo llvm-cov` is not installed (nix develop, or cargo install --locked cargo-llvm-cov)"
            );
        }
        eprintln!("xtask: skipping coverage, cargo-llvm-cov is not installed (nix develop has it)");
        return Ok(());
    }
    let base = [
        "llvm-cov",
        "--workspace",
        "--ignore-filename-regex",
        COVERAGE_IGNORE,
    ];
    // One run of the tests, then each report from its data.
    cargo(&[&base[..], &["--no-report"]].concat())?;
    if html {
        cargo(&[
            "llvm-cov",
            "report",
            "--ignore-filename-regex",
            COVERAGE_IGNORE,
            "--html",
        ])?;
    }
    if lcov {
        std::fs::create_dir_all("target/llvm-cov")?;
        cargo(&[
            "llvm-cov",
            "report",
            "--ignore-filename-regex",
            COVERAGE_IGNORE,
            "--lcov",
            "--output-path",
            "target/llvm-cov/lcov.info",
        ])?;
    }
    cargo(&[
        "llvm-cov",
        "report",
        "--ignore-filename-regex",
        COVERAGE_IGNORE,
    ])
}

/// Every fuzz target, by file name, in order.
fn fuzz_targets() -> Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir("fuzz/fuzz_targets")
        .context("fuzz/fuzz_targets is missing")?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "rs").then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Run one target. With rustup the nightly toolchain is selected with
/// `+nightly`; in the Nix fuzz shell `cargo` already is nightly.
fn fuzz(target: &str, extra: &[String]) -> Result<()> {
    let rustup = Command::new("rustup")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success());
    let mut args: Vec<&str> = Vec::new();
    if rustup {
        args.push("+nightly");
    }
    args.extend(["fuzz", "run", target]);
    if !extra.is_empty() {
        args.push("--");
        args.extend(extra.iter().map(String::as_str));
    }
    cargo(&args)
}

/// The Markdown tools are npm dev dependencies (`package.json`), installed
/// with `npm ci`. They are optional locally so the Rust checks still run
/// without node.
fn docs(require: bool) -> Result<()> {
    // CI always has the tools; a laptop may not.
    let require = require || std::env::var_os("CI").is_some();
    let lint = std::path::Path::new("node_modules/.bin/markdownlint-cli2");
    let mermaid = std::path::Path::new("node_modules/mermaid");
    if !lint.exists() || !mermaid.exists() {
        if require {
            bail!("the docs tools are not installed (run `npm ci`)");
        }
        eprintln!("xtask: skipping docs checks, run `npm ci` to enable them");
        return Ok(());
    }
    exec(Command::new(lint))?;
    let mut files = vec!["README.md".to_owned(), "AGENTS.md".to_owned()];
    collect_markdown(std::path::Path::new("docs"), &mut files)?;
    let mut cmd = Command::new("node");
    cmd.arg("scripts/check-mermaid.mjs").args(&files);
    exec(cmd)
}

/// Every `.md` file under `dir`, skipping `docs/superpowers`.
fn collect_markdown(dir: &std::path::Path, out: &mut Vec<String>) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if path.ends_with("superpowers") {
                continue;
            }
            collect_markdown(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn changelog(check: bool, tag: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("git-cliff");
    cmd.args(["--config", "cliff.toml"]);
    if let Some(tag) = tag {
        cmd.args(["--tag", tag]);
    }
    if check {
        let out = cmd
            .output()
            .context("failed to run git-cliff (cargo install git-cliff)")?;
        if !out.status.success() {
            bail!(
                "git-cliff failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let have = std::fs::read("CHANGELOG.md").unwrap_or_default();
        if out.stdout != have {
            bail!("CHANGELOG.md is out of date (run `cargo xtask changelog`)");
        }
        return Ok(());
    }
    cmd.args(["--output", "CHANGELOG.md"]);
    exec(cmd)
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

/// Fail early with an install hint when a cargo subcommand is missing.
fn require_subcommand(name: &str, package: &str) -> Result<()> {
    let found = Command::new("cargo")
        .args([name, "--version"])
        .output()
        .is_ok_and(|out| out.status.success());
    if found {
        Ok(())
    } else {
        bail!("`cargo {name}` is not installed (cargo install --locked {package})")
    }
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

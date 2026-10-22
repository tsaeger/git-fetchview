// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! `cargo xtask prep-release <version>`: everything up to, but not including,
//! the push.
//!
//! It bumps the version, regenerates the changelog, runs the checks, then makes
//! the release commit and the tag. It never pushes. The maintainer reviews the
//! commit and pushes it, or resets it and starts over.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// The files a release changes.
const FILES: [&str; 3] = ["Cargo.toml", "Cargo.lock", "CHANGELOG.md"];

/// `MAJOR.MINOR.PATCH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    /// Read `1.2.3` or `v1.2.3`.
    pub fn parse(text: &str) -> Result<Self> {
        let bare = text.strip_prefix('v').unwrap_or(text);
        let parts: Vec<&str> = bare.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            bail!("`{text}` is not a version like 1.2.3");
        };
        let number = |part: &str| -> Result<u64> {
            let plain = !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            if !plain {
                bail!("`{text}` is not a version like 1.2.3");
            }
            part.parse()
                .with_context(|| format!("`{text}` is not a version like 1.2.3"))
        };
        Ok(Self {
            major: number(major)?,
            minor: number(minor)?,
            patch: number(patch)?,
        })
    }

    /// The git tag for this version.
    pub fn tag(self) -> String {
        format!("v{self}")
    }
}

/// The version in the `[package]` table of a manifest.
pub fn manifest_version(manifest: &str) -> Result<Version> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start().trim_start_matches('=').trim();
            return Version::parse(value.trim_matches('"'));
        }
    }
    bail!("no version in the [package] table of Cargo.toml")
}

/// The manifest with the `[package]` version replaced. Nothing else changes.
pub fn set_manifest_version(manifest: &str, new: Version) -> Result<String> {
    let mut in_package = false;
    let mut done = false;
    let mut out = String::with_capacity(manifest.len());
    for line in manifest.split_inclusive('\n') {
        let key = line.trim();
        if key.starts_with('[') {
            in_package = key == "[package]";
        }
        if in_package && !done && key.starts_with("version") && key.contains('=') {
            let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
            out.push_str("version = \"");
            out.push_str(&new.to_string());
            out.push('"');
            out.push_str(ending);
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        bail!("no version in the [package] table of Cargo.toml");
    }
    Ok(out)
}

/// What to show once the release commit and tag exist.
pub fn next_steps(tag: &str, branch: &str) -> String {
    format!(
        "\n{tag} is committed and tagged here. Nothing has been pushed.\n\n\
         Review it:\n  git show --stat HEAD\n  git diff HEAD~1 -- CHANGELOG.md\n\n\
         If it looks right, publish it:\n  git push origin {branch} {tag}\n\n\
         If not, undo the commit and the tag, and start over:\n  \
         git tag -d {tag} && git reset --hard HEAD~1\n"
    )
}

/// The commit message: subject, then body.
fn commit_message(version: Version) -> (String, String) {
    (
        format!("chore(release): {}", version.tag()),
        format!("Set the version to {version} and regenerate the changelog for the release."),
    )
}

/// Run `git` and return its trimmed output.
fn git(args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .context("failed to run git")?;
    if !out.status.success() {
        bail!(
            "git {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Prepare release `text`. `publish_dry_run` adds a `cargo publish --dry-run`.
pub fn prep(text: &str, publish_dry_run: bool) -> Result<()> {
    let version = Version::parse(text)?;
    let tag = version.tag();
    let manifest = std::fs::read_to_string("Cargo.toml").context("reading Cargo.toml")?;
    let current = manifest_version(&manifest)?;
    if version <= current {
        bail!("{version} is not newer than the current version {current}");
    }
    if !git(&["status", "--porcelain"])?.is_empty() {
        bail!("the working tree is not clean; commit or stash first");
    }
    if !git(&["tag", "--list", &tag])?.is_empty() {
        bail!("tag {tag} already exists");
    }
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == "HEAD" {
        bail!("HEAD is detached; check out the branch to release from");
    }
    if Command::new("git-cliff")
        .arg("--version")
        .output()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        bail!("git-cliff is not installed (cargo install git-cliff)");
    }

    // The tree was clean, so putting these three files back undoes a failure.
    let result = build(&manifest, version, &tag, publish_dry_run);
    if let Err(e) = result {
        let mut restore = Command::new("git");
        restore.args(["checkout", "--"]).args(FILES);
        let _ = restore.output();
        return Err(
            e.context("release not prepared; Cargo.toml, Cargo.lock and CHANGELOG.md restored")
        );
    }
    println!("{}", next_steps(&tag, &branch));
    Ok(())
}

/// Bump, check, commit and tag.
fn build(manifest: &str, version: Version, tag: &str, publish_dry_run: bool) -> Result<()> {
    std::fs::write("Cargo.toml", set_manifest_version(manifest, version)?)
        .context("writing Cargo.toml")?;
    crate::cargo(&["update", "--workspace", "--offline"])?;
    crate::changelog(false, Some(tag))?;
    crate::run(crate::Task::Ci)?;
    crate::run(crate::Task::Dist)?;
    if publish_dry_run {
        crate::cargo(&[
            "publish",
            "--dry-run",
            "--allow-dirty",
            "--package",
            "git-fetchview",
        ])?;
    }
    let (subject, body) = commit_message(version);
    git(&["add", "--"]
        .iter()
        .chain(&FILES)
        .copied()
        .collect::<Vec<_>>())?;
    git(&["commit", "-s", "-m", &subject, "-m", &body])?;
    git(&["tag", "-a", tag, "-m", tag])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn a_version_reads_with_or_without_a_v() {
        assert_eq!(v("0.1.1"), v("v0.1.1"));
        assert_eq!(v("v10.20.30").to_string(), "10.20.30");
        assert_eq!(v("0.1.1").tag(), "v0.1.1");
    }

    #[test]
    fn a_malformed_version_is_rejected() {
        for bad in [
            "",
            "v",
            "1",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1.2.3-rc1",
            "01.2.3",
            "1..3",
            "-1.2.3",
            "vv1.2.3",
            " 1.2.3",
            "1.2.3 ",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn versions_compare_by_number_not_text() {
        assert!(v("0.1.10") > v("0.1.9"));
        assert!(v("0.2.0") > v("0.1.99"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("0.1.1") <= v("0.1.1"));
    }

    const MANIFEST: &str = "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                            [dependencies]\nclap = { version = \"4.6.7\" }\nversion = \"9\"\n";

    #[test]
    fn the_package_version_is_found() {
        assert_eq!(manifest_version(MANIFEST).unwrap(), v("0.1.0"));
        assert!(manifest_version("[dependencies]\nversion = \"1.0.0\"\n").is_err());
    }

    #[test]
    fn only_the_package_version_changes() {
        let out = set_manifest_version(MANIFEST, v("0.1.1")).unwrap();
        assert_eq!(out, MANIFEST.replacen("0.1.0", "0.1.1", 1));
        assert_eq!(manifest_version(&out).unwrap(), v("0.1.1"));
        assert!(out.contains("clap = { version = \"4.6.7\" }"));
        assert!(out.ends_with("version = \"9\"\n"));
    }

    #[test]
    fn line_endings_and_a_missing_final_newline_survive() {
        let crlf = "[package]\r\nversion = \"0.1.0\"\r\n";
        assert_eq!(
            set_manifest_version(crlf, v("0.2.0")).unwrap(),
            "[package]\r\nversion = \"0.2.0\"\r\n"
        );
        let bare = "[package]\nversion = \"0.1.0\"";
        assert_eq!(
            set_manifest_version(bare, v("0.2.0")).unwrap(),
            "[package]\nversion = \"0.2.0\""
        );
    }

    #[test]
    fn a_manifest_without_a_package_version_is_an_error() {
        assert!(set_manifest_version("[workspace]\nmembers = []\n", v("1.0.0")).is_err());
    }

    #[test]
    fn the_next_steps_push_but_never_run_it() {
        let text = next_steps("v0.1.1", "main");
        assert!(text.contains("git push origin main v0.1.1"));
        assert!(text.contains("git tag -d v0.1.1 && git reset --hard HEAD~1"));
        assert!(text.contains("Nothing has been pushed"));
    }

    #[test]
    fn the_commit_follows_the_project_rules() {
        let (subject, body) = commit_message(v("0.1.1"));
        assert_eq!(subject, "chore(release): v0.1.1");
        assert!(subject.len() <= 72);
        assert!(body.lines().all(|l| l.len() <= 79));
    }
}

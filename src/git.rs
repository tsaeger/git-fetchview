// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Thin, printable wrappers around the git and tig commands.
//!
//! Every external call is a [`Cmd`]. Its `Display` is exactly what gets run,
//! so the cut-and-paste trail can never drift from what the tool does.

use std::fmt;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// A command line that can be both printed and run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd {
    program: &'static str,
    args: Vec<String>,
}

impl Cmd {
    fn new(program: &'static str, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            program,
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// `git remote -v`
    #[must_use]
    pub fn list_remotes() -> Self {
        Self::new("git", ["remote", "-v"])
    }

    /// `git ls-remote <remote>`
    #[must_use]
    pub fn ls_remote(remote: &str) -> Self {
        Self::new("git", ["ls-remote", remote])
    }

    /// `git fetch <remote> <ref>`
    #[must_use]
    pub fn fetch(remote: &str, refname: &str) -> Self {
        Self::new("git", ["fetch", remote, refname])
    }

    /// `git config --get <key>`
    #[must_use]
    pub fn config_get(key: &str) -> Self {
        Self::new("git", ["config", "--get", key])
    }

    /// `git config --type=path --show-origin --get-all <key>`
    #[must_use]
    pub fn config_paths(key: &str) -> Self {
        Self::new(
            "git",
            ["config", "--type=path", "--show-origin", "--get-all", key],
        )
    }

    /// `git config --add <scope_flag> <key> <value>`, e.g. `--local`.
    #[must_use]
    pub fn config_add(scope_flag: &str, key: &str, value: &str) -> Self {
        Self::new("git", ["config", "--add", scope_flag, key, value])
    }

    /// `git rev-parse --path-format=absolute --git-common-dir`
    #[must_use]
    pub fn git_common_dir() -> Self {
        Self::new(
            "git",
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
    }

    /// `git rev-parse --show-toplevel`
    #[must_use]
    pub fn toplevel() -> Self {
        Self::new("git", ["rev-parse", "--show-toplevel"])
    }

    /// `tig FETCH_HEAD`
    #[must_use]
    pub fn tig_fetch_head() -> Self {
        Self::new("tig", ["FETCH_HEAD"])
    }

    /// `git log --merges --format=%s [-n N] <range> --`
    #[must_use]
    pub fn log_merges(range: &str, max: Option<usize>) -> Self {
        let mut args = vec![
            "log".to_owned(),
            "--merges".to_owned(),
            "--format=%s".to_owned(),
        ];
        if let Some(n) = max {
            args.push("-n".to_owned());
            args.push(n.to_string());
        }
        args.push(range.to_owned());
        args.push("--".to_owned());
        Self::new("git", args)
    }

    /// Run and call `f` on each line of stdout as it arrives, so a large
    /// history is not held in memory. Invalid UTF-8 is replaced.
    ///
    /// # Errors
    /// [`Error::Spawn`] or [`Error::Failed`].
    pub fn for_each_line(&self, mut f: impl FnMut(&str)) -> Result<()> {
        let spawn_err = |source| Error::Spawn {
            command: self.to_string(),
            source,
        };
        let mut child = Command::new(self.program)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(spawn_err)?;
        let Some(stdout) = child.stdout.take() else {
            return Err(spawn_err(std::io::Error::other("no stdout pipe")));
        };
        for chunk in BufReader::new(stdout).split(b'\n') {
            let bytes = chunk.map_err(spawn_err)?;
            f(&String::from_utf8_lossy(&bytes));
        }
        let out = child.wait_with_output().map_err(spawn_err)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(Error::Failed {
                command: self.to_string(),
                status: out.status,
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
    }

    /// Run, capture stdout, and fail on a non-zero exit.
    ///
    /// # Errors
    /// [`Error::Spawn`], [`Error::Failed`] or [`Error::NotUtf8`].
    pub fn output(&self) -> Result<String> {
        let out = Command::new(self.program)
            .args(&self.args)
            .stdin(Stdio::null())
            .output()
            .map_err(|source| Error::Spawn {
                command: self.to_string(),
                source,
            })?;
        if !out.status.success() {
            return Err(Error::Failed {
                command: self.to_string(),
                status: out.status,
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        String::from_utf8(out.stdout).map_err(|_| Error::NotUtf8 {
            command: self.to_string(),
        })
    }

    /// Run with inherited stdio (progress output, interactive programs).
    ///
    /// # Errors
    /// [`Error::Spawn`] or [`Error::Failed`].
    pub fn status(&self) -> Result<()> {
        let status = Command::new(self.program)
            .args(&self.args)
            .status()
            .map_err(|source| Error::Spawn {
                command: self.to_string(),
                source,
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Failed {
                command: self.to_string(),
                status,
                stderr: String::new(),
            })
        }
    }
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program)?;
        for arg in &self.args {
            write!(f, " {}", shell_quote(arg))?;
        }
        Ok(())
    }
}

/// Quote `s` for POSIX shells only when it needs it.
#[must_use]
pub fn shell_quote(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,~^".contains(c));
    if plain {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// Drop a password embedded in a `scheme://user:password@host/...` URL.
///
/// The trail is meant to be pasted to other people, so it must not carry
/// credentials. The user name is kept, and git will prompt for the password.
#[must_use]
pub fn redact_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (authority, path) = rest.find('/').map_or((rest, ""), |i| rest.split_at(i));
    let Some((userinfo, host)) = authority.rsplit_once('@') else {
        return url.to_owned();
    };
    let user = userinfo.split_once(':').map_or(userinfo, |(user, _)| user);
    format!("{scheme}://{user}@{host}{path}")
}

/// A configured git remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Remote name, e.g. `origin`.
    pub name: String,
    /// Fetch URL.
    pub url: String,
    /// Where a collected remote came from; empty for real remotes.
    pub tags: Vec<String>,
}

/// A ref advertised by a remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRef {
    /// Full ref name, e.g. `refs/heads/main`.
    pub name: String,
    /// Object id the ref points at.
    pub oid: String,
}

/// Parse `git remote -v`, keeping each remote's fetch URL.
#[must_use]
pub fn parse_remotes(text: &str) -> Vec<Remote> {
    text.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(['\t', ' '])?;
            let url = rest.trim().strip_suffix("(fetch)")?.trim();
            Some(Remote {
                name: name.to_owned(),
                url: url.to_owned(),
                tags: Vec::new(),
            })
        })
        .collect()
}

/// Parse `git ls-remote`, dropping peeled `^{}` entries.
#[must_use]
pub fn parse_ls_remote(text: &str) -> Vec<RemoteRef> {
    text.lines()
        .filter_map(|line| {
            let (oid, name) = line.split_once('\t')?;
            if name.ends_with("^{}") {
                return None;
            }
            Some(RemoteRef {
                name: name.to_owned(),
                oid: oid.to_owned(),
            })
        })
        .collect()
}

/// List the repository's configured remotes. May be empty.
///
/// # Errors
/// Git cannot be run, or this is not a repository.
pub fn configured_remotes() -> Result<Vec<Remote>> {
    Ok(parse_remotes(&Cmd::list_remotes().output()?))
}

/// List the refs a remote advertises. `remote` is a name or a URL.
///
/// # Errors
/// Command failures, or [`Error::NoRefs`].
pub fn remote_refs(remote: &str) -> Result<Vec<RemoteRef>> {
    let refs = parse_ls_remote(&Cmd::ls_remote(remote).output()?);
    if refs.is_empty() {
        Err(Error::NoRefs(remote.to_owned()))
    } else {
        Ok(refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(shell_quote("refs/heads/main"), "refs/heads/main");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn display_matches_what_runs() {
        assert_eq!(
            Cmd::fetch("origin", "refs/pull/1/head").to_string(),
            "git fetch origin refs/pull/1/head"
        );
        assert_eq!(Cmd::tig_fetch_head().to_string(), "tig FETCH_HEAD");
    }

    #[test]
    fn redacts_embedded_passwords_only() {
        assert_eq!(
            redact_url("https://bob:s3cret@host.example/a/b.git"),
            "https://bob@host.example/a/b.git"
        );
        assert_eq!(redact_url("https://host/a/b.git"), "https://host/a/b.git");
        assert_eq!(redact_url("https://tok@host/a"), "https://tok@host/a");
        assert_eq!(
            redact_url("git@github.com:a/b.git"),
            "git@github.com:a/b.git"
        );
        assert_eq!(redact_url("/srv/git/repo"), "/srv/git/repo");
    }

    #[test]
    fn config_commands_display() {
        assert_eq!(
            Cmd::config_add("--local", "fetchview.remotes", "fetchview.remotes").to_string(),
            "git config --add --local fetchview.remotes fetchview.remotes"
        );
        assert_eq!(
            Cmd::config_paths("fetchview.remotes").to_string(),
            "git config --type=path --show-origin --get-all fetchview.remotes"
        );
    }

    #[test]
    fn log_merges_display() {
        assert_eq!(
            Cmd::log_merges("v6.1..v6.2", Some(5)).to_string(),
            "git log --merges --format=%s -n 5 v6.1..v6.2 --"
        );
        assert_eq!(
            Cmd::log_merges("HEAD", None).to_string(),
            "git log --merges --format=%s HEAD --"
        );
    }

    #[test]
    fn parses_remote_v() {
        let text = "origin\tgit@host:a/b.git (fetch)\norigin\tgit@host:a/b.git (push)\nup\thttps://x/y (fetch)\n";
        let got = parse_remotes(text);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "origin");
        assert_eq!(got[1].url, "https://x/y");
    }

    #[test]
    fn parses_ls_remote_and_skips_peeled() {
        let text = "aaa\tHEAD\nbbb\trefs/tags/v1\nccc\trefs/tags/v1^{}\n";
        let got = parse_ls_remote(text);
        assert_eq!(
            got.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["HEAD", "refs/tags/v1"]
        );
    }
}

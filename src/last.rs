// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The remote and ref you picked last, so the picker can start there.
//!
//! It is kept per repository, in `<git-common-dir>/fetchview/last`, two lines:
//! the remote's URL, then the ref's name. Nothing here is important, so every
//! failure to read it means "no memory" and a failure to write it is ignored by
//! the caller. `fetchview.remember = false` turns it off.

use std::fs;
use std::path::PathBuf;

use crate::error::Result;
use crate::git::Cmd;

/// Config key: whether to remember. Defaults to true.
pub const KEY: &str = "fetchview.remember";

/// What was picked last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Last {
    /// The remote's URL, as it was fetched from.
    pub url: String,
    /// The ref that was picked.
    pub refname: String,
}

/// Read the two lines. Anything else, such as a file from another version,
/// is no memory.
#[must_use]
pub fn parse(text: &str) -> Option<Last> {
    let mut lines = text.lines();
    let url = lines.next()?.trim();
    let refname = lines.next()?.trim();
    (!url.is_empty() && !refname.is_empty()).then(|| Last {
        url: url.to_owned(),
        refname: refname.to_owned(),
    })
}

/// The file's text for `last`.
#[must_use]
pub fn render(last: &Last) -> String {
    format!("{}\n{}\n", last.url, last.refname)
}

fn path() -> Result<PathBuf> {
    let dir = Cmd::git_common_dir().output()?;
    Ok(PathBuf::from(dir.trim()).join("fetchview").join("last"))
}

/// The remembered pick, if there is one and it can be read.
#[must_use]
pub fn load() -> Option<Last> {
    parse(&fs::read_to_string(path().ok()?).ok()?)
}

/// Remember a pick.
///
/// # Errors
/// The git directory cannot be found or the file cannot be written.
pub fn save(last: &Last) -> Result<()> {
    let path = path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|source| crate::error::Error::Io {
            path: dir.to_owned(),
            source,
        })?;
    }
    fs::write(&path, render(last)).map_err(|source| crate::error::Error::Io { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_lines_round_trip() {
        let last = Last {
            url: "https://h/p/x".into(),
            refname: "refs/tags/v1".into(),
        };
        assert_eq!(parse(&render(&last)), Some(last));
    }

    #[test]
    fn anything_else_is_no_memory() {
        for text in [
            "",
            "\n",
            "only-one-line\n",
            "\nref\n",
            "url\n\n",
            "  \n  \n",
        ] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn extra_lines_and_spaces_are_ignored() {
        assert_eq!(
            parse(" https://h/x \n refs/heads/main \nmore\n"),
            Some(Last {
                url: "https://h/x".into(),
                refname: "refs/heads/main".into()
            })
        );
    }
}

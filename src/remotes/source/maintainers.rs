// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Pull `T:` git URLs from a MAINTAINERS file.

use std::fs;
use std::path::PathBuf;

use super::{Scan, Source, collect};
use crate::error::{Error, Result};

/// The URL of every `T: git <url> [branch]` line.
#[must_use]
pub fn parse_maintainers(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("T:")?;
            let mut words = rest.split_whitespace();
            (words.next()? == "git").then_some(())?;
            words.next()
        })
        .collect()
}

/// Pull URLs from a MAINTAINERS file.
#[derive(Debug, Clone)]
pub struct Maintainers {
    /// Path to the file.
    pub path: PathBuf,
}

impl Source for Maintainers {
    fn scan(&self) -> Result<Scan> {
        let text = fs::read_to_string(&self.path).map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(collect(parse_maintainers(&text), "maintainers"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
NETWORKING [GENERAL]
M:\tSomeone <someone@example.org>
L:\tnetdev@vger.kernel.org
T:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git
T:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git main
F:\tnet/

SOMETHING OLD
T:\tquilt https://example.org/patches/
T:\thg https://example.org/hg/repo
T:\tgit
T:\tgit https://example.org/x.git
";

    #[test]
    fn takes_git_t_lines_and_ignores_the_branch() {
        assert_eq!(
            parse_maintainers(SAMPLE),
            [
                "git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git",
                "git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git",
                "https://example.org/x.git",
            ]
        );
    }
}

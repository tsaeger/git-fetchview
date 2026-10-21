// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Pull URLs from merge commit subjects.

use super::{Scan, Source, collect};
use crate::error::Result;
use crate::git::Cmd;

/// True for `scheme://...` and for scp-style `host:path`.
fn url_shaped(token: &str) -> bool {
    token.contains("://")
        || token
            .split_once(':')
            .is_some_and(|(head, rest)| !head.is_empty() && !head.contains('/') && !rest.is_empty())
}

/// The URL in a merge subject, if it has one.
///
/// Takes the text after the last ` of `, or the first word when there is no
/// ` of `, and keeps it only if it looks like a URL. Validation happens later.
#[must_use]
pub fn parse_merge_subject(subject: &str) -> Option<&str> {
    let rest = subject.strip_prefix("Merge ")?;
    let candidate = rest.rfind(" of ").map_or(rest, |i| &rest[i + 4..]);
    let token = candidate.split_whitespace().next()?;
    url_shaped(token).then_some(token)
}

/// Pull URLs from merge commits in a revision range.
#[derive(Debug, Clone)]
pub struct Merges {
    /// Revision range, e.g. `HEAD` or `v6.1..v6.2`.
    pub range: String,
    /// Stop after this many merges.
    pub max: Option<usize>,
}

impl Source for Merges {
    fn scan(&self) -> Result<Scan> {
        let mut urls: Vec<String> = Vec::new();
        Cmd::log_merges(&self.range, self.max).for_each_line(|subject| {
            if let Some(url) = parse_merge_subject(subject) {
                urls.push(url.to_owned());
            }
        })?;
        Ok(collect(urls.iter().map(String::as_str), "merge"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_url_after_of() {
        assert_eq!(
            parse_merge_subject(
                "Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"
            ),
            Some("git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net")
        );
        assert_eq!(
            parse_merge_subject(
                "Merge branch 'for-linus' of https://git.kernel.org/pub/scm/x/foo.git"
            ),
            Some("https://git.kernel.org/pub/scm/x/foo.git")
        );
    }

    #[test]
    fn handles_old_style_and_into_suffix() {
        assert_eq!(
            parse_merge_subject("Merge git://git.kernel.org/pub/scm/x/foo.git"),
            Some("git://git.kernel.org/pub/scm/x/foo.git")
        );
        assert_eq!(
            parse_merge_subject("Merge branch 'x' of git://h/p/y into master"),
            Some("git://h/p/y")
        );
    }

    #[test]
    fn accepts_scp_style() {
        assert_eq!(
            parse_merge_subject("Merge branch 'x' of git@github.com:a/b.git"),
            Some("git@github.com:a/b.git")
        );
    }

    #[test]
    fn skips_subjects_without_a_url() {
        for subject in [
            "Merge remote-tracking branch 'a/b'",
            "Merge branch 'master' of ../local/path",
            "Merge branch 'x' of the state of the art",
            "Revert \"Merge tag 'x' of git://h/p/y\"",
            "Merge",
            "",
        ] {
            assert_eq!(parse_merge_subject(subject), None, "{subject:?}");
        }
    }
}

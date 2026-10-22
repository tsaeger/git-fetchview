// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Places that remote URLs can be collected from.

pub mod maintainers;
pub mod manual;
pub mod merges;

use std::collections::HashMap;

use super::entry::{Entry, validate_url};
use crate::error::Result;

/// What a source found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Valid, deduplicated entries, tagged with the source name.
    pub entries: Vec<Entry>,
    /// URLs that failed validation.
    pub rejected: usize,
}

/// Something that can produce candidate entries.
pub trait Source {
    /// Look for remotes.
    ///
    /// # Errors
    /// The source could not be read.
    fn scan(&self) -> Result<Scan>;
}

/// Validate raw URLs, tag them, and collapse duplicates.
#[must_use]
pub fn collect<'a>(urls: impl IntoIterator<Item = &'a str>, tag: &str) -> Scan {
    let mut rejected = 0;
    let mut found = Vec::new();
    for raw in urls {
        match validate_url(raw) {
            Ok(url) => found.push(Entry::new(url, tag)),
            Err(_) => rejected += 1,
        }
    }
    Scan {
        entries: dedupe(found),
        rejected,
    }
}

fn scheme_rank(url: &str) -> u8 {
    match url.split_once("://") {
        Some((scheme, _)) => match scheme.to_ascii_lowercase().as_str() {
            "https" => 0,
            "git" => 1,
            "http" => 2,
            _ => 3,
        },
        None => 3,
    }
}

/// Collapse entries with the same key. Tags are unioned and the best scheme
/// is kept: `https`, then `git`, then `http`.
#[must_use]
pub fn dedupe(entries: Vec<Entry>) -> Vec<Entry> {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Entry> = Vec::new();
    for entry in entries {
        let k = entry.key();
        if let Some(&i) = index.get(&k) {
            let keep = &mut out[i];
            keep.tags.extend(entry.tags.iter().cloned());
            if scheme_rank(&entry.url) < scheme_rank(&keep.url) {
                keep.url = entry.url;
            }
        } else {
            index.insert(k, out.len());
            out.push(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_validates_counts_rejects_and_tags() {
        let scan = collect(
            ["https://h/p/a", "ext::sh -c id", "-oBad", "git://h/p/b"],
            "manual",
        );
        assert_eq!(scan.entries.len(), 2);
        assert_eq!(scan.rejected, 2);
        assert!(scan.entries.iter().all(|e| e.tags.contains("manual")));
    }

    #[test]
    fn dedupe_prefers_https_then_git_then_http_and_unions_tags() {
        let a = Entry::new("http://h/p/x", "merge");
        let b = Entry::new("git://h/p/x", "merge");
        let c = Entry::new("https://h/p/x.git", "maintainers");
        let out = dedupe(vec![a, b, c]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].url, "https://h/p/x.git");
        assert_eq!(out[0].tags.len(), 2);
    }

    #[test]
    fn both_spellings_of_one_kernel_tree_collapse_to_the_clean_url() {
        let scan = collect(
            [
                "git://git.kernel.org:/pub/scm/fs/xfs/xfs-linux",
                "git://git.kernel.org/pub/scm/fs/xfs/xfs-linux",
            ],
            "merge",
        );
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(
            scan.entries[0].url,
            "git://git.kernel.org/pub/scm/fs/xfs/xfs-linux"
        );
    }

    #[test]
    fn dedupe_folds_ssh_into_the_https_url_whichever_comes_first() {
        for order in [
            ["https://h/p/x", "git@h:p/x"],
            ["git@h:p/x", "https://h/p/x"],
        ] {
            let out = dedupe(order.iter().map(|u| Entry::new(*u, "t")).collect());
            assert_eq!(out.len(), 1, "{order:?}");
            assert_eq!(out[0].url, "https://h/p/x");
        }
    }

    #[test]
    fn dedupe_keeps_a_lone_ssh_url_as_written() {
        let out = dedupe(vec![Entry::new("git@h:p/x", "t")]);
        assert_eq!(out[0].url, "git@h:p/x");
    }
}

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Remotes collected outside `.git/config`, kept in files.

pub mod entry;
pub mod file;
pub mod plan;
pub mod registry;
pub mod source;

use std::collections::HashSet;

use crate::git::Remote;
use entry::{Entry, key};

/// The remotes the picker lists: real ones first, then collected entries
/// sorted by label. An entry for a repo that is already a real remote is
/// hidden.
#[must_use]
pub fn picker_remotes(real: Vec<Remote>, entries: &[Entry]) -> Vec<Remote> {
    let real_keys: HashSet<String> = real.iter().map(|r| key(&r.url)).collect();
    let mut extra: Vec<Remote> = entries
        .iter()
        .filter(|e| !real_keys.contains(&e.key()))
        .map(|e| Remote {
            name: e.display_label(),
            url: e.url.clone(),
            tags: e.tags.iter().cloned().collect(),
        })
        .collect();
    extra.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.url.cmp(&b.url)));
    real.into_iter().chain(extra).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(name: &str, url: &str) -> Remote {
        Remote {
            name: name.into(),
            url: url.into(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn real_remotes_come_first_then_entries_sorted_by_label() {
        let entries = [
            Entry::new("https://h/p/zeta", "merge"),
            Entry::new("https://h/p/alpha", "maintainers"),
        ];
        let got = picker_remotes(vec![real("origin", "https://example.org/o")], &entries);
        let names: Vec<_> = got.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["origin", "p/alpha", "p/zeta"]);
        assert_eq!(got[1].tags, ["maintainers"]);
    }

    #[test]
    fn a_real_remote_hides_the_same_repo_from_the_entries() {
        let entries = [Entry::new("git://git.kernel.org/pub/scm/x/net", "merge")];
        let got = picker_remotes(
            vec![real("upstream", "https://git.kernel.org/pub/scm/x/net.git")],
            &entries,
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "upstream");
    }

    #[test]
    fn an_explicit_label_is_used() {
        let mut e = Entry::new("https://h/p/x", "manual");
        e.label = Some("mine".into());
        let got = picker_remotes(Vec::new(), &[e]);
        assert_eq!(got[0].name, "mine");
    }
}

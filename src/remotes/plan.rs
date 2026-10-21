// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Decide what `add` should do with the candidates a source found.

use std::collections::HashSet;

use super::entry::Entry;

/// What to do with each candidate.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// In no registered file: offered for adding.
    pub new: Vec<Entry>,
    /// Already in the target file: only their tags are merged in.
    pub tag_updates: Vec<Entry>,
    /// In another registered file: shown as present, left alone.
    pub present: Vec<Entry>,
}

/// Sort `candidates` by where their repo already is.
///
/// `target` is the entries of the file being written, `elsewhere` the
/// entries of every other registered file.
#[must_use]
pub fn plan(candidates: Vec<Entry>, target: &[Entry], elsewhere: &[Entry]) -> Plan {
    let target_keys: HashSet<String> = target.iter().map(Entry::key).collect();
    let other_keys: HashSet<String> = elsewhere.iter().map(Entry::key).collect();
    let mut plan = Plan::default();
    for candidate in candidates {
        let k = candidate.key();
        if target_keys.contains(&k) {
            plan.tag_updates.push(candidate);
        } else if other_keys.contains(&k) {
            plan.present.push(candidate);
        } else {
            plan.new.push(candidate);
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(url: &str) -> Entry {
        Entry::new(url, "t")
    }

    #[test]
    fn splits_new_tag_updates_and_present() {
        let candidates = vec![
            e("https://h/p/new"),
            e("https://h/p/here"),
            e("https://h/p/there"),
        ];
        let target = [e("git://h/p/here")];
        let elsewhere = [e("https://h/p/there")];
        let plan = plan(candidates, &target, &elsewhere);
        assert_eq!(plan.new.len(), 1);
        assert_eq!(plan.new[0].url, "https://h/p/new");
        assert_eq!(plan.tag_updates.len(), 1);
        assert_eq!(plan.tag_updates[0].url, "https://h/p/here");
        assert_eq!(plan.present.len(), 1);
        assert_eq!(plan.present[0].url, "https://h/p/there");
    }
}

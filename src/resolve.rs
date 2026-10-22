// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Turning what you typed for a ref into one the remote advertises.
//!
//! `git fv <url> <ref>` skips the picker, so the ref is matched against the
//! remote's own list. The answer is always a name from that list, never the
//! text as typed, so nothing typed can reach git as an option.

use crate::git::RemoteRef;

/// Why a ref could not be settled on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// The remote advertises nothing that matches.
    NoMatch,
    /// More than one ref matches; these are they.
    Ambiguous(Vec<String>),
}

/// The ref a request names.
///
/// In order: a full name the remote advertises; a pull request number, bare
/// or with a `#` (`refs/pull/N/head` on GitHub and Gitea,
/// `refs/merge-requests/N/head` on GitLab); a short name, which is a branch or
/// a tag of that name, or anything under `refs/` that ends the way it was
/// typed (`pull/7/head`).
///
/// # Errors
/// [`Unresolved`] when nothing, or more than one thing, matches.
pub fn resolve_ref(refs: &[RemoteRef], query: &str) -> Result<String, Unresolved> {
    let query = query.trim();
    let has = |name: &str| refs.iter().any(|r| r.name == name);
    if has(query) {
        return Ok(query.to_owned());
    }
    let number = query.strip_prefix('#').unwrap_or(query);
    let candidates: Vec<String> =
        if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) {
            [
                format!("refs/pull/{number}/head"),
                format!("refs/merge-requests/{number}/head"),
            ]
            .into_iter()
            .filter(|name| has(name))
            .collect()
        } else if query.is_empty() {
            Vec::new()
        } else {
            [
                format!("refs/heads/{query}"),
                format!("refs/tags/{query}"),
                format!("refs/{query}"),
            ]
            .into_iter()
            .filter(|name| has(name))
            .collect()
        };
    match candidates.len() {
        0 => Err(Unresolved::NoMatch),
        1 => Ok(candidates.into_iter().next().unwrap_or_default()),
        _ => Err(Unresolved::Ambiguous(candidates)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(names: &[&str]) -> Vec<RemoteRef> {
        names
            .iter()
            .map(|n| RemoteRef {
                name: (*n).to_owned(),
                oid: "0123456789".to_owned(),
            })
            .collect()
    }

    fn all() -> Vec<RemoteRef> {
        refs(&[
            "HEAD",
            "refs/heads/main",
            "refs/heads/dup",
            "refs/tags/v1.0",
            "refs/tags/dup",
            "refs/pull/7/head",
            "refs/pull/7/merge",
            "refs/merge-requests/9/head",
        ])
    }

    #[test]
    fn a_full_name_the_remote_advertises_is_taken_as_it_is() {
        assert_eq!(
            resolve_ref(&all(), "refs/heads/main").unwrap(),
            "refs/heads/main"
        );
        assert_eq!(resolve_ref(&all(), "HEAD").unwrap(), "HEAD");
        assert_eq!(
            resolve_ref(&all(), "refs/pull/7/merge").unwrap(),
            "refs/pull/7/merge"
        );
    }

    #[test]
    fn a_number_or_hash_number_is_a_pull_or_merge_request() {
        assert_eq!(resolve_ref(&all(), "7").unwrap(), "refs/pull/7/head");
        assert_eq!(resolve_ref(&all(), "#7").unwrap(), "refs/pull/7/head");
        assert_eq!(
            resolve_ref(&all(), "#9").unwrap(),
            "refs/merge-requests/9/head"
        );
        assert_eq!(resolve_ref(&all(), "8"), Err(Unresolved::NoMatch));
    }

    #[test]
    fn a_number_on_both_styles_is_ambiguous() {
        let both = refs(&["refs/pull/3/head", "refs/merge-requests/3/head"]);
        assert_eq!(
            resolve_ref(&both, "3"),
            Err(Unresolved::Ambiguous(vec![
                "refs/pull/3/head".into(),
                "refs/merge-requests/3/head".into()
            ]))
        );
    }

    #[test]
    fn a_short_name_is_a_branch_or_a_tag_or_a_path_under_refs() {
        assert_eq!(resolve_ref(&all(), "main").unwrap(), "refs/heads/main");
        assert_eq!(resolve_ref(&all(), "v1.0").unwrap(), "refs/tags/v1.0");
        assert_eq!(
            resolve_ref(&all(), "pull/7/head").unwrap(),
            "refs/pull/7/head"
        );
        assert_eq!(resolve_ref(&all(), "nope"), Err(Unresolved::NoMatch));
    }

    #[test]
    fn a_branch_and_a_tag_of_one_name_are_ambiguous_and_listed() {
        assert_eq!(
            resolve_ref(&all(), "dup"),
            Err(Unresolved::Ambiguous(vec![
                "refs/heads/dup".into(),
                "refs/tags/dup".into()
            ]))
        );
    }

    #[test]
    fn nothing_typed_can_come_back_as_an_option() {
        for query in ["", " ", "--upload-pack=x", "-f", "#", "#x", "ma*"] {
            assert_eq!(
                resolve_ref(&all(), query),
                Err(Unresolved::NoMatch),
                "{query:?}"
            );
        }
    }
}

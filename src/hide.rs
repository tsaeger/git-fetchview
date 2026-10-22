// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Hiding refs that other trees already carry.
//!
//! In a kernel tree most tags are the same ones mainline and stable have, and
//! you would fetch those from mainline or stable. `fetchview.hideRefsFrom`
//! lists repositories whose refs are left out of the refs picker:
//!
//! ```text
//! git config --add fetchview.hideRefsFrom "<url> refs/tags/* refs/heads/master"
//! ```
//!
//! A ref is hidden when a listed repository has the same name at the same
//! commit. Those lists are cached under the git directory and refetched when
//! they are a day old, so opening a refs list does not wait on the network.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::Result;
use crate::git::{self, Cancel, Cmd, RemoteRef};
use crate::remotes::entry::validate_url;

/// Config key: repositories whose refs are hidden. Multi-valued.
pub const KEY: &str = "fetchview.hideRefsFrom";

/// Used when [`KEY`] is not set anywhere: mainline and stable tags.
pub const DEFAULT: [&str; 2] = [
    "git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git refs/tags/*",
    "git://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git refs/tags/*",
];

/// Config key: ref patterns that are always hidden. Multi-valued.
pub const PATTERNS_KEY: &str = "fetchview.hideRefs";

/// Used when [`PATTERNS_KEY`] is not set anywhere: the merge refs hosts make
/// for every pull request, and the namespaces GitLab keeps for itself.
pub const DEFAULT_PATTERNS: [&str; 5] = [
    "refs/pull/*/merge",
    "refs/merge-requests/*/merge",
    "refs/pipelines/*",
    "refs/environments/*",
    "refs/keep-around/*",
];

/// How long a cached list is used before it is fetched again.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// One `fetchview.hideRefsFrom` value: a repository and the ref patterns to
/// take from it. No patterns means every ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Repository URL.
    pub url: String,
    /// Patterns in the style `git ls-remote` takes, e.g. `refs/tags/*`.
    pub patterns: Vec<String>,
}

/// Parse one value: the URL, then any number of patterns.
///
/// # Errors
/// The reason the value is not usable.
pub fn parse_rule(value: &str) -> std::result::Result<Rule, String> {
    let mut words = value.split_whitespace();
    let url = validate_url(words.next().unwrap_or_default()).map_err(|e| e.to_string())?;
    let patterns: Vec<String> = words.map(str::to_owned).collect();
    if let Some(bad) = patterns.iter().find(|p| p.starts_with('-')) {
        return Err(format!("pattern `{bad}` starts with '-'"));
    }
    Ok(Rule { url, patterns })
}

/// Parse the values of [`KEY`]. `None` (unset) gives the defaults. Blank
/// values are ignored, so a single empty value turns hiding off.
///
/// # Errors
/// The first bad value and why it was rejected.
pub fn parse_rules(values: Option<&str>) -> std::result::Result<Vec<Rule>, (String, String)> {
    let Some(text) = values else {
        return Ok(DEFAULT.iter().filter_map(|v| parse_rule(v).ok()).collect());
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_rule(line).map_err(|reason| (line.trim().to_owned(), reason)))
        .collect()
}

/// Parse the values of [`PATTERNS_KEY`]: patterns split on whitespace. `None`
/// (unset) gives the defaults, and blank values are ignored, so a single empty
/// value turns this hiding off.
#[must_use]
pub fn parse_patterns(values: Option<&str>) -> Vec<String> {
    match values {
        None => DEFAULT_PATTERNS.iter().map(|p| (*p).to_owned()).collect(),
        Some(text) => text.split_whitespace().map(str::to_owned).collect(),
    }
}

/// Whether `text` matches `pattern`, where `*` stands for any run of
/// characters, `/` included.
#[must_use]
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some((pi, ti));
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == b'*')
}

/// Split `refs` into the ones to show and the ones matching a pattern, in
/// their original order.
#[must_use]
pub fn hide_matching(
    patterns: &[String],
    refs: Vec<RemoteRef>,
) -> (Vec<RemoteRef>, Vec<RemoteRef>) {
    refs.into_iter()
        .partition(|r| !patterns.iter().any(|p| glob_match(p, &r.name)))
}

/// Cache file name for a rule; the same rule always gives the same name.
#[must_use]
pub fn cache_name(rule: &Rule) -> String {
    // FNV-1a: stable across Rust versions, unlike `DefaultHasher`.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let words = std::iter::once(&rule.url).chain(&rule.patterns);
    for word in words {
        for byte in word.bytes().chain(std::iter::once(0)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

fn same_url(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

/// Refs to leave out, with the problems met while loading them.
#[derive(Debug, Default)]
pub struct Hidden {
    entries: HashSet<(String, String)>,
    urls: Vec<String>,
    /// A message for the status line when a list could not be refreshed.
    pub note: Option<String>,
}

impl Hidden {
    /// Load every rule's list from `dir`, calling `fetch` for those that are
    /// missing, older than [`MAX_AGE`], or when `force` is set. A failed
    /// fetch falls back to the old copy if there is one.
    pub fn load(
        rules: &[Rule],
        dir: &Path,
        now: SystemTime,
        force: bool,
        fetch: &mut dyn FnMut(&Rule) -> Result<String>,
    ) -> Self {
        let mut hidden = Self::default();
        let mut problems = Vec::new();
        for rule in rules {
            hidden.urls.push(rule.url.clone());
            let path = dir.join(cache_name(rule));
            let cached = fs::read_to_string(&path).ok();
            let age = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                // A file from the future (clock skew) counts as new.
                .map(|t| now.duration_since(t).unwrap_or_default());
            let fresh = !force && cached.is_some() && age.is_some_and(|a| a < MAX_AGE);
            let text = if fresh {
                cached
            } else {
                match fetch(rule) {
                    Ok(text) => {
                        // A cache that cannot be written only costs a refetch.
                        let _ = fs::create_dir_all(dir).and_then(|()| fs::write(&path, &text));
                        Some(text)
                    }
                    Err(_) if cached.is_some() => {
                        problems.push(format!(
                            "could not refresh {}, using the cached list",
                            rule.url
                        ));
                        cached
                    }
                    Err(_) => {
                        problems.push(format!("could not load hidden refs from {}", rule.url));
                        None
                    }
                }
            };
            for r in git::parse_ls_remote(&text.unwrap_or_default()) {
                hidden.entries.insert((r.name, r.oid));
            }
        }
        hidden.note = match problems.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            [first, rest @ ..] => Some(format!("{first} (+{} more)", rest.len())),
        };
        hidden
    }

    /// Where the cache lives: inside the git common dir, so linked worktrees
    /// share it.
    ///
    /// # Errors
    /// Not in a git repository, or git cannot be run.
    pub fn cache_dir() -> Result<PathBuf> {
        let dir = Cmd::git_common_dir().output()?;
        Ok(PathBuf::from(dir.trim()).join("fetchview").join("hidden"))
    }

    /// [`Hidden::load`] against the real cache and `git ls-remote`. When
    /// `cancel` is set the fetches stop, and the result should be thrown away.
    #[must_use]
    pub fn load_real(rules: &[Rule], force: bool, cancel: &Cancel) -> Self {
        let fetch = &mut |rule: &Rule| {
            Cmd::ls_remote_patterns(&rule.url, &rule.patterns).output_until(cancel)
        };
        match Self::cache_dir() {
            Ok(dir) => Self::load(rules, &dir, SystemTime::now(), force, fetch),
            Err(e) => Self {
                note: Some(e.to_string()),
                ..Self::default()
            },
        }
    }

    /// Split `refs` into the ones to show and the ones other trees carry.
    /// Asking for one of the listed repositories itself hides nothing.
    #[must_use]
    pub fn apply(
        &self,
        remote_url: &str,
        refs: Vec<RemoteRef>,
    ) -> (Vec<RemoteRef>, Vec<RemoteRef>) {
        if self.urls.iter().any(|u| same_url(u, remote_url)) {
            return (refs, Vec::new());
        }
        refs.into_iter()
            .partition(|r| !self.entries.contains(&(r.name.clone(), r.oid.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const URL: &str = "git://example.org/a.git";

    fn rule(patterns: &[&str]) -> Rule {
        Rule {
            url: URL.into(),
            patterns: patterns.iter().map(|p| (*p).to_owned()).collect(),
        }
    }

    fn r(name: &str, oid: &str) -> RemoteRef {
        RemoteRef {
            name: name.into(),
            oid: oid.into(),
        }
    }

    #[test]
    fn a_star_matches_any_run_including_slashes() {
        assert!(glob_match("refs/pull/*/merge", "refs/pull/12/merge"));
        assert!(glob_match("refs/pull/*/merge", "refs/pull//merge"));
        assert!(!glob_match("refs/pull/*/merge", "refs/pull/12/head"));
        assert!(!glob_match("refs/pull/*/merge", "refs/pull/merge"));
        assert!(glob_match("refs/pipelines/*", "refs/pipelines/9"));
        assert!(!glob_match("refs/pipelines/*", "refs/pipelinesx"));
        assert!(glob_match("refs/heads/main", "refs/heads/main"));
        assert!(!glob_match("refs/heads/main", "refs/heads/main2"));
        assert!(glob_match("*", "anything/at/all"));
        assert!(glob_match("a*b*c", "aXXbYYc"));
        assert!(!glob_match("a*b*c", "aXXbYY"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "x"));
    }

    #[test]
    fn unset_patterns_mean_the_host_noise_and_blank_means_off() {
        let defaults = parse_patterns(None);
        assert!(defaults.contains(&"refs/pull/*/merge".to_owned()));
        assert!(defaults.contains(&"refs/merge-requests/*/merge".to_owned()));
        assert!(defaults.contains(&"refs/pipelines/*".to_owned()));
        assert_eq!(parse_patterns(Some("")), Vec::<String>::new());
        assert_eq!(parse_patterns(Some("\n  \n")), Vec::<String>::new());
        assert_eq!(
            parse_patterns(Some("refs/x/*\nrefs/y/* refs/z")),
            ["refs/x/*", "refs/y/*", "refs/z"]
        );
    }

    #[test]
    fn matching_refs_are_split_out_in_their_original_order() {
        let patterns = parse_patterns(None);
        let (shown, hidden) = hide_matching(
            &patterns,
            vec![
                r("HEAD", "1"),
                r("refs/heads/main", "1"),
                r("refs/merge-requests/3/head", "2"),
                r("refs/merge-requests/3/merge", "3"),
                r("refs/pipelines/77", "4"),
                r("refs/pull/9/merge", "5"),
                r("refs/pull/9/head", "6"),
            ],
        );
        let names = |v: &[RemoteRef]| v.iter().map(|x| x.name.clone()).collect::<Vec<_>>();
        assert_eq!(
            names(&shown),
            [
                "HEAD",
                "refs/heads/main",
                "refs/merge-requests/3/head",
                "refs/pull/9/head"
            ]
        );
        assert_eq!(
            names(&hidden),
            [
                "refs/merge-requests/3/merge",
                "refs/pipelines/77",
                "refs/pull/9/merge"
            ]
        );
    }

    #[test]
    fn a_rule_is_a_url_then_any_number_of_patterns() {
        assert_eq!(
            parse_rule(&format!("{URL} refs/tags/* refs/heads/master")),
            Ok(rule(&["refs/tags/*", "refs/heads/master"]))
        );
        assert_eq!(parse_rule(URL), Ok(rule(&[])));
    }

    #[test]
    fn a_rule_rejects_what_git_could_read_as_an_option() {
        assert!(parse_rule("").is_err());
        assert!(parse_rule("--upload-pack=x refs/tags/*").is_err());
        assert!(parse_rule(&format!("{URL} --heads")).is_err());
    }

    #[test]
    fn unset_means_the_kernel_defaults_and_blank_means_off() {
        let defaults = parse_rules(None).unwrap();
        assert_eq!(defaults.len(), 2);
        assert!(defaults[0].url.contains("torvalds"));
        assert!(defaults[1].url.contains("stable"));
        assert_eq!(parse_rules(Some("\n")).unwrap(), vec![]);
        assert_eq!(
            parse_rules(Some(&format!("{URL} refs/tags/*\n\n"))).unwrap(),
            vec![rule(&["refs/tags/*"])]
        );
    }

    #[test]
    fn a_bad_value_is_reported_with_its_text() {
        let (value, reason) = parse_rules(Some("ext::sh -c x")).unwrap_err();
        assert!(value.starts_with("ext::"));
        assert_ne!(reason, "");
    }

    #[test]
    fn cache_names_differ_by_url_and_patterns() {
        let a = rule(&["refs/tags/*"]);
        assert_eq!(cache_name(&a), cache_name(&a.clone()));
        assert_ne!(cache_name(&a), cache_name(&rule(&["refs/heads/*"])));
        assert_ne!(
            cache_name(&a),
            cache_name(&Rule {
                url: "git://example.org/b.git".into(),
                ..a
            })
        );
    }

    #[test]
    fn hides_only_refs_with_the_same_name_and_commit() {
        let dir = tempfile::tempdir().unwrap();
        let mut fetch = |_: &Rule| Ok("1111\trefs/tags/v1\n2222\trefs/tags/v2\n".to_owned());
        let hidden = Hidden::load(
            &[rule(&["refs/tags/*"])],
            dir.path(),
            SystemTime::now(),
            false,
            &mut fetch,
        );
        let (kept, n) = hidden.apply(
            "git://example.org/fork.git",
            vec![
                r("refs/tags/v1", "1111"),
                r("refs/tags/v2", "9999"),
                r("refs/heads/dev", "3333"),
            ],
        );
        assert_eq!(n.len(), 1);
        assert_eq!(
            kept.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            ["refs/tags/v2", "refs/heads/dev"]
        );
    }

    #[test]
    fn a_listed_repository_still_shows_everything() {
        let dir = tempfile::tempdir().unwrap();
        let mut fetch = |_: &Rule| Ok("1111\trefs/tags/v1\n".to_owned());
        let hidden = Hidden::load(
            &[rule(&[])],
            dir.path(),
            SystemTime::now(),
            false,
            &mut fetch,
        );
        let (kept, n) = hidden.apply(URL, vec![r("refs/tags/v1", "1111")]);
        assert_eq!((kept.len(), n.len()), (1, 0));
    }

    #[test]
    fn a_fresh_cache_is_not_fetched_again_but_force_and_age_refetch() {
        let dir = tempfile::tempdir().unwrap();
        let rules = [rule(&["refs/tags/*"])];
        let calls = std::cell::Cell::new(0);
        let mut fetch = |_: &Rule| {
            calls.set(calls.get() + 1);
            Ok("1111\trefs/tags/v1\n".to_owned())
        };
        let now = SystemTime::now();
        Hidden::load(&rules, dir.path(), now, false, &mut fetch);
        Hidden::load(&rules, dir.path(), now, false, &mut fetch);
        assert_eq!(calls.get(), 1, "second load used the cache");
        Hidden::load(&rules, dir.path(), now, true, &mut fetch);
        assert_eq!(calls.get(), 2, "force refetches");
        Hidden::load(
            &rules,
            dir.path(),
            now + MAX_AGE + Duration::from_secs(60),
            false,
            &mut fetch,
        );
        assert_eq!(calls.get(), 3, "an old cache refetches");
    }

    #[test]
    fn a_failed_refresh_uses_the_old_copy_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let rules = [rule(&["refs/tags/*"])];
        let now = SystemTime::now();
        Hidden::load(&rules, dir.path(), now, false, &mut |_| {
            Ok("1111\trefs/tags/v1\n".to_owned())
        });
        let mut fail = |_: &Rule| Err(crate::Error::NoRefs("x".into()));
        let hidden = Hidden::load(&rules, dir.path(), now, true, &mut fail);
        let (_, n) = hidden.apply("git://other", vec![r("refs/tags/v1", "1111")]);
        assert_eq!(n.len(), 1);
        assert!(hidden.note.unwrap().contains(URL));
    }

    #[test]
    fn a_failed_first_fetch_hides_nothing_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let mut fail = |_: &Rule| Err(crate::Error::NoRefs("x".into()));
        let hidden = Hidden::load(
            &[rule(&[])],
            dir.path(),
            SystemTime::now(),
            false,
            &mut fail,
        );
        let (kept, n) = hidden.apply("git://other", vec![r("refs/tags/v1", "1111")]);
        assert_eq!((kept.len(), n.len()), (1, 0));
        assert!(hidden.note.is_some());
        assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
    }
}

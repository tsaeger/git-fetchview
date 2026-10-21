// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! One collected remote: a URL, an optional label, and tags.

use std::collections::{BTreeSet, HashMap};

use crate::git::redact_url;

/// A collected repo URL with an optional label and a set of tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Validated URL, with any embedded password removed.
    pub url: String,
    /// Short name; derived from the URL when absent.
    pub label: Option<String>,
    /// Where the entry came from: `merge`, `maintainers`, `manual`, ...
    pub tags: BTreeSet<String>,
}

impl Entry {
    /// An entry with no label and a single tag.
    #[must_use]
    pub fn new(url: impl Into<String>, tag: &str) -> Self {
        Self {
            url: url.into(),
            label: None,
            tags: BTreeSet::from([tag.to_owned()]),
        }
    }

    /// Identity of the repo, see [`key`].
    #[must_use]
    pub fn key(&self) -> String {
        key(&self.url)
    }

    /// The explicit label, or one derived from the URL.
    #[must_use]
    pub fn display_label(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| derive_label(&self.url))
    }
}

/// Why a URL was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Rejected(pub &'static str);

const WEB_SCHEMES: [&str; 3] = ["https", "http", "git"];

/// Check that `raw` is a URL we are willing to store and fetch from.
///
/// Returns the URL with any embedded password removed.
///
/// # Errors
/// [`Rejected`] with the reason when the URL is empty, malformed, uses an
/// unsupported scheme or transport helper, or could be read as an option.
pub fn validate_url(raw: &str) -> Result<String, Rejected> {
    let url = raw.trim();
    if url.is_empty() {
        return Err(Rejected("empty url"));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Rejected("whitespace or control character"));
    }
    if url.starts_with('-') {
        return Err(Rejected("starts with '-'"));
    }
    if url.starts_with("file:") {
        return Err(Rejected("file: urls are not allowed"));
    }
    if is_transport_helper(url) {
        return Err(Rejected("transport helper"));
    }
    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "https" | "http" | "git" | "ssh"
        ) {
            return Err(Rejected("unsupported scheme"));
        }
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.starts_with('-') {
            return Err(Rejected("authority starts with '-'"));
        }
        if authority.contains('%') {
            return Err(Rejected("percent-encoding in host"));
        }
        check_host(host_of(authority))?;
    } else {
        let Some((head, path)) = url.split_once(':') else {
            return Err(Rejected("not a url"));
        };
        if head.contains('/') {
            return Err(Rejected("not a url"));
        }
        if path.is_empty() {
            return Err(Rejected("empty path"));
        }
        if path.starts_with('-') {
            return Err(Rejected("path starts with '-'"));
        }
        if head.contains('%') {
            return Err(Rejected("percent-encoding in host"));
        }
        check_host(host_of(head))?;
    }
    Ok(drop_empty_port(&redact_url(url)))
}

/// `ext::...`, `fd::...` and similar: git runs a helper for these.
fn is_transport_helper(url: &str) -> bool {
    url.split_once("::").is_some_and(|(helper, _)| {
        !helper.is_empty()
            && helper
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    })
}

fn check_host(host: &str) -> Result<(), Rejected> {
    if host.is_empty() {
        Err(Rejected("empty host"))
    } else if host.starts_with('-') {
        Err(Rejected("host starts with '-'"))
    } else {
        Ok(())
    }
}

/// `user:pass@host:port` -> `host:port`.
fn hostport(authority: &str) -> &str {
    authority.rsplit_once('@').map_or(authority, |(_, h)| h)
}

/// `user:pass@host:port` -> `host`; handles `[::1]:22`.
fn host_of(authority: &str) -> &str {
    let hostport = hostport(authority);
    hostport
        .strip_prefix('[')
        .and_then(|h| h.split_once(']'))
        .map_or_else(
            || hostport.split(':').next().unwrap_or_default(),
            |(h, _)| h,
        )
}

/// `host:` -> `host`: a colon with no port number means the default port.
/// A lone trailing colon only; `[::1]` and `host::` are left alone.
fn strip_empty_port(authority: &str) -> &str {
    authority
        .strip_suffix(':')
        .filter(|rest| !rest.ends_with(':'))
        .unwrap_or(authority)
}

/// Drop an empty port from a `scheme://host:/path` URL.
fn drop_empty_port(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    format!("{scheme}://{}{path}", strip_empty_port(authority))
}

fn trim_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    path.trim_end_matches('/').to_owned()
}

/// Identity of a repo: two URLs with the same key name the same repo.
///
/// The host is lowercased, an empty port (`host:/path`) and a trailing `/`
/// or `.git` are ignored. For
/// `http`, `https` and `git` the scheme and user are ignored too. `ssh://`
/// and scp-style URLs keep theirs, since they are not interchangeable.
#[must_use]
pub fn key(url: &str) -> String {
    let url = url.trim();
    if let Some((scheme, rest)) = url.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
        let authority = strip_empty_port(authority);
        let path = trim_path(path);
        if WEB_SCHEMES.contains(&scheme.as_str()) {
            format!("{}{path}", hostport(authority).to_ascii_lowercase())
        } else {
            format!("{scheme}://{}{path}", authority.to_ascii_lowercase())
        }
    } else {
        let (head, path) = url.split_once(':').unwrap_or((url, ""));
        format!("{}:{}", head.to_ascii_lowercase(), trim_path(path))
    }
}

/// A short name from the last two path segments: `netdev/net-next`.
#[must_use]
pub fn derive_label(url: &str) -> String {
    let url = url.trim();
    let (host, path) = if let Some((_, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
        (host_of(authority), path)
    } else {
        let (head, path) = url.split_once(':').unwrap_or((url, ""));
        (host_of(head), path)
    };
    let path = trim_path(path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => host.to_owned(),
        [only] => format!("{host}/{only}"),
        [.., parent, last] => format!("{parent}/{last}"),
    }
}

/// Collapse entries with the same key. The first URL and label win, tags
/// are unioned, and order of first appearance is kept.
#[must_use]
pub fn merge_entries(entries: impl IntoIterator<Item = Entry>) -> Vec<Entry> {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Entry> = Vec::new();
    for entry in entries {
        let k = entry.key();
        if let Some(&i) = index.get(&k) {
            out[i].tags.extend(entry.tags);
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
    fn accepts_supported_forms() {
        for url in [
            "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git",
            "http://host/x",
            "git://git.kernel.org/pub/scm/x/y",
            "ssh://git@host/a/b",
            "git@github.com:a/b.git",
            "ssh://[::1]/a/b",
            "git@[::1]:a/b",
        ] {
            assert!(validate_url(url).is_ok(), "{url}");
        }
    }

    #[test]
    fn strips_embedded_password_but_keeps_user() {
        assert_eq!(
            validate_url("https://bob:pw@host/x").unwrap(),
            "https://bob@host/x"
        );
    }

    #[test]
    fn rejects_unsafe_or_malformed() {
        for url in [
            "",
            "   ",
            "ext::sh -c id",
            "ext::foo",
            "fd::7",
            "file:///etc/passwd",
            "/srv/git/x",
            "-oProxyCommand=x",
            "ftp://h/x",
            "https:///x",
            "https://-h/x",
            "https://h/a b",
            "host:",
            "a/b:c",
            "ssh://-oProxyCommand=touch${IFS}pwn@host/p",
            "ssh://%2DoProxyCommand=x/p",
            "git@host:-oProxyCommand=x",
            "git@-host:p",
        ] {
            assert!(validate_url(url).is_err(), "{url:?}");
        }
    }

    #[test]
    fn same_repo_spelled_differently_has_one_key() {
        let a = key("git://git.kernel.org/pub/scm/x/net");
        let b = key("https://git.kernel.org/pub/scm/x/net.git/");
        let c = key("HTTPS://GIT.Kernel.org/pub/scm/x/net");
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn an_empty_port_does_not_make_a_different_repo() {
        // From the kernel's own history: a colon after the host, then a path.
        let odd = "git://git.kernel.org:/pub/scm/fs/xfs/xfs-linux";
        let normal = "git://git.kernel.org/pub/scm/fs/xfs/xfs-linux";
        assert_eq!(key(odd), key(normal));
        assert_eq!(key("ssh://git@host:/a/b"), key("ssh://git@host/a/b"));
        assert_eq!(key("ssh://[::1]:/a/b"), key("ssh://[::1]/a/b"));
        assert_eq!(derive_label(odd), derive_label(normal));
    }

    #[test]
    fn a_real_port_still_matters() {
        assert_ne!(key("https://h:8443/p/x"), key("https://h/p/x"));
    }

    #[test]
    fn validate_accepts_an_empty_port_and_drops_it_from_the_stored_url() {
        assert_eq!(
            validate_url("git://git.kernel.org:/pub/scm/fs/xfs/xfs-linux").as_deref(),
            Ok("git://git.kernel.org/pub/scm/fs/xfs/xfs-linux")
        );
        assert_eq!(
            validate_url("ssh://[::1]:/a/b").as_deref(),
            Ok("ssh://[::1]/a/b")
        );
        assert_eq!(
            validate_url("https://h:8443/x").as_deref(),
            Ok("https://h:8443/x")
        );
        assert_eq!(validate_url("https://h/x").as_deref(), Ok("https://h/x"));
    }

    #[test]
    fn ssh_and_scp_forms_are_distinct_from_web_and_each_other() {
        let web = key("https://github.com/a/b");
        let scp = key("git@github.com:a/b.git");
        let ssh = key("ssh://git@github.com/a/b");
        assert_ne!(web, scp);
        assert_ne!(scp, ssh);
        assert_ne!(web, ssh);
    }

    #[test]
    fn labels_use_the_last_two_path_segments() {
        assert_eq!(
            derive_label("git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"),
            "netdev/net"
        );
        assert_eq!(
            derive_label("https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git/"),
            "netdev/net-next"
        );
        assert_eq!(
            derive_label("git@github.com:torvalds/linux.git"),
            "torvalds/linux"
        );
        assert_eq!(derive_label("https://example.com/x.git"), "example.com/x");
        assert_eq!(derive_label("https://example.com"), "example.com");
    }

    #[test]
    fn merge_entries_unions_tags_and_keeps_first() {
        let a = Entry::new("git://h/p/x", "merge");
        let b = Entry::new("https://h/p/x.git", "maintainers");
        let c = Entry::new("https://h/p/y", "manual");
        let merged = merge_entries([a, b, c]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].url, "git://h/p/x");
        assert_eq!(
            merged[0]
                .tags
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["maintainers", "merge"]
        );
    }
}

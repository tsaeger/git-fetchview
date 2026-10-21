<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com> -->

# Remote Sources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `git fetchview` collect remote URLs from merge commits, the `MAINTAINERS` file and plain URL lists into files that `fetchview.remotes` points at, and show them in the remote picker.

**Architecture:** A new `remotes` module holds pure parsers and a small file layer (entries, the remotes file, the config pointers, sources). `git config` stays the only configuration. The existing picker gets a combined remote list. A second TUI mode (multi-select) drives interactive `add`. Subcommands `add`, `list` and `remove` are thin wiring over the module.

**Tech Stack:** Rust 2024, clap derive, ratatui 0.30, thiserror. New dev-dependency: `tempfile`.

**Spec:** `docs/superpowers/specs/2026-10-20-remote-sources-design.md`

## Global Constraints

- Minimum Rust is `1.88.0`; edition 2024. Linux and macOS are the targets.
- Every `.rs`, `.toml`, `.md` and `.gitignore` file carries the SPDX header. After creating files run `cargo xtask headers --fix`.
- Workspace lints apply: `unsafe` forbidden, clippy pedantic, `missing_docs`. `cargo xtask clippy` denies warnings. Public functions that return `Result` need a `# Errors` doc section; public pure functions that return a value need `#[must_use]`.
- Settings live only in `git config` (`fetchview.*`). The one new key is `fetchview.remotes`, multi-valued, paths. No other config files, no env vars.
- Accepted URL forms only: `https://`, `http://`, `git://`, `ssh://` with a non-empty host, or scp-style `[user@]host:path`. Reject `ext::`/`fd::` style helpers, `file:`, bare paths, a leading `-`, whitespace or control characters, and an empty host.
- Remotes file: one entry per line, fields `url<TAB>label<TAB>tags`; `#` lines and blank lines are comments and are preserved.
- Dedupe key ignores scheme among `http`/`https`/`git`; prefer `https`, then `git`, then `http`, when one scan finds several. An existing entry's URL is never rewritten.
- Keys stay consistent: `Esc` does nothing at the top level, `q` quits from any level. In the multi-select only, `Enter` confirms.
- Commits follow `AGENTS.md`: Conventional Commits, a body wrapped at 79 characters, `git commit -s`, no `Co-Authored-By`. Show each message to the user and wait for approval before committing, unless a `.auto-approve-commit` file exists in the repo root. Each task ends with a proposed message.
- Writing style: plain, no filler or hype words (see `AGENTS.md`).
- Run the full check before each commit: `cargo xtask headers`, `cargo xtask clippy`, `cargo xtask test`.

## Review Focus

Inputs and conditions the spec implies but a straightforward test list could miss, most likely first. Each has a test in the task named.

1. A hostile URL arriving in a `MAINTAINERS` `T:` line, a merge subject or a shared remotes file (`ext::sh -c ...`, `-oProxyCommand=...`, `file:///...`) must be rejected and counted, never stored or fetched. Tasks 1, 4, 7.
2. The same repo spelled `git://host/p/x`, `https://host/p/x.git/` and `HTTPS://HOST/p/x` must be one entry, with `https` kept. Tasks 1, 4.
3. A registered file that is missing, unreadable or has garbage lines must produce a warning in the status line, not a crash and not a blank picker. Tasks 2, 3, 5.
4. A linked worktree must write the default file to the main repo's git dir, and a temp dir reached through a symlink (macOS `/var` vs `/private/var`) must not register the same file twice. Tasks 3, 7.
5. Merge subjects that carry no URL (`Merge remote-tracking branch 'a/b'`), have `into` suffixes, or contain the words " of " elsewhere must be skipped without being counted as rejected. Task 4.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/remotes/mod.rs` | Module wiring; `picker_remotes` (combine real remotes and entries) |
| `src/remotes/entry.rs` | `Entry`, `validate_url`, `key`, `derive_label`, `merge_entries` |
| `src/remotes/file.rs` | Parse, render, merge, remove, atomic write of a remotes file |
| `src/remotes/registry.rs` | Read `fetchview.remotes` pointers, register a path, load all files |
| `src/remotes/plan.rs` | Split scan results into new, tag-update and already-present entries |
| `src/remotes/source/mod.rs` | `Source` trait, `Scan`, `collect`, `dedupe` |
| `src/remotes/source/merges.rs` | `parse_merge_subject`, `Merges` source |
| `src/remotes/source/maintainers.rs` | `parse_maintainers`, `Maintainers` source |
| `src/remotes/source/manual.rs` | `parse_url_list`, `Manual` source |
| `src/tui/select.rs` | Multi-select state machine |
| `src/commands.rs` | `add`, `list`, `remove` orchestration |
| `tests/cli.rs` | End-to-end tests against the built binary in temp repos |
| `fuzz/fuzz_targets/*.rs` | Four new fuzz targets |

Modified: `src/lib.rs`, `src/error.rs`, `src/git.rs`, `src/cli.rs`, `src/tui/{app,keys,list,mod,ui}.rs`, `Cargo.toml`, `fuzz/Cargo.toml`, `README.md`, `AGENTS.md`, `CHANGELOG.md`.

---

### Task 1: URL validation, identity and labels

**Files:**
- Create: `src/remotes/mod.rs`, `src/remotes/entry.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `crate::git::redact_url(&str) -> String`.
- Produces (used by every later task):
  - `pub struct Entry { pub url: String, pub label: Option<String>, pub tags: BTreeSet<String> }`
  - `Entry::new(url: impl Into<String>, tag: &str) -> Entry`, `Entry::key(&self) -> String`, `Entry::display_label(&self) -> String`
  - `pub struct Rejected(pub &'static str)` (Display = the reason)
  - `pub fn validate_url(raw: &str) -> Result<String, Rejected>`
  - `pub fn key(url: &str) -> String`
  - `pub fn derive_label(url: &str) -> String`
  - `pub fn merge_entries(entries: impl IntoIterator<Item = Entry>) -> Vec<Entry>`

- [ ] **Step 1: Wire the module and write the failing tests**

`src/lib.rs`: add `pub mod remotes;` after `pub mod git;`.

`src/remotes/mod.rs`:

```rust
//! Remotes collected outside `.git/config`, kept in files.

pub mod entry;
```

`src/remotes/entry.rs` (tests only for now):

```rust
//! One collected remote: a URL, an optional label, and tags.

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
        assert_eq!(derive_label("git@github.com:torvalds/linux.git"), "torvalds/linux");
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
            merged[0].tags.iter().map(String::as_str).collect::<Vec<_>>(),
            ["maintainers", "merge"]
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors such as ``cannot find function `validate_url` in this scope``.

- [ ] **Step 3: Implement**

Insert above the `tests` module in `src/remotes/entry.rs`:

```rust
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
        check_host(host_of(head))?;
    }
    Ok(redact_url(url))
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

fn trim_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    path.trim_end_matches('/').to_owned()
}

/// Identity of a repo: two URLs with the same key name the same repo.
///
/// The host is lowercased and a trailing `/` or `.git` is ignored. For
/// `http`, `https` and `git` the scheme and user are ignored too. `ssh://`
/// and scp-style URLs keep theirs, since they are not interchangeable.
#[must_use]
pub fn key(url: &str) -> String {
    let url = url.trim();
    if let Some((scheme, rest)) = url.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all tests pass, including the 7 new ones.

- [ ] **Step 5: Lint and headers**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`
Expected: no warnings. Fix any pedantic lint by the smallest change (for example add `# Errors` docs or `#[must_use]`).

- [ ] **Step 6: Commit**

Show the user this message, wait for approval, then run `git add src/lib.rs src/remotes && git commit -s -F -` with it:

```
feat(remotes): validate and identify remote urls

Add Entry and the URL rules for collected remotes. validate_url accepts
https, http, git and ssh URLs and scp-style addresses, and refuses
transport helpers like ext::, file: URLs, bare paths, options and empty
hosts. key() makes git://, https:// and .git spellings of one repo
compare equal, and derive_label() makes a short name from the path.
```

---

### Task 2: The remotes file

**Files:**
- Create: `src/remotes/file.rs`
- Modify: `src/remotes/mod.rs`, `src/error.rs`, `Cargo.toml`

**Interfaces:**
- Consumes: `Entry`, `validate_url`, `Rejected` from Task 1.
- Produces:
  - `Error::Io { path: PathBuf, source: io::Error }`
  - `pub struct Parsed { pub entries: Vec<Entry>, pub warnings: Vec<String> }`
  - `pub fn parse(text: &str) -> Parsed` (warnings read `line N: reason`)
  - `pub fn render_line(entry: &Entry) -> String`
  - `pub fn merge(text: &str, add: &[Entry]) -> String`
  - `pub fn remove(text: &str, target_key: &str) -> (String, usize)`
  - `pub fn write_atomic(path: &Path, text: &str) -> Result<()>`

- [ ] **Step 1: Add the dev-dependency and error variant**

Run: `cargo add --dev tempfile`

`src/error.rs`: add `use std::path::PathBuf;` next to the other imports, and this variant before `Terminal`:

```rust
    /// Reading or writing a file failed.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
```

`src/remotes/mod.rs`: add `pub mod file;`.

- [ ] **Step 2: Write the failing tests**

`src/remotes/file.rs`:

```rust
//! The remotes file: one entry per line, `url<TAB>label<TAB>tags`.

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# collected 2026-10-20
https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\tnetdev/net\tmaintainers

git://git.kernel.org/pub/scm/linux/kernel/git/davem/net-next
";

    #[test]
    fn parses_fields_and_skips_comments_and_blanks() {
        let parsed = parse(SAMPLE);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].label.as_deref(), Some("netdev/net"));
        assert!(parsed.entries[0].tags.contains("maintainers"));
        assert_eq!(parsed.entries[1].label, None);
        assert!(parsed.entries[1].tags.is_empty());
    }

    #[test]
    fn bad_lines_become_warnings_with_line_numbers() {
        let parsed = parse("https://ok/x\next::sh\n\n-oBad\n");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.warnings.len(), 2);
        assert!(parsed.warnings[0].starts_with("line 2:"));
        assert!(parsed.warnings[1].starts_with("line 4:"));
    }

    #[test]
    fn merge_appends_new_entries_and_keeps_everything_else() {
        let add = [Entry::new("https://h/p/new", "manual")];
        let out = merge(SAMPLE, &add);
        assert!(out.starts_with(SAMPLE));
        assert!(out.ends_with("https://h/p/new\t\tmanual\n"));
    }

    #[test]
    fn merge_unions_tags_into_a_matching_line_only_when_they_change() {
        let same = [Entry::new("https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net", "maintainers")];
        assert_eq!(merge(SAMPLE, &same), SAMPLE);

        let more = [Entry::new("https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net", "merge")];
        let out = merge(SAMPLE, &more);
        assert!(out.contains("netdev/net\tmaintainers,merge\n"));
        assert_eq!(out.lines().count(), SAMPLE.lines().count());
    }

    #[test]
    fn merge_appends_a_duplicate_candidate_once() {
        let add = [
            Entry::new("https://h/p/x", "a"),
            Entry::new("git://h/p/x", "b"),
        ];
        let out = merge("", &add);
        assert_eq!(out.lines().count(), 1);
    }

    #[test]
    fn merge_into_text_without_trailing_newline() {
        let out = merge("https://h/a", &[Entry::new("https://h/b", "t")]);
        assert_eq!(out, "https://h/a\nhttps://h/b\t\tt\n");
    }

    #[test]
    fn remove_drops_matching_lines_and_counts_them() {
        let k = key("https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net");
        let (out, n) = remove(SAMPLE, &k);
        assert_eq!(n, 1);
        assert!(!out.contains("netdev/net"));
        assert!(out.contains("# collected"));
    }

    #[test]
    fn write_atomic_creates_parents_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/remotes");
        write_atomic(&path, "one\n").unwrap();
        write_atomic(&path, "two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two\n");
        assert!(!dir.path().join("a/b/.remotes.tmp").exists());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors, `cannot find function parse`.

- [ ] **Step 4: Implement**

Insert above the `tests` module in `src/remotes/file.rs`:

```rust
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::Path;

use super::entry::{Entry, Rejected, key, validate_url};
use crate::error::{Error, Result};

/// Entries read from a file, plus a warning for each line that was skipped.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Valid entries, in file order.
    pub entries: Vec<Entry>,
    /// `line N: reason` for each rejected line.
    pub warnings: Vec<String>,
}

enum Line {
    Skip,
    Entry(Entry),
    Bad(&'static str),
}

fn parse_line(line: &str) -> Line {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Line::Skip;
    }
    let mut fields = line.split('\t');
    let raw_url = fields.next().unwrap_or_default();
    let url = match validate_url(raw_url) {
        Ok(url) => url,
        Err(Rejected(reason)) => return Line::Bad(reason),
    };
    let label = fields
        .next()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned);
    let tags = fields
        .next()
        .map(|t| {
            t.split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    Line::Entry(Entry { url, label, tags })
}

/// Parse a remotes file. Bad lines are skipped and reported, never fatal.
#[must_use]
pub fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    for (i, line) in text.lines().enumerate() {
        match parse_line(line) {
            Line::Skip => {}
            Line::Entry(entry) => parsed.entries.push(entry),
            Line::Bad(reason) => parsed.warnings.push(format!("line {}: {reason}", i + 1)),
        }
    }
    parsed
}

/// One entry as a file line, without the newline.
#[must_use]
pub fn render_line(entry: &Entry) -> String {
    let mut line = entry.url.clone();
    if entry.label.is_some() || !entry.tags.is_empty() {
        line.push('\t');
        line.push_str(entry.label.as_deref().unwrap_or(""));
    }
    if !entry.tags.is_empty() {
        line.push('\t');
        line.push_str(&entry.tags.iter().cloned().collect::<Vec<_>>().join(","));
    }
    line
}

/// Add `add` to the file text.
///
/// Lines, comments and order are kept. An entry whose key is already in the
/// file only has its tags unioned in, and its line is rewritten only if the
/// tags changed. Other entries are appended once each.
#[must_use]
pub fn merge(text: &str, add: &[Entry]) -> String {
    let mut wanted: HashMap<String, &Entry> = HashMap::new();
    for entry in add {
        wanted.entry(entry.key()).or_insert(entry);
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = String::new();
    for line in text.lines() {
        if let Line::Entry(existing) = parse_line(line) {
            let k = existing.key();
            if let Some(extra) = wanted.get(&k) {
                seen.insert(k);
                let mut merged = existing.clone();
                merged.tags.extend(extra.tags.iter().cloned());
                if merged.tags == existing.tags {
                    out.push_str(line);
                } else {
                    out.push_str(&render_line(&merged));
                }
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    for entry in add {
        let k = entry.key();
        if seen.insert(k) {
            out.push_str(&render_line(entry));
            out.push('\n');
        }
    }
    out
}

/// Drop every entry line whose key is `target_key`. Returns the new text and
/// how many lines were dropped.
#[must_use]
pub fn remove(text: &str, target_key: &str) -> (String, usize) {
    let mut out = String::new();
    let mut removed = 0;
    for line in text.lines() {
        if let Line::Entry(entry) = parse_line(line) {
            if entry.key() == target_key {
                removed += 1;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    (out, removed)
}

/// Write `text` to `path` by writing a sibling temporary file and renaming.
///
/// # Errors
/// [`Error::Io`] if the directory, temporary file or rename fails.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let io_err = |source: io::Error| Error::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(io_err)?;
    }
    let name = path
        .file_name()
        .map_or_else(|| "remotes".into(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&tmp, text).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all pass. If `merge_appends_new_entries_and_keeps_everything_else` fails on the final line, check that `render_line` writes `url<TAB><TAB>tag` for an entry with tags and no label.

- [ ] **Step 6: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `Cargo.toml Cargo.lock src/error.rs src/remotes`:

```
feat(remotes): read and write the remotes file

Add the tab-separated remotes file: parse with per-line warnings, edit
while keeping comments and order, drop entries by key, and write
atomically through a temporary file and rename. A line that fails
validation is skipped and reported, so a shared or hand-edited file
cannot break the picker. Adds the Io error and the tempfile dev
dependency.
```

---

### Task 3: Config pointers and loading files

**Files:**
- Create: `src/remotes/registry.rs`
- Modify: `src/remotes/mod.rs`, `src/git.rs`

**Interfaces:**
- Consumes: `file::{parse, Parsed}`, `Entry`, `merge_entries`, `Cmd`, `Error`.
- Produces:
  - `Cmd::config_paths(key) -> Cmd`, `Cmd::config_add(scope_flag, key, value) -> Cmd`, `Cmd::git_common_dir() -> Cmd`, `Cmd::toplevel() -> Cmd`
  - `pub const KEY: &str = "fetchview.remotes"`, `pub const DEFAULT_FILE: &str = "fetchview.remotes"`
  - `pub enum Scope { Local, Global }`
  - `pub fn parse_pointers(text: &str) -> Vec<PathBuf>`
  - `pub fn files() -> Result<Vec<PathBuf>>`
  - `pub fn default_file() -> Result<PathBuf>`
  - `pub fn same_path(a: &Path, b: &Path) -> bool`
  - `pub fn register(stored: &str, resolved: &Path, scope: Scope) -> Result<bool>`
  - `pub struct LoadedFile { pub path: PathBuf, pub text: String, pub entries: Vec<Entry> }`
  - `pub struct Loaded { pub files: Vec<LoadedFile>, pub warnings: Vec<String> }` with `Loaded::merged(&self) -> Vec<Entry>`
  - `pub fn load(paths: &[PathBuf]) -> Loaded`

- [ ] **Step 1: Add the git commands with a test**

`src/git.rs`, inside `impl Cmd` after `config_get`:

```rust
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
```

In the `tests` module of `src/git.rs` add:

```rust
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
```

- [ ] **Step 2: Write the failing registry tests**

`src/remotes/mod.rs`: add `pub mod registry;`.

`src/remotes/registry.rs`:

```rust
//! Which remotes files `git config` points at, and loading them.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_values_are_kept() {
        let text = "file:/home/u/.gitconfig\t/data/k.remotes\n";
        assert_eq!(parse_pointers(text), [PathBuf::from("/data/k.remotes")]);
    }

    #[test]
    fn relative_values_resolve_against_the_config_files_directory() {
        let text = "file:/home/u/.config/git/config\tlists/k.remotes\n\
                    file:.git/config\tfetchview.remotes\n";
        assert_eq!(
            parse_pointers(text),
            [
                PathBuf::from("/home/u/.config/git/lists/k.remotes"),
                PathBuf::from(".git/fetchview.remotes"),
            ]
        );
    }

    #[test]
    fn values_from_the_command_line_or_stdin_are_ignored() {
        let text = "command line:\tx\nstandard input:\ty\nblob:abc\tz\n";
        assert!(parse_pointers(text).is_empty());
    }

    #[test]
    fn duplicate_paths_are_listed_once_in_order() {
        let text = "file:/a/config\t/x\nfile:/b/config\t/y\nfile:/c/config\t/x\n";
        assert_eq!(
            dedupe_paths(parse_pointers(text)),
            [PathBuf::from("/x"), PathBuf::from("/y")]
        );
    }

    #[test]
    fn load_reports_missing_and_bad_lines_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good");
        std::fs::write(&good, "https://h/p/a\t\tmanual\next::sh\n").unwrap();
        let missing = dir.path().join("missing");
        let loaded = load(&[good.clone(), missing.clone()]);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].entries.len(), 1);
        assert_eq!(loaded.warnings.len(), 2);
        assert!(loaded.warnings[0].contains("line 2"));
        assert!(loaded.warnings[1].contains("not found"));
    }

    #[test]
    fn merged_dedupes_across_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::write(&a, "git://h/p/x\t\tmerge\n").unwrap();
        std::fs::write(&b, "https://h/p/x.git\t\tmaintainers\n").unwrap();
        let merged = load(&[a, b]).merged();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tags.len(), 2);
    }

    #[test]
    fn same_path_sees_through_a_symlinked_directory() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(same_path(&real.join("f"), &link.join("f")));
        assert!(!same_path(&real.join("f"), &link.join("g")));
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors, `cannot find function parse_pointers`.

- [ ] **Step 4: Implement**

Insert above the `tests` module in `src/remotes/registry.rs`:

```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::entry::{Entry, merge_entries};
use super::file;
use crate::error::{Error, Result};
use crate::git::Cmd;

/// The multi-valued config key that lists remotes files.
pub const KEY: &str = "fetchview.remotes";

/// Name of the default file inside the git common dir.
pub const DEFAULT_FILE: &str = "fetchview.remotes";

/// Which config file to register a path in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The repository's config.
    Local,
    /// `~/.gitconfig`.
    Global,
}

impl Scope {
    fn flag(self) -> &'static str {
        match self {
            Self::Local => "--local",
            Self::Global => "--global",
        }
    }
}

/// Turn `git config --show-origin --get-all` output into file paths.
///
/// A relative value is resolved against the directory of the config file it
/// came from, like `include.path`. Values from the command line, stdin or a
/// blob have no directory and are ignored.
#[must_use]
pub fn parse_pointers(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| {
            let (origin, value) = line.split_once('\t')?;
            let origin_file = origin.strip_prefix("file:")?;
            let value = Path::new(value);
            if value.is_absolute() {
                return Some(value.to_owned());
            }
            let base = Path::new(origin_file)
                .parent()
                .unwrap_or_else(|| Path::new(""));
            Some(base.join(value))
        })
        .collect()
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Every remotes file registered at any config level, in git's order.
///
/// # Errors
/// Git cannot be run.
pub fn files() -> Result<Vec<PathBuf>> {
    match Cmd::config_paths(KEY).output() {
        Ok(text) => Ok(dedupe_paths(parse_pointers(&text))),
        // `git config --get-all` exits 1 when the key is unset.
        Err(Error::Failed { status, .. }) if status.code() == Some(1) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// The default remotes file, inside the git common dir so that linked
/// worktrees share it.
///
/// # Errors
/// Not in a git repository, or git cannot be run.
pub fn default_file() -> Result<PathBuf> {
    let dir = Cmd::git_common_dir().output()?;
    Ok(PathBuf::from(dir.trim()).join(DEFAULT_FILE))
}

/// Make a path absolute and resolve symlinks in its directory, so the same
/// file compares equal however it was spelled. The file need not exist.
fn normalize(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    match (abs.parent().and_then(|d| d.canonicalize().ok()), abs.file_name()) {
        (Some(dir), Some(name)) => dir.join(name),
        _ => abs,
    }
}

/// True if both paths name the same file.
#[must_use]
pub fn same_path(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// Add `stored` to `fetchview.remotes` in `scope`, unless `resolved` is
/// already registered anywhere. Returns whether it was added.
///
/// # Errors
/// Git cannot be run or refuses to write the config.
pub fn register(stored: &str, resolved: &Path, scope: Scope) -> Result<bool> {
    if files()?.iter().any(|f| same_path(f, resolved)) {
        return Ok(false);
    }
    Cmd::config_add(scope.flag(), KEY, stored).output()?;
    Ok(true)
}

/// One registered file after reading.
#[derive(Debug, Clone)]
pub struct LoadedFile {
    /// Where it was read from.
    pub path: PathBuf,
    /// The raw text, for editing.
    pub text: String,
    /// Its valid entries.
    pub entries: Vec<Entry>,
}

/// Everything read from the registered files, and what went wrong.
#[derive(Debug, Default)]
pub struct Loaded {
    /// Files that were read.
    pub files: Vec<LoadedFile>,
    /// A message for each missing file, unreadable file or bad line.
    pub warnings: Vec<String>,
}

impl Loaded {
    /// All entries, deduplicated by key across files.
    #[must_use]
    pub fn merged(&self) -> Vec<Entry> {
        merge_entries(self.files.iter().flat_map(|f| f.entries.iter().cloned()))
    }
}

/// Read every file. Problems become warnings, never errors.
#[must_use]
pub fn load(paths: &[PathBuf]) -> Loaded {
    let mut loaded = Loaded::default();
    for path in paths {
        match fs::read_to_string(path) {
            Ok(text) => {
                let parsed = file::parse(&text);
                loaded.warnings.extend(
                    parsed
                        .warnings
                        .into_iter()
                        .map(|w| format!("{}: {w}", path.display())),
                );
                loaded.files.push(LoadedFile {
                    path: path.clone(),
                    text,
                    entries: parsed.entries,
                });
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                loaded.warnings.push(format!("{}: not found", path.display()));
            }
            Err(e) => loaded.warnings.push(format!("{}: {e}", path.display())),
        }
    }
    loaded
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all pass.

- [ ] **Step 6: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `src/git.rs src/remotes`:

```
feat(remotes): read file pointers from git config

Read fetchview.remotes with --show-origin so a relative path resolves
against the config file that holds it, as include.path does. Load every
registered file, turning missing files and bad lines into warnings.
Registering a path compares files after resolving symlinked directories,
so a temp dir reached by two spellings is not added twice. The default
file lives in the git common dir so linked worktrees share it.
```

---

### Task 4: Sources

**Files:**
- Create: `src/remotes/source/mod.rs`, `src/remotes/source/merges.rs`, `src/remotes/source/maintainers.rs`, `src/remotes/source/manual.rs`
- Modify: `src/remotes/mod.rs`, `src/git.rs`

**Interfaces:**
- Consumes: `Entry`, `validate_url`, `Cmd`, `Error::Io`.
- Produces:
  - `Cmd::log_merges(range: &str, max: Option<usize>) -> Cmd`
  - `Cmd::for_each_line(&self, f: impl FnMut(&str)) -> Result<()>`
  - `pub struct Scan { pub entries: Vec<Entry>, pub rejected: usize }`
  - `pub trait Source { fn scan(&self) -> Result<Scan>; }`
  - `pub fn collect<'a>(urls: impl IntoIterator<Item = &'a str>, tag: &str) -> Scan`
  - `pub fn dedupe(entries: Vec<Entry>) -> Vec<Entry>`
  - `merges::{parse_merge_subject(&str) -> Option<&str>, Merges { pub range: String, pub max: Option<usize> }}`
  - `maintainers::{parse_maintainers(&str) -> Vec<&str>, Maintainers { pub path: PathBuf }}`
  - `manual::{parse_url_list(&str) -> Vec<&str>, Manual { pub urls: Vec<String> }}`

- [ ] **Step 1: Add the streaming git helpers with tests**

`src/git.rs` imports: add `use std::io::{BufRead, BufReader};`.

In `impl Cmd`:

```rust
    /// `git log --merges --format=%s [-n N] <range> --`
    #[must_use]
    pub fn log_merges(range: &str, max: Option<usize>) -> Self {
        let mut args = vec!["log".to_owned(), "--merges".to_owned(), "--format=%s".to_owned()];
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
        let stdout = child.stdout.take().expect("stdout is piped");
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
```

In `git.rs` tests:

```rust
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
```

- [ ] **Step 2: Write the failing parser tests**

`src/remotes/mod.rs`: add `pub mod source;`.

`src/remotes/source/mod.rs`:

```rust
//! Places that remote URLs can be collected from.

pub mod maintainers;
pub mod manual;
pub mod merges;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_validates_counts_rejects_and_tags() {
        let scan = collect(["https://h/p/a", "ext::sh -c id", "-oBad", "git://h/p/b"], "manual");
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
    fn dedupe_keeps_ssh_separate() {
        let out = dedupe(vec![
            Entry::new("https://h/p/x", "t"),
            Entry::new("git@h:p/x", "t"),
        ]);
        assert_eq!(out.len(), 2);
    }
}
```

`src/remotes/source/merges.rs`:

```rust
//! Pull URLs from merge commit subjects.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_url_after_of() {
        assert_eq!(
            parse_merge_subject("Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"),
            Some("git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net")
        );
        assert_eq!(
            parse_merge_subject("Merge branch 'for-linus' of https://git.kernel.org/pub/scm/x/foo.git"),
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
```

`src/remotes/source/maintainers.rs`:

```rust
//! Pull `T:` git URLs from a MAINTAINERS file.

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
```

`src/remotes/source/manual.rs`:

```rust
//! URLs given by hand, in arguments or a file.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_and_comment_lines() {
        let text = "# list\n\n https://h/a \nssh://h/b\n#git://h/c\n";
        assert_eq!(parse_url_list(text), ["https://h/a", "ssh://h/b"]);
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors for the missing functions.

- [ ] **Step 4: Implement the shared pieces**

Insert in `src/remotes/source/mod.rs` between the `pub mod` lines and `#[cfg(test)]`:

```rust
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
```

- [ ] **Step 5: Implement the three sources**

`src/remotes/source/merges.rs`, above the tests:

```rust
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
```

`src/remotes/source/maintainers.rs`, above the tests:

```rust
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
```

`src/remotes/source/manual.rs`, above the tests:

```rust
use super::{Scan, Source, collect};
use crate::error::Result;

/// One URL per line; blank lines and `#` comments are skipped.
#[must_use]
pub fn parse_url_list(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// URLs given directly.
#[derive(Debug, Clone, Default)]
pub struct Manual {
    /// The URLs, unvalidated.
    pub urls: Vec<String>,
}

impl Source for Manual {
    fn scan(&self) -> Result<Scan> {
        Ok(collect(self.urls.iter().map(String::as_str), "manual"))
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all pass. If `skips_subjects_without_a_url` fails for the `Revert "Merge tag ..."` case, confirm `strip_prefix("Merge ")` is anchored at the start.

- [ ] **Step 7: Add a source test against a real repo**

Append to the `tests` module in `src/remotes/source/merges.rs`:

```rust
    #[test]
    fn scans_merge_commits_from_a_real_repo() {
        use std::process::Command;
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@example.org")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@example.org")
                .output()
                .unwrap();
            assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["commit", "-q", "--allow-empty", "-m", "root"]);
        git(&["checkout", "-q", "-b", "side"]);
        git(&["commit", "-q", "--allow-empty", "-m", "side"]);
        git(&["checkout", "-q", "main"]);
        git(&["commit", "-q", "--allow-empty", "-m", "main"]);
        git(&["merge", "-q", "--no-ff", "side", "-m", "Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"]);

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let scan = Merges { range: "HEAD".into(), max: None }.scan();
        std::env::set_current_dir(prev).unwrap();

        let scan = scan.unwrap();
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.rejected, 0);
        assert!(scan.entries[0].tags.contains("merge"));
    }
```

Changing the process working directory is not safe if other unit tests run at the same time and depend on it, and none of them do. If a later task adds such a test, move this one to `tests/cli.rs`, where each test runs the binary with its own `current_dir`.

Run: `cargo xtask test` and confirm it passes.

- [ ] **Step 8: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `src/git.rs src/remotes`:

```
feat(remotes): add merges, maintainers and url sources

Add the Source trait and three sources. merges reads merge commit
subjects from git log as a stream and takes the URL after "of".
maintainers takes the URL from T: git lines of a MAINTAINERS file.
url takes URLs from arguments or a list. All URLs go through
validate_url, rejects are counted, and duplicates collapse with the
https spelling preferred.
```

---

### Task 5: Collected remotes in the picker

**Files:**
- Modify: `src/git.rs`, `src/remotes/mod.rs`, `src/tui/app.rs`, `src/tui/ui.rs`, `src/tui/mod.rs`, `src/cli.rs`

**Interfaces:**
- Consumes: `Entry`, `key`, `registry::{files, load}`, `Loaded::merged`.
- Produces:
  - `git::Remote` gains `pub tags: Vec<String>`
  - `git::configured_remotes() -> Result<Vec<Remote>>` (replaces `remotes()`; may be empty)
  - `remotes::picker_remotes(real: Vec<Remote>, entries: &[Entry]) -> Vec<Remote>`
  - `App::with_notice(self, Option<String>) -> App`, `App::notice(&self) -> Option<&str>`
  - `tui::pick(remotes, theme, notice: Option<String>)`

- [ ] **Step 1: Write the failing tests**

`src/remotes/mod.rs`: add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn real(name: &str, url: &str) -> Remote {
        Remote { name: name.into(), url: url.into(), tags: Vec::new() }
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
```

In `src/tui/app.rs` tests module add:

```rust
    #[test]
    fn rows_show_tags_only_for_collected_remotes() {
        let a = App::new(vec![
            Remote { name: "origin".into(), url: "u".into(), tags: Vec::new() },
            Remote { name: "netdev/net".into(), url: "v".into(), tags: vec!["merge".into(), "maintainers".into()] },
        ]);
        assert_eq!(
            a.list().visible_labels(),
            ["origin  u", "netdev/net  v  [merge,maintainers]"]
        );
    }

    #[test]
    fn the_notice_clears_on_the_first_action() {
        let mut a = app().with_notice(Some("2 files missing".into()));
        assert_eq!(a.notice(), Some("2 files missing"));
        a.update(Action::Down, &mut refs);
        assert_eq!(a.notice(), None);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors, unknown field `tags` and missing `picker_remotes`.

- [ ] **Step 3: Implement**

`src/git.rs`:

- Add `/// Labels for where a collected remote came from; empty for real remotes.\n    pub tags: Vec<String>,` to `Remote`.
- In `parse_remotes`, build `Remote { name: ..., url: ..., tags: Vec::new() }`.
- Replace `remotes()` with:

```rust
/// List the repository's configured remotes. May be empty.
///
/// # Errors
/// Git cannot be run, or this is not a repository.
pub fn configured_remotes() -> Result<Vec<Remote>> {
    Ok(parse_remotes(&Cmd::list_remotes().output()?))
}
```

- Update the `parse_remotes` test and any literal `Remote { ... }` in tests to include `tags: Vec::new()`. In `src/tui/app.rs` tests the `app()` helper builds two remotes; add `tags: Vec::new()` to both.

`src/remotes/mod.rs` (above the tests):

```rust
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
```

`src/tui/app.rs`:

- Add field `notice: Option<String>,` to `App` and `notice: None,` in `new`.
- Build row labels with a helper:

```rust
fn remote_label(r: &Remote) -> String {
    if r.tags.is_empty() {
        format!("{}  {}", r.name, r.url)
    } else {
        format!("{}  {}  [{}]", r.name, r.url, r.tags.join(","))
    }
}
```

  and in `new` use `remotes.iter().map(remote_label).collect()`.
- Add:

```rust
    /// Show a message in the status line until the next key press.
    #[must_use]
    pub fn with_notice(mut self, notice: Option<String>) -> Self {
        self.notice = notice;
        self
    }

    /// The pending notice, if any.
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
```

- At the top of `update`, next to `self.error = None;`, add `self.notice = None;`.

`src/tui/ui.rs`: in the status-line chain, add a branch between `app.typing()` and the filter-empty check:

```rust
    } else if let Some(notice) = app.notice() {
        Line::from(Span::styled(notice.to_owned(), Style::new().fg(theme.accent)))
```

`src/tui/mod.rs`: change `pick` to

```rust
pub fn pick(remotes: Vec<Remote>, theme: ThemeName, notice: Option<String>) -> Result<Option<Selection>> {
    let theme = Theme::from(theme);
    let mut app = App::new(remotes).with_notice(notice);
```

`src/cli.rs`: replace the start of `pick_and_view`:

```rust
fn pick_and_view(theme_flag: Option<ThemeName>) -> Result<()> {
    let config = Config::load(theme_flag)?;
    let loaded = registry::load(&registry::files()?);
    let remotes = remotes::picker_remotes(git::configured_remotes()?, &loaded.merged());
    if remotes.is_empty() {
        return Err(Error::NoRemotes);
    }
    let Some(selection) = tui::pick(remotes, config.theme, notice(&loaded.warnings))? else {
        return Ok(());
    };
```

and add:

```rust
/// One line for the status bar: the first warning and how many more there are.
fn notice(warnings: &[String]) -> Option<String> {
    match warnings {
        [] => None,
        [one] => Some(one.clone()),
        [first, rest @ ..] => Some(format!("{first} (+{} more)", rest.len())),
    }
}
```

Fix the imports in `cli.rs` (`use crate::error::{Error, Result};`, `use crate::remotes::{self, registry};`) and add a test for `notice` in a `#[cfg(test)]` module at the bottom of `cli.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::notice;

    #[test]
    fn notice_summarizes_warnings() {
        assert_eq!(notice(&[]), None);
        assert_eq!(notice(&["a".into()]), Some("a".into()));
        assert_eq!(notice(&["a".into(), "b".into(), "c".into()]), Some("a (+2 more)".into()));
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all pass.

- [ ] **Step 5: Try it by hand**

Run, from a repo with a remote:
`printf 'https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\t\tmanual\n' > .git/fetchview.remotes && git config --add fetchview.remotes fetchview.remotes && cargo run -q --bin git-fetchview`
Expected: the picker lists the real remotes, then `netdev/net  https://...  [manual]`. Press `q`. Remove the config entry afterwards: `git config --unset fetchview.remotes`.

- [ ] **Step 6: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `src/`:

```
feat: list collected remotes in the picker

Read the files that fetchview.remotes points at and add their entries to
the remote picker after the real remotes. A collected entry for a repo
that is already a real remote is hidden. Rows show the tags, so / can
filter by source. Missing files and bad lines show as a notice in the
status line. The picker now starts when there are collected entries but
no real remote.
```

---

### Task 6: Multi-select picker

**Files:**
- Create: `src/tui/select.rs`
- Modify: `src/tui/keys.rs`, `src/tui/list.rs`, `src/tui/app.rs`, `src/tui/ui.rs`, `src/tui/mod.rs`

**Interfaces:**
- Consumes: `List`, `Action`, `Theme`.
- Produces:
  - `Action::Toggle`, `Action::ToggleAll`
  - `List::visible_rows(&self) -> Vec<(usize, &str)>`
  - `pub struct Row { pub label: String, pub have: bool }`
  - `pub enum SelectFlow { Continue, Quit, Done(Vec<usize>) }`
  - `Select::{new(Vec<Row>), update(Action) -> SelectFlow, list(), typing(), is_checked(usize), is_have(usize), checked_count(), selectable_count()}`
  - `tui::select_many(rows: Vec<Row>, theme: ThemeName) -> Result<Option<Vec<usize>>>`
  - `ui::draw_select(frame, &Select, &Theme)`

- [ ] **Step 1: Write the failing tests**

`src/tui/keys.rs` tests module, add:

```rust
    #[test]
    fn space_and_a_toggle_but_only_when_not_typing() {
        assert_eq!(map(key(KeyCode::Char(' ')), false), Some(Action::Toggle));
        assert_eq!(map(key(KeyCode::Char('a')), false), Some(Action::ToggleAll));
        assert_eq!(map(key(KeyCode::Char(' ')), true), Some(Action::Char(' ')));
        assert_eq!(map(key(KeyCode::Char('a')), true), Some(Action::Char('a')));
    }
```

`src/tui/list.rs` tests module, add:

```rust
    #[test]
    fn visible_rows_pair_original_index_with_label() {
        let mut l = list();
        "heads".chars().for_each(|c| l.push(c));
        assert_eq!(l.visible_rows(), [(0, "refs/heads/main"), (1, "refs/heads/Dev")]);
    }
```

`src/tui/select.rs`:

```rust
//! Multi-select list used by `add`.

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        vec![
            Row { label: "net  https://h/net".into(), have: false },
            Row { label: "mm  https://h/mm".into(), have: true },
            Row { label: "usb  https://h/usb".into(), have: false },
        ]
    }

    #[test]
    fn space_toggles_the_row_under_the_cursor() {
        let mut s = Select::new(rows());
        s.update(Action::Toggle);
        assert!(s.is_checked(0));
        s.update(Action::Toggle);
        assert!(!s.is_checked(0));
    }

    #[test]
    fn rows_already_present_cannot_be_checked() {
        let mut s = Select::new(rows());
        s.update(Action::Down);
        s.update(Action::Toggle);
        assert!(!s.is_checked(1));
        assert_eq!(s.checked_count(), 0);
        assert_eq!(s.selectable_count(), 2);
    }

    #[test]
    fn a_checks_all_visible_then_unchecks_them() {
        let mut s = Select::new(rows());
        s.update(Action::ToggleAll);
        assert_eq!((s.is_checked(0), s.is_checked(1), s.is_checked(2)), (true, false, true));
        s.update(Action::ToggleAll);
        assert_eq!(s.checked_count(), 0);
    }

    #[test]
    fn a_only_touches_rows_the_filter_shows() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        s.update(Action::Char('u'));
        s.update(Action::Char('s'));
        s.update(Action::Char('b'));
        s.update(Action::Enter);
        s.update(Action::ToggleAll);
        assert!(s.is_checked(2));
        assert!(!s.is_checked(0));
    }

    #[test]
    fn enter_confirms_with_checked_indexes_in_order() {
        let mut s = Select::new(rows());
        s.update(Action::ToggleAll);
        assert_eq!(s.update(Action::Enter), SelectFlow::Done(vec![0, 2]));
    }

    #[test]
    fn enter_while_typing_only_confirms_the_search() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        assert_eq!(s.update(Action::Enter), SelectFlow::Continue);
        assert!(!s.typing());
    }

    #[test]
    fn q_quits_and_esc_does_nothing_at_the_top_level() {
        let mut s = Select::new(rows());
        assert_eq!(s.update(Action::Back), SelectFlow::Continue);
        assert_eq!(s.update(Action::Quit), SelectFlow::Quit);
    }

    #[test]
    fn esc_while_typing_abandons_the_search() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        s.update(Action::Char('x'));
        s.update(Action::Back);
        assert!(!s.typing());
        assert_eq!(s.list().filter(), "");
    }

    #[test]
    fn checks_survive_a_filter_change() {
        let mut s = Select::new(rows());
        s.update(Action::Toggle);
        s.update(Action::Search);
        s.update(Action::Char('u'));
        s.update(Action::Enter);
        s.update(Action::Clear);
        assert!(s.is_checked(0));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: compile errors for `Action::Toggle`, `visible_rows`, `Select`.

- [ ] **Step 3: Implement keys and list**

`src/tui/keys.rs`: add to `Action`

```rust
    /// Toggle the row under the cursor (multi-select).
    Toggle,
    /// Toggle every visible row (multi-select).
    ToggleAll,
```

and to `map`, with the other non-typing arms:

```rust
        (false, KeyCode::Char(' ')) => Some(Action::Toggle),
        (false, KeyCode::Char('a')) => Some(Action::ToggleAll),
```

Add two rows to the keymap table in the module doc: `` `Space` | toggle a row (multi-select) `` and `` `a` | toggle all visible rows (multi-select) ``.

`src/tui/app.rs`: change the catch-all arm in `update` to

```rust
            Action::Quit
            | Action::Char(_)
            | Action::Backspace
            | Action::Toggle
            | Action::ToggleAll => {}
```

`src/tui/list.rs`:

```rust
    /// Visible rows as `(original index, label)`.
    #[must_use]
    pub fn visible_rows(&self) -> Vec<(usize, &str)> {
        self.visible()
            .into_iter()
            .map(|i| (i, self.labels[i].as_str()))
            .collect()
    }
```

- [ ] **Step 4: Implement `Select`**

`src/tui/select.rs`, above the tests:

```rust
use std::collections::BTreeSet;

use crate::tui::keys::Action;
use crate::tui::list::List;

/// One selectable row.
#[derive(Debug, Clone)]
pub struct Row {
    /// Text to show and filter on.
    pub label: String,
    /// Already present: shown, but cannot be checked.
    pub have: bool,
}

/// What the event loop should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectFlow {
    /// Keep going.
    Continue,
    /// Leave without choosing anything.
    Quit,
    /// Confirmed: the checked rows, as original indexes in order.
    Done(Vec<usize>),
}

/// Multi-select state.
#[derive(Debug)]
pub struct Select {
    list: List,
    have: Vec<bool>,
    checked: BTreeSet<usize>,
    typing: bool,
}

impl Select {
    /// Build from rows.
    #[must_use]
    pub fn new(rows: Vec<Row>) -> Self {
        let have = rows.iter().map(|r| r.have).collect();
        let list = List::new(rows.into_iter().map(|r| r.label).collect());
        Self {
            list,
            have,
            checked: BTreeSet::new(),
            typing: false,
        }
    }

    /// The underlying list.
    #[must_use]
    pub fn list(&self) -> &List {
        &self.list
    }

    /// True while a search is being typed.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.typing
    }

    /// Whether row `i` (original index) is checked.
    #[must_use]
    pub fn is_checked(&self, i: usize) -> bool {
        self.checked.contains(&i)
    }

    /// Whether row `i` is already present.
    #[must_use]
    pub fn is_have(&self, i: usize) -> bool {
        self.have[i]
    }

    /// How many rows are checked.
    #[must_use]
    pub fn checked_count(&self) -> usize {
        self.checked.len()
    }

    /// How many rows can be checked at all.
    #[must_use]
    pub fn selectable_count(&self) -> usize {
        self.have.iter().filter(|h| !**h).count()
    }

    /// Apply one action.
    pub fn update(&mut self, action: Action) -> SelectFlow {
        if action == Action::Quit {
            return SelectFlow::Quit;
        }
        if self.typing {
            self.update_typing(action);
            return SelectFlow::Continue;
        }
        match action {
            Action::Down => self.list.down(),
            Action::Up => self.list.up(),
            Action::Top => self.list.top(),
            Action::Bottom => self.list.bottom(),
            Action::Search => self.typing = true,
            Action::Clear => self.list.clear(),
            Action::Toggle => {
                if let Some(i) = self.list.selected() {
                    self.toggle(i);
                }
            }
            Action::ToggleAll => self.toggle_all(),
            Action::Enter => return SelectFlow::Done(self.checked.iter().copied().collect()),
            Action::Back | Action::Quit | Action::Char(_) | Action::Backspace => {}
        }
        SelectFlow::Continue
    }

    fn update_typing(&mut self, action: Action) {
        match action {
            Action::Char(c) => self.list.push(c),
            Action::Backspace => self.list.pop(),
            Action::Enter => self.typing = false,
            Action::Back => {
                self.list.clear();
                self.typing = false;
            }
            _ => {}
        }
    }

    fn toggle(&mut self, i: usize) {
        if self.have[i] {
            return;
        }
        if !self.checked.remove(&i) {
            self.checked.insert(i);
        }
    }

    fn toggle_all(&mut self) {
        let visible: Vec<usize> = self
            .list
            .visible()
            .into_iter()
            .filter(|&i| !self.have[i])
            .collect();
        let all_checked = !visible.is_empty() && visible.iter().all(|i| self.checked.contains(i));
        for i in visible {
            if all_checked {
                self.checked.remove(&i);
            } else {
                self.checked.insert(i);
            }
        }
    }
}
```

`src/tui/mod.rs`: add `pub mod select;`, and

```rust
use select::{Row, Select, SelectFlow};

/// Run the multi-select. Returns `None` if the user quit, otherwise the
/// checked rows as indexes into `rows`.
///
/// # Errors
/// Terminal I/O failures.
pub fn select_many(rows: Vec<Row>, theme: ThemeName) -> Result<Option<Vec<usize>>> {
    let theme = Theme::from(theme);
    let mut state = Select::new(rows);
    let mut terminal = ratatui::init();
    let result = run_select(&mut terminal, &mut state, &theme);
    ratatui::restore();
    result
}

fn run_select(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut Select,
    theme: &Theme,
) -> Result<Option<Vec<usize>>> {
    loop {
        terminal.draw(|frame| ui::draw_select(frame, state, theme))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(action) = keys::map(key, state.typing()) else {
            continue;
        };
        match state.update(action) {
            SelectFlow::Continue => {}
            SelectFlow::Quit => return Ok(None),
            SelectFlow::Done(chosen) => return Ok(Some(chosen)),
        }
    }
}
```

- [ ] **Step 5: Replace `src/tui/ui.rs`**

Keep the two SPDX header lines at the top, then:

```rust
//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List as ListWidget, ListItem, ListState, Paragraph};

use crate::tui::app::{App, Level};
use crate::tui::select::Select;
use crate::tui::theme::Theme;

const HINTS: &str = "j/k move  l/enter open  h/esc back  / search  c clear  q quit";
const SELECT_HINTS: &str = "space toggle  a all  enter write  / search  c clear  q quit";

fn base(theme: &Theme) -> Style {
    Style::new().fg(theme.fg).bg(theme.bg)
}

fn render_list(
    frame: &mut Frame<'_>,
    area: Rect,
    title: String,
    items: Vec<ListItem<'_>>,
    cursor: usize,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            title,
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::new().fg(theme.dim))
        .style(base(theme));
    let list = ListWidget::new(items)
        .block(block)
        .highlight_style(Style::new().bg(theme.selection).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(cursor));
    frame.render_stateful_widget(list, area, &mut state);
}

fn typing_line<'a>(filter: &str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled("/", Style::new().fg(theme.accent)),
        Span::raw(filter.to_owned()),
        Span::styled("█", Style::new().fg(theme.accent)),
    ])
}

fn hint_line<'a>(text: &str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(text.to_owned(), Style::new().fg(theme.dim)))
}

/// Draw the picker.
pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme) {
    frame.render_widget(Block::new().style(base(theme)), frame.area());
    let [body, status] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let title = match (app.level(), app.chosen_remote()) {
        (Level::Refs, Some(remote)) => format!(" refs of {remote} "),
        _ => " remotes ".to_owned(),
    };
    let items = app
        .list()
        .visible_labels()
        .into_iter()
        .map(ListItem::new)
        .collect();
    render_list(frame, body, title, items, app.list().cursor(), theme);

    let line = if let Some(err) = app.error() {
        Line::from(Span::styled(err.to_owned(), Style::new().fg(theme.error)))
    } else if app.typing() {
        typing_line(app.list().filter(), theme)
    } else if let Some(notice) = app.notice() {
        Line::from(Span::styled(notice.to_owned(), Style::new().fg(theme.accent)))
    } else if app.list().filter().is_empty() {
        hint_line(HINTS, theme)
    } else {
        Line::from(vec![
            Span::styled(format!("/{}", app.list().filter()), Style::new().fg(theme.accent)),
            Span::styled("  c clear", Style::new().fg(theme.dim)),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}

/// Draw the multi-select.
pub fn draw_select(frame: &mut Frame<'_>, select: &Select, theme: &Theme) {
    frame.render_widget(Block::new().style(base(theme)), frame.area());
    let [body, status] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let items = select
        .list()
        .visible_rows()
        .into_iter()
        .map(|(i, label)| {
            if select.is_have(i) {
                ListItem::new(Line::from(Span::styled(
                    format!("(have) {label}"),
                    Style::new().fg(theme.dim),
                )))
            } else {
                let mark = if select.is_checked(i) { "[x]" } else { "[ ]" };
                ListItem::new(format!("{mark} {label}"))
            }
        })
        .collect();
    render_list(frame, body, " add remotes ".to_owned(), items, select.list().cursor(), theme);

    let line = if select.typing() {
        typing_line(select.list().filter(), theme)
    } else {
        let counts = format!(
            "{} checked of {}  ",
            select.checked_count(),
            select.selectable_count()
        );
        let mut spans = vec![Span::styled(counts, Style::new().fg(theme.accent))];
        spans.push(Span::styled(SELECT_HINTS, Style::new().fg(theme.dim)));
        Line::from(spans)
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all pass, including 9 `select` tests, the new keys test and the list test.

- [ ] **Step 7: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `src/tui`:

```
feat(tui): add a multi-select picker

Add Select, a second mode of the list for choosing many rows. Space
toggles the row under the cursor, a toggles all visible rows so it
respects the / filter, and Enter confirms the whole selection. Rows
that are already present show as (have) and cannot be checked. q quits
and Esc does nothing at the top level, as in the main picker. Checks
survive filter changes. ui.rs shares its list rendering between the two
modes.
```

---

### Task 7: The `add`, `list` and `remove` commands

**Files:**
- Create: `src/remotes/plan.rs`, `src/commands.rs`, `tests/cli.rs`
- Modify: `src/remotes/mod.rs`, `src/lib.rs`, `src/error.rs`, `src/cli.rs`

**Interfaces:**
- Consumes: everything from Tasks 1 to 6.
- Produces:
  - `plan::Plan { pub new: Vec<Entry>, pub tag_updates: Vec<Entry>, pub present: Vec<Entry> }` and `plan::plan(candidates: Vec<Entry>, target: &[Entry], elsewhere: &[Entry]) -> Plan`
  - `Error::{NotATerminal, NotFound(String), Ambiguous { target: String, matches: String }}`
  - `cli::{AddArgs, AddSource}` and `Command::{Add(AddArgs), List, Remove { target: String }}`
  - `commands::{add(&AddArgs) -> Result<()>, list() -> Result<()>, remove(&str) -> Result<()>}`

- [ ] **Step 1: Write the failing plan tests**

`src/remotes/mod.rs`: add `pub mod plan;`.

`src/remotes/plan.rs`:

```rust
//! Decide what `add` should do with the candidates a source found.

#[cfg(test)]
mod tests {
    use super::*;

    fn e(url: &str) -> Entry {
        Entry::new(url, "t")
    }

    #[test]
    fn splits_new_tag_updates_and_present() {
        let candidates = vec![e("https://h/p/new"), e("https://h/p/here"), e("https://h/p/there")];
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
```

`src/remotes/plan.rs` implementation, above the tests:

```rust
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
```

- [ ] **Step 2: Write the failing end-to-end tests**

`tests/cli.rs`:

```rust
//! End-to-end tests: run the built binary in throwaway repos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const NET: &str = "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git";

struct Repo {
    dir: TempDir,
    home: TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["commit", "-q", "--allow-empty", "-m", "root"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn global_config(&self) -> PathBuf {
        self.home.path().join("gitconfig")
    }

    fn isolate(&self, cmd: &mut Command, cwd: &Path) {
        cmd.current_dir(cwd)
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_GLOBAL", self.global_config())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.org")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.org")
            .stdin(Stdio::null());
    }

    fn git_in(&self, cwd: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.args(args);
        self.isolate(&mut cmd, cwd);
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(self.path(), args)
    }

    fn fv_in(&self, cwd: &Path, args: &[&str]) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_git-fetchview"));
        cmd.args(args);
        self.isolate(&mut cmd, cwd);
        cmd.output().unwrap()
    }

    fn fv(&self, args: &[&str]) -> Output {
        self.fv_in(self.path(), args)
    }

    fn remotes_file(&self) -> String {
        fs::read_to_string(self.path().join(".git/fetchview.remotes")).unwrap_or_default()
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn add_url_writes_the_default_file_and_registers_it_once() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(repo.remotes_file().contains("netdev/net") || repo.remotes_file().contains(NET));
    assert_eq!(repo.git(&["config", "--get-all", "fetchview.remotes"]).trim(), "fetchview.remotes");

    let again = repo.fv(&["add", "--all", "url", NET]);
    assert!(again.status.success());
    assert_eq!(repo.git(&["config", "--get-all", "fetchview.remotes"]).lines().count(), 1);
    assert_eq!(repo.remotes_file().lines().count(), 1);
}

#[test]
fn list_shows_label_url_tags_and_file() {
    let repo = Repo::new();
    repo.fv(&["add", "--all", "url", NET]);
    let out = repo.fv(&["list"]);
    let text = stdout(&out);
    assert!(text.starts_with("netdev/net\thttps://git.kernel.org/"), "{text}");
    assert!(text.contains("\tmanual\t"));
    assert!(text.contains("fetchview.remotes"));
}

#[test]
fn dry_run_changes_nothing() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "--dry-run", "url", NET]);
    assert!(out.status.success());
    assert!(stdout(&out).contains(NET));
    assert_eq!(repo.remotes_file(), "");
    assert!(repo.git_in(repo.path(), &["config", "--get-all", "--default", "", "fetchview.remotes"]).trim().is_empty());
}

#[test]
fn without_a_terminal_add_needs_all() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "url", NET]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("terminal"), "{}", stderr(&out));
    assert_eq!(repo.remotes_file(), "");
}

#[test]
fn hostile_urls_are_rejected_and_counted() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "url", "ext::sh -c id", "-oProxyCommand=x", "file:///etc", NET]);
    assert!(out.status.success());
    assert!(stderr(&out).contains("rejected 3"), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1);
    assert!(!file.contains("ext::"));
}

#[test]
fn add_merges_finds_pull_urls() {
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "side"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "side"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "main"]);
    repo.git(&["merge", "-q", "--no-ff", "side", "-m", "Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"]);
    repo.git(&["checkout", "-q", "-b", "other", "main~1"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "other"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["merge", "-q", "--no-ff", "other", "-m", "Merge remote-tracking branch 'a/b'"]);

    let out = repo.fv(&["add", "--all", "merges"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1, "{file}");
    assert!(file.contains("git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"));
    assert!(file.contains("\tmerge"));
    assert!(stderr(&out).contains("rejected 0"));
}

#[test]
fn add_maintainers_reads_t_lines() {
    let repo = Repo::new();
    fs::write(
        repo.path().join("MAINTAINERS"),
        "NET\nT:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\nT:\tquilt https://x/y\n",
    )
    .unwrap();
    let out = repo.fv(&["add", "--all", "maintainers"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(repo.remotes_file().contains("\tmaintainers"));
    assert_eq!(repo.remotes_file().lines().count(), 1);
}

#[test]
fn the_same_repo_from_two_sources_is_one_line_with_both_tags() {
    let repo = Repo::new();
    fs::write(
        repo.path().join("MAINTAINERS"),
        "T:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\n",
    )
    .unwrap();
    repo.fv(&["add", "--all", "maintainers"]);
    repo.fv(&["add", "--all", "url", NET]);
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1, "{file}");
    assert!(file.contains("maintainers,manual"), "{file}");
}

#[test]
fn remove_by_label_edits_the_file() {
    let repo = Repo::new();
    repo.fv(&["add", "--all", "url", NET, "https://git.kernel.org/pub/scm/linux/kernel/git/davem/net-next.git"]);
    let out = repo.fv(&["remove", "netdev/net"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1);
    assert!(file.contains("net-next"));
    assert!(!repo.fv(&["remove", "nothing/here"]).status.success());
}

#[test]
fn a_relative_pointer_in_the_global_config_resolves_against_that_config() {
    let repo = Repo::new();
    let cfg_dir = repo.home.path().join("cfg");
    fs::create_dir_all(cfg_dir.join("lists")).unwrap();
    fs::write(cfg_dir.join("lists/k.remotes"), format!("{NET}\tmine\tshared\n")).unwrap();
    fs::write(
        repo.global_config(),
        format!("[include]\n\tpath = {}\n", cfg_dir.join("gitconfig").display()),
    )
    .unwrap();
    fs::write(cfg_dir.join("gitconfig"), "[fetchview]\n\tremotes = lists/k.remotes\n").unwrap();

    let out = repo.fv(&["list"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).starts_with("mine\t"), "{}", stdout(&out));
}

#[test]
fn a_linked_worktree_shares_the_default_file() {
    let repo = Repo::new();
    let wt = repo.home.path().join("wt");
    repo.git(&["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "wt"]);
    let out = repo.fv_in(&wt, &["add", "--all", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(repo.remotes_file().lines().count(), 1);
    assert!(!wt.join(".git").is_dir());
    let listed = repo.fv(&["list"]);
    assert!(stdout(&listed).contains("netdev/net"));
}

#[test]
fn file_and_global_register_an_explicit_file_in_the_global_config() {
    let repo = Repo::new();
    let file = repo.home.path().join("shared.remotes");
    let out = repo.fv(&["add", "--all", "--file", file.to_str().unwrap(), "--global", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(fs::read_to_string(&file).unwrap().contains("netdev/net") || fs::read_to_string(&file).unwrap().contains(NET));
    let cfg = fs::read_to_string(repo.global_config()).unwrap();
    assert!(cfg.contains("shared.remotes"), "{cfg}");
    assert_eq!(repo.remotes_file(), "");
}

#[test]
fn global_without_file_is_refused() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "--global", "url", NET]);
    assert!(!out.status.success());
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo xtask test`
Expected: the plan test fails to compile; the `tests/cli.rs` tests fail because the `add`, `list` and `remove` subcommands do not exist (clap error `unrecognized subcommand`).

- [ ] **Step 4: Implement `plan`**

Add the implementation shown in Step 1 above the tests in `src/remotes/plan.rs`.

- [ ] **Step 5: Add error variants**

`src/error.rs`, before `Terminal`:

```rust
    /// `add` needs to ask which remotes to add, but there is no terminal.
    #[error("no terminal to choose from; pass --all to add every new remote")]
    NotATerminal,

    /// `remove` matched nothing.
    #[error("no collected remote matches `{0}`")]
    NotFound(String),

    /// `remove` matched more than one repo.
    #[error("`{target}` matches more than one remote:\n{matches}")]
    Ambiguous {
        /// What the user typed.
        target: String,
        /// One `label  url` per line.
        matches: String,
    },
```

- [ ] **Step 6: Add the CLI definitions**

`src/cli.rs`: add `use std::path::PathBuf;`, `use clap::Args;`, and `use crate::commands;`. Extend `Command` and add the argument types:

```rust
    /// Collect remotes from a source into a remotes file.
    Add(AddArgs),
    /// List collected remotes.
    List,
    /// Remove a collected remote by URL or label.
    Remove {
        /// A URL or an exact label.
        target: String,
    },
```

```rust
/// Arguments for `add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Write to this file instead of `.git/fetchview.remotes`, and register it.
    #[arg(long, global = true)]
    pub file: Option<PathBuf>,

    /// Register the file in `~/.gitconfig` instead of the local config.
    #[arg(long, requires = "file", global = true)]
    pub global: bool,

    /// Add every new remote without opening the picker.
    #[arg(long, global = true)]
    pub all: bool,

    /// Print what would be written and change nothing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Where to look.
    #[command(subcommand)]
    pub source: AddSource,
}

/// Where `add` looks for remotes.
#[derive(Debug, Subcommand)]
pub enum AddSource {
    /// Pull URLs from merge commit subjects.
    Merges {
        /// Stop after this many merges.
        #[arg(short = 'n', long)]
        max: Option<usize>,
        /// Revision range (default: HEAD).
        range: Option<String>,
    },
    /// Pull `T: git` URLs from a MAINTAINERS file.
    Maintainers {
        /// Path to the file (default: MAINTAINERS at the worktree root).
        path: Option<PathBuf>,
    },
    /// Add URLs given by hand.
    Url {
        /// Read URLs from this file, one per line; `-` is stdin.
        #[arg(long)]
        from_file: Option<String>,
        /// URLs to add.
        urls: Vec<String>,
    },
}
```

In `run`, add arms:

```rust
        Some(Command::Add(args)) => commands::add(args),
        Some(Command::List) => commands::list(),
        Some(Command::Remove { target }) => commands::remove(target),
```

`src/lib.rs`: add `pub mod commands;`.

- [ ] **Step 7: Implement `src/commands.rs`**

```rust
//! The `add`, `list` and `remove` subcommands.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use crate::cli::{AddArgs, AddSource};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::git::Cmd;
use crate::remotes::entry::{Entry, key, validate_url};
use crate::remotes::file::{merge, remove as remove_lines, render_line, write_atomic};
use crate::remotes::plan::plan;
use crate::remotes::registry::{self, Scope};
use crate::remotes::source::maintainers::Maintainers;
use crate::remotes::source::manual::{Manual, parse_url_list};
use crate::remotes::source::merges::Merges;
use crate::remotes::source::{Scan, Source};
use crate::tui::{self, select::Row};

fn io_err(path: &std::path::Path) -> impl Fn(io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_owned(),
        source,
    }
}

fn scan(source: &AddSource) -> Result<Scan> {
    match source {
        AddSource::Merges { max, range } => Merges {
            range: range.clone().unwrap_or_else(|| "HEAD".to_owned()),
            max: *max,
        }
        .scan(),
        AddSource::Maintainers { path } => {
            let path = match path {
                Some(path) => path.clone(),
                None => PathBuf::from(Cmd::toplevel().output()?.trim()).join("MAINTAINERS"),
            };
            Maintainers { path }.scan()
        }
        AddSource::Url { from_file, urls } => {
            let mut all = urls.clone();
            if let Some(from) = from_file {
                let text = if from == "-" {
                    let mut text = String::new();
                    io::stdin()
                        .read_to_string(&mut text)
                        .map_err(io_err(std::path::Path::new("<stdin>")))?;
                    text
                } else {
                    let path = PathBuf::from(from);
                    fs::read_to_string(&path).map_err(io_err(&path))?
                };
                all.extend(parse_url_list(&text).into_iter().map(str::to_owned));
            }
            Manual { urls: all }.scan()
        }
    }
}

/// The file to write, the value to store in config, and the config scope.
fn target(args: &AddArgs) -> Result<(PathBuf, String, Scope)> {
    let scope = if args.global { Scope::Global } else { Scope::Local };
    match &args.file {
        Some(file) => {
            let abs = std::path::absolute(file).map_err(io_err(file))?;
            let stored = abs.to_string_lossy().into_owned();
            Ok((abs, stored, scope))
        }
        None => Ok((
            registry::default_file()?,
            registry::DEFAULT_FILE.to_owned(),
            scope,
        )),
    }
}

fn row_label(entry: &Entry) -> String {
    let tags = entry.tags.iter().cloned().collect::<Vec<_>>().join(",");
    format!("{}  {}  [{tags}]", entry.display_label(), entry.url)
}

/// Collect remotes from a source into a remotes file.
///
/// # Errors
/// The source cannot be read, there is no terminal and `--all` was not
/// given, or the file or config cannot be written.
pub fn add(args: &AddArgs) -> Result<()> {
    let theme = Config::load(None)?.theme;
    let scanned = scan(&args.source)?;
    let (target_path, stored, scope) = target(args)?;

    let loaded = registry::load(&registry::files()?);
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
    let (in_target, elsewhere): (Vec<_>, Vec<_>) = loaded
        .files
        .iter()
        .partition(|f| registry::same_path(&f.path, &target_path));
    let in_target: Vec<Entry> = in_target.into_iter().flat_map(|f| f.entries.clone()).collect();
    let elsewhere: Vec<Entry> = elsewhere.into_iter().flat_map(|f| f.entries.clone()).collect();
    let plan = plan(scanned.entries, &in_target, &elsewhere);

    let chosen: Vec<Entry> = if args.all {
        plan.new.clone()
    } else {
        if !(io::stdin().is_terminal() && io::stdout().is_terminal()) {
            return Err(Error::NotATerminal);
        }
        let rows: Vec<Row> = plan
            .new
            .iter()
            .map(|e| Row { label: row_label(e), have: false })
            .chain(plan.present.iter().map(|e| Row { label: row_label(e), have: true }))
            .collect();
        match tui::select_many(rows, theme)? {
            None => return Ok(()),
            Some(picked) => picked
                .into_iter()
                .filter(|&i| i < plan.new.len())
                .map(|i| plan.new[i].clone())
                .collect(),
        }
    };

    let summary = format!(
        "added {}, tags updated {}, already present {}, rejected {}",
        chosen.len(),
        plan.tag_updates.len(),
        plan.present.len(),
        scanned.rejected
    );
    let mut write = chosen;
    write.extend(plan.tag_updates);

    if args.dry_run {
        println!("would write to {}:", target_path.display());
        for entry in &write {
            println!("{}", render_line(entry));
        }
        eprintln!("{summary}");
        return Ok(());
    }
    if write.is_empty() {
        eprintln!("{summary}");
        return Ok(());
    }

    let existing = match fs::read_to_string(&target_path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(io_err(&target_path)(e)),
    };
    write_atomic(&target_path, &merge(&existing, &write))?;
    registry::register(&stored, &target_path, scope)?;
    eprintln!("{summary}");
    Ok(())
}

/// Print every collected remote: `label  url  tags  file`, tab-separated.
///
/// # Errors
/// Git cannot be run.
pub fn list() -> Result<()> {
    let loaded = registry::load(&registry::files()?);
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
    for file in &loaded.files {
        for entry in &file.entries {
            let tags = entry.tags.iter().cloned().collect::<Vec<_>>().join(",");
            println!(
                "{}\t{}\t{}\t{}",
                entry.display_label(),
                entry.url,
                tags,
                file.path.display()
            );
        }
    }
    Ok(())
}

/// Remove a collected remote, matched by URL or exact label, from every
/// registered file that has it.
///
/// # Errors
/// Nothing matches, more than one repo matches, or a file cannot be written.
pub fn remove(target: &str) -> Result<()> {
    let loaded = registry::load(&registry::files()?);
    let wanted_key = validate_url(target).ok().map(|url| key(&url));
    let mut matches: BTreeMap<String, String> = BTreeMap::new();
    for file in &loaded.files {
        for entry in &file.entries {
            if wanted_key.as_deref() == Some(entry.key().as_str()) || entry.display_label() == target {
                matches.insert(entry.key(), format!("{}  {}", entry.display_label(), entry.url));
            }
        }
    }
    let k = match matches.len() {
        0 => return Err(Error::NotFound(target.to_owned())),
        1 => matches.keys().next().cloned().unwrap_or_default(),
        _ => {
            return Err(Error::Ambiguous {
                target: target.to_owned(),
                matches: matches.values().cloned().collect::<Vec<_>>().join("\n"),
            });
        }
    };
    for file in &loaded.files {
        let (text, removed) = remove_lines(&file.text, &k);
        if removed > 0 {
            write_atomic(&file.path, &text)?;
            eprintln!("removed from {}", file.path.display());
        }
    }
    Ok(())
}
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo xtask test`
Expected: all unit tests and the 12 end-to-end tests pass. Likely snags and fixes:
- If `without_a_terminal_add_needs_all` hangs, the TUI was entered; check that the `is_terminal` test runs before `select_many`.
- If `hostile_urls_are_rejected_and_counted` reports `rejected 2`, the `file:///etc` argument is not being passed: check that `urls` is a trailing positional list.
- If the worktree test writes into the worktree, `default_file` is using `--git-dir` instead of `--git-common-dir`.

- [ ] **Step 9: Try the picker by hand**

Run, in a real clone: `cargo run -q --bin git-fetchview -- add maintainers` in a kernel tree, or `cargo run -q --bin git-fetchview -- add merges -n 200`.
Expected: the multi-select opens. `/net` filters, `a` checks the visible rows, `Space` toggles one, `Enter` writes, `q` writes nothing.

- [ ] **Step 10: Lint, headers and commit**

Run: `cargo xtask headers --fix && cargo fmt --all && cargo xtask clippy`

Propose, wait for approval, then commit `src tests`:

```
feat: add the add, list and remove commands

git fetchview add merges|maintainers|url scans a source, drops invalid
URLs, and writes new entries to .git/fetchview.remotes in the git common
dir, registering it in the local config. --file writes elsewhere and
--global registers that file globally. --all skips the picker and
--dry-run prints what would be written. Without a terminal and without
--all it fails instead of hanging. list prints every collected remote
and remove deletes one by URL or label. End-to-end tests cover each
source, dedupe across sources, linked worktrees and relative pointers.
```

---

### Task 8: Fuzz targets, documentation and changelog

**Files:**
- Create: `fuzz/fuzz_targets/parse_merge_subject.rs`, `parse_maintainers.rs`, `parse_remotes_file.rs`, `validate_url.rs`
- Modify: `fuzz/Cargo.toml`, `README.md`, `AGENTS.md`, `CHANGELOG.md`

- [ ] **Step 1: Add the fuzz targets**

`fuzz/fuzz_targets/parse_merge_subject.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = git_fetchview::remotes::source::merges::parse_merge_subject(text);
});
```

`fuzz/fuzz_targets/parse_maintainers.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = git_fetchview::remotes::source::maintainers::parse_maintainers(text);
});
```

`fuzz/fuzz_targets/parse_remotes_file.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let parsed = git_fetchview::remotes::file::parse(text);
    // Everything that parses must survive a render and merge round trip.
    let merged = git_fetchview::remotes::file::merge("", &parsed.entries);
    let _ = git_fetchview::remotes::file::parse(&merged);
});
```

`fuzz/fuzz_targets/validate_url.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    if let Ok(url) = git_fetchview::remotes::entry::validate_url(text) {
        // Whatever is accepted must never look like an option or a helper.
        assert!(!url.starts_with('-'));
        assert!(!url.contains("::") || url.contains("://"));
        let _ = git_fetchview::remotes::entry::key(&url);
        let _ = git_fetchview::remotes::entry::derive_label(&url);
    }
});
```

`fuzz/Cargo.toml`: add, after the existing `[[bin]]` blocks and before `[workspace]`:

```toml
[[bin]]
name = "parse_merge_subject"
path = "fuzz_targets/parse_merge_subject.rs"
test = false
doc = false

[[bin]]
name = "parse_maintainers"
path = "fuzz_targets/parse_maintainers.rs"
test = false
doc = false

[[bin]]
name = "parse_remotes_file"
path = "fuzz_targets/parse_remotes_file.rs"
test = false
doc = false

[[bin]]
name = "validate_url"
path = "fuzz_targets/validate_url.rs"
test = false
doc = false
```

- [ ] **Step 2: Run the targets briefly if cargo-fuzz is installed**

Run: `cargo xtask fuzz validate_url -- -max_total_time=20` and the same for the other three names.
Expected: no crash. If `cargo fuzz` or the nightly toolchain is missing, skip this step and say so in the commit review; the targets still compile-check with `cd fuzz && cargo check` on a nightly toolchain.

If `validate_url` finds an input that violates the assertions, fix `validate_url` first (add the input as a case in `rejects_unsafe_or_malformed`), then re-run.

- [ ] **Step 3: Commit the fuzz targets**

Propose, wait for approval, then commit `fuzz`:

```
test: fuzz the remote source parsers

Add fuzz targets for the merge subject parser, the MAINTAINERS parser,
the remotes file parser and URL validation. The file target checks that
parsed entries survive a render and parse round trip. The URL target
checks that an accepted URL never starts with a dash and never contains
a transport helper marker.
```

- [ ] **Step 4: Document the feature in the README**

`README.md`: insert this section before `## Configuration`:

````markdown
## Collecting remotes

A kernel tree has far more interesting repos than any clone has remotes.
`git fetchview add` collects them into files, and the picker lists them after
your real remotes.

```
git fetchview add merges -n 500          # pull URLs from merge commits in HEAD
git fetchview add merges v6.11..v6.12    # or from a range
git fetchview add maintainers            # T: git lines of ./MAINTAINERS
git fetchview add url https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git
git fetchview add url --from-file urls.txt
git fetchview list
git fetchview remove netdev/net
```

`add` shows a picker of what it found: `Space` toggles a row, `a` toggles every
visible row, `/` filters, `Enter` writes, `q` writes nothing. Rows already
collected show as `(have)`. `--all` adds everything new without the picker, and
`--dry-run` prints what would be written.

Entries are written to `.git/fetchview.remotes` (shared by linked worktrees) and
that path is added to `fetchview.remotes` in the local config. To keep one set
for every clone:

```
git fetchview add --file ~/.local/share/fetchview/kernel.remotes --global merges
```

`fetchview.remotes` can list any number of files, at any config level. A
relative path is relative to the config file that names it.

Each line of a remotes file is `url<TAB>label<TAB>tags`. Lines starting with `#`
are comments and are kept when the file is rewritten. The label defaults to the
last two path segments (`netdev/net`), and the tags record where the entry came
from, so `/maintainers` in the picker shows only those.

URLs from merge subjects and `MAINTAINERS` are untrusted. Only `https`, `http`,
`git`, `ssh` and scp-style URLs are accepted. Anything else, such as `ext::`
helpers or `file:` URLs, is rejected and counted in the summary.
````

- [ ] **Step 5: Update `AGENTS.md`**

Append to the conventions list:

```markdown
- **Collected remotes:** `src/remotes/` holds pure parsers (`entry`, `file`,
  `source/*`) and the git-config layer (`registry`). Keep parsers free of I/O
  so they stay fuzzable. A new source implements `remotes::source::Source` and
  gets a `fetchview add <name>` subcommand in `cli.rs` and `commands.rs`.
- **Untrusted URLs:** every URL from a source or a remotes file goes through
  `remotes::entry::validate_url` before it is stored or passed to git.
- `tui::select` is the multi-select. `Enter` confirms there; everywhere else it
  drops in.
```

- [ ] **Step 6: Regenerate the changelog**

Run: `cargo xtask changelog`
Expected: `CHANGELOG.md` gains the new commits under `[Unreleased]`.

- [ ] **Step 7: Check everything**

Run: `cargo xtask headers && cargo xtask ci`
Expected: headers, lint, check, test, doc, deny and audit all pass. `cargo deny` will pick up `tempfile`; if it reports a license not in `deny.toml`, add that license to the allowlist in a separate commit.

- [ ] **Step 8: Commit the docs**

Propose, wait for approval, then commit `README.md AGENTS.md`:

```
docs: describe collecting remotes

Document the add, list and remove commands, the multi-select keys, the
remotes file format, how fetchview.remotes resolves relative paths and
which URL forms are accepted. AGENTS.md notes where the parsers live, that
every URL goes through validate_url, and that Enter confirms in the
multi-select.
```

Then commit `CHANGELOG.md` separately:

```
docs: regenerate changelog

Run cargo xtask changelog to list the remote sources work.
```

---

## Self-Review

**Spec coverage**

| Spec section | Task |
|--------------|------|
| Configuration (`fetchview.remotes`, relative resolution, hierarchy, common dir) | 3 |
| Remotes file format, comments preserved, warnings per line | 2 |
| Label derivation | 1 |
| URL rules and identity | 1 |
| `add` options (`--file`, `--global`, `--all`, `--dry-run`), where `add` writes, registration | 7 |
| `add` flow steps 1 to 5 and the no-terminal failure | 7 |
| Sources: merges (streaming), maintainers, url | 4 (and 7 for wiring) |
| `list`, `remove` | 7 |
| Picker: real first, hide duplicates, row format, warnings in the status line | 5 |
| Multi-select keys and `(have)` rows | 6 |
| Code layout | File Structure |
| Testing, fuzz targets | each task, 8 |
| Deferred: lore, remembered refs | not built |

One deliberate difference from the spec: it says to take the URL after the last ` of `. The plan does that, and Task 4's `skips_subjects_without_a_url` pins the case where the last ` of ` is not a URL.

**Placeholder scan:** none; every code step has code.

**Type consistency:** `Entry`, `validate_url`, `Rejected`, `key`, `derive_label`, `merge_entries` (Task 1) are used with the same signatures in Tasks 2 to 7. `Scan`, `collect`, `dedupe` (Task 4) match their use in Task 7. `registry::{files, load, register, default_file, same_path, Scope}` (Task 3) match Task 7. `Select`, `Row`, `SelectFlow` and `tui::select_many` (Task 6) match Task 7. `git::Remote.tags` (Task 5) is used by `picker_remotes` and `App::new`.

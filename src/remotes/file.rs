// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The remotes file: one entry per line, `url<TAB>label<TAB>tags`.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::Path;

use super::entry::{Entry, Rejected, validate_url};
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
    let raw_label = fields.next().unwrap_or_default();
    let raw_tags = fields.next().unwrap_or_default();
    if raw_label.chars().any(char::is_control) || raw_tags.chars().any(char::is_control) {
        return Line::Bad("control character in label or tags");
    }
    let label = Some(raw_label.trim())
        .filter(|l| !l.is_empty())
        .map(str::to_owned);
    let tags = Some(raw_tags)
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
        if let Line::Entry(entry) = parse_line(line)
            && entry.key() == target_key
        {
            removed += 1;
            continue;
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
    // Write through a symlink instead of replacing it, so a list shared by
    // linking to it stays shared.
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let path = target.as_path();
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(io_err)?;
    }
    let name = path
        .file_name()
        .map_or_else(|| "remotes".into(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&tmp, text).map_err(io_err)?;
    if let Ok(meta) = fs::metadata(path) {
        fs::set_permissions(&tmp, meta.permissions()).map_err(io_err)?;
    }
    fs::rename(&tmp, path).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remotes::entry::key;

    const SAMPLE: &str = "\
# collected 2026-10-20
https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\tnetdev/net\tmaintainers

git://git.kernel.org/pub/scm/linux/kernel/git/davem/net-next
";

    #[test]
    fn parses_fields_and_skips_comments_and_blanks() {
        let parsed = parse(SAMPLE);
        assert_eq!(parsed.warnings, Vec::<String>::new());
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
        let same = [Entry::new(
            "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net",
            "maintainers",
        )];
        assert_eq!(merge(SAMPLE, &same), SAMPLE);

        let more = [Entry::new(
            "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net",
            "merge",
        )];
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
    fn control_characters_in_labels_and_tags_are_rejected() {
        let parsed = parse(
            "https://h/p/a\tla\x1b]0;PWNED\x07bel\tt\nhttps://h/p/b\tok\tt\x1b[31mred\nhttps://h/p/c\tfine\tt\n",
        );
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].label.as_deref(), Some("fine"));
        assert_eq!(parsed.warnings.len(), 2);
        assert!(parsed.warnings[0].starts_with("line 1:"));
    }

    #[test]
    fn write_atomic_writes_through_a_symlink_and_keeps_it() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("shared");
        std::fs::write(&real, "old\n").unwrap();
        let link = dir.path().join("mine");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "new\n").unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new\n");
    }

    #[test]
    fn write_atomic_keeps_the_existing_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("remotes");
        std::fs::write(&path, "old\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
        write_atomic(&path, "new\n").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o664);
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

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Which remotes files `git config` points at, and loading them.

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
    match (
        abs.parent().and_then(|d| d.canonicalize().ok()),
        abs.file_name(),
    ) {
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
                loaded
                    .warnings
                    .push(format!("{}: not found", path.display()));
            }
            Err(e) => loaded.warnings.push(format!("{}: {e}", path.display())),
        }
    }
    loaded
}

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
        assert_eq!(parse_pointers(text), Vec::<PathBuf>::new());
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

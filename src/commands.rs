// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The `add`, `list` and `remove` subcommands.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use crate::cli::{AddArgs, AddSource};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::git::{Cmd, redact_url};
use crate::remotes::entry::{Entry, key, validate_url};
use crate::remotes::file::render_line;
use crate::remotes::plan::{Plan, plan};
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
    let scope = if args.global {
        Scope::Global
    } else {
        Scope::Local
    };
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

fn row(entry: &Entry, have: bool) -> Row {
    Row {
        cells: row_cells(entry),
        name: entry.display_label(),
        url: entry.url.clone(),
        source: entry.tags.iter().cloned().collect::<Vec<_>>().join(","),
        have,
    }
}

/// The rows the picker shows: new entries first, selectable, then every entry
/// that is already collected (in this file or another), shown as `(have)`.
#[must_use]
pub fn picker_rows(plan: &Plan) -> Vec<Row> {
    plan.new
        .iter()
        .map(|e| row(e, false))
        .chain(
            plan.tag_updates
                .iter()
                .chain(&plan.present)
                .map(|e| row(e, true)),
        )
        .collect()
}

fn row_cells(entry: &Entry) -> Vec<String> {
    let tags = entry.tags.iter().cloned().collect::<Vec<_>>().join(",");
    vec![entry.display_label(), redact_url(&entry.url), tags]
}

/// What a scan found, sorted by where each repo already is, and where the
/// result would be written.
#[derive(Debug)]
pub struct Prepared {
    /// New, tag-update and already-present entries.
    pub plan: Plan,
    /// URLs that failed validation.
    pub rejected: usize,
    /// The remotes file to write.
    pub path: PathBuf,
    /// What to store in `fetchview.remotes` for it.
    pub stored: String,
    /// Which config level to register it in.
    pub scope: Scope,
    /// Problems with the registered files, for the caller to show.
    pub warnings: Vec<String>,
}

/// Run a source's scan and sort the results against the registered files.
///
/// # Errors
/// The source cannot be read, or the registered files cannot be listed.
pub fn prepare(source: &AddSource, target: (PathBuf, String, Scope)) -> Result<Prepared> {
    let scanned = scan(source)?;
    let (path, stored, scope) = target;
    let loaded = registry::load(&registry::files()?);
    let (in_target, elsewhere): (Vec<_>, Vec<_>) = loaded
        .files
        .iter()
        .partition(|f| registry::same_path(&f.path, &path));
    let in_target: Vec<Entry> = in_target
        .into_iter()
        .flat_map(|f| f.entries.clone())
        .collect();
    let elsewhere: Vec<Entry> = elsewhere
        .into_iter()
        .flat_map(|f| f.entries.clone())
        .collect();
    Ok(Prepared {
        plan: plan(scanned.entries, &in_target, &elsewhere),
        rejected: scanned.rejected,
        path,
        stored,
        scope,
        warnings: loaded.warnings,
    })
}

/// The default target: `.git/fetchview.remotes`, registered locally.
///
/// # Errors
/// The git directory cannot be found.
pub fn default_target() -> Result<(PathBuf, String, Scope)> {
    Ok((
        registry::default_file()?,
        registry::DEFAULT_FILE.to_owned(),
        Scope::Local,
    ))
}

/// Write the chosen new entries (indexes into `prepared.plan.new`) and merge
/// the tags of entries already in the file. Returns the entries that were
/// added.
///
/// # Errors
/// The file or the config cannot be written.
pub fn write_chosen(prepared: &Prepared, chosen: &[usize]) -> Result<Vec<Entry>> {
    let added: Vec<Entry> = chosen
        .iter()
        .filter_map(|&i| prepared.plan.new.get(i).cloned())
        .collect();
    let mut write = added.clone();
    write.extend(prepared.plan.tag_updates.iter().cloned());
    if !write.is_empty() {
        registry::write_entries(&prepared.path, &prepared.stored, prepared.scope, &write)?;
    }
    Ok(added)
}

/// Collect remotes from a source into a remotes file.
///
/// # Errors
/// The source cannot be read, there is no terminal and `--all` was not
/// given, or the file or config cannot be written.
pub fn add(args: &AddArgs) -> Result<()> {
    let theme = Config::load(None, None)?.theme;
    let prepared = prepare(&args.source, target(args)?)?;
    for warning in &prepared.warnings {
        eprintln!("warning: {warning}");
    }
    let plan = &prepared.plan;

    // The picker is for choosing among what a search found. URLs named by hand
    // are already the choice, and with nothing new there is nothing to choose.
    let named_by_hand = matches!(args.source, AddSource::Url { .. });
    let chosen: Vec<usize> = if args.all || named_by_hand || plan.new.is_empty() {
        (0..plan.new.len()).collect()
    } else {
        if !(io::stdin().is_terminal() && io::stdout().is_terminal()) {
            return Err(Error::NotATerminal);
        }
        match tui::select_many(picker_rows(plan), theme)? {
            None => return Ok(()),
            Some(picked) => picked.into_iter().filter(|&i| i < plan.new.len()).collect(),
        }
    };

    let (added, updated) = if args.dry_run {
        ("would add", "would update tags on")
    } else {
        ("added", "tags updated")
    };
    let summary = format!(
        "{added} {}, {updated} {}, already present {}, rejected {}",
        chosen.len(),
        plan.tag_updates.len(),
        plan.present.len(),
        prepared.rejected
    );

    if args.dry_run {
        let write = chosen
            .iter()
            .map(|&i| plan.new[i].clone())
            .chain(plan.tag_updates.iter().cloned());
        let lines = std::iter::once(format!("would write to {}:", prepared.path.display()))
            .chain(write.map(|e| render_line(&e)));
        print_lines(lines)?;
        eprintln!("{summary}");
        return Ok(());
    }
    write_chosen(&prepared, &chosen)?;
    eprintln!("{summary}");
    Ok(())
}

/// Print every collected remote: `label  url  tags  file`, tab-separated.
///
/// # Errors
/// Git cannot be run.
pub fn list() -> Result<()> {
    let loaded = registry::load_registered();
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
    print_lines(loaded.files.iter().flat_map(|file| {
        file.entries.iter().map(move |entry| {
            let tags = entry.tags.iter().cloned().collect::<Vec<_>>().join(",");
            format!(
                "{}\t{}\t{}\t{}",
                entry.display_label(),
                entry.url,
                tags,
                file.path.display()
            )
        })
    }))
}

/// Print lines to stdout. A reader that goes away (`... | head`) is not an
/// error, so stop quietly instead of panicking like `println!` does.
fn print_lines(lines: impl IntoIterator<Item = String>) -> Result<()> {
    let mut out = io::stdout().lock();
    for line in lines {
        match writeln!(out, "{line}") {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => break,
            Err(e) => return Err(e.into()),
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
            if wanted_key.as_deref() == Some(entry.key().as_str())
                || entry.display_label() == target
            {
                matches.insert(
                    entry.key(),
                    format!("{}  {}", entry.display_label(), entry.url),
                );
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
    for path in registry::remove_entry(&k)? {
        eprintln!("removed from {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(url: &str) -> Entry {
        Entry::new(url, "maintainers")
    }

    #[test]
    fn rows_list_new_entries_first_then_everything_already_collected() {
        let plan = Plan {
            new: vec![entry("https://h/p/new")],
            tag_updates: vec![entry("https://h/p/mine")],
            present: vec![entry("https://h/p/other")],
        };
        let rows = picker_rows(&plan);
        assert_eq!(
            rows.iter().map(|r| r.have).collect::<Vec<_>>(),
            [false, true, true]
        );
        assert!(rows[0].cells[0].starts_with("p/new"));
        assert!(rows[1].cells[0].starts_with("p/mine"));
        assert!(rows[2].cells[0].starts_with("p/other"));
    }
}

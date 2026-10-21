// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

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
        label: row_label(entry),
        name: entry.display_label(),
        url: entry.url.clone(),
        have,
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
    let theme = Config::load(None, None)?.theme;
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
    let in_target: Vec<Entry> = in_target
        .into_iter()
        .flat_map(|f| f.entries.clone())
        .collect();
    let elsewhere: Vec<Entry> = elsewhere
        .into_iter()
        .flat_map(|f| f.entries.clone())
        .collect();
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
            .map(|e| row(e, false))
            .chain(plan.present.iter().map(|e| row(e, true)))
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
    let loaded = registry::load_registered();
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
    for file in &loaded.files {
        let (text, removed) = remove_lines(&file.text, &k);
        if removed > 0 {
            write_atomic(&file.path, &text)?;
            eprintln!("removed from {}", file.path.display());
        }
    }
    Ok(())
}

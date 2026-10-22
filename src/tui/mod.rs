// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The ratatui front end.

pub mod app;
pub mod keys;
pub mod list;
pub mod select;
pub mod theme;
pub mod ui;

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event};

use crate::cli::AddSource;
use crate::commands;
use crate::error::{Error, Result};
use crate::git::{self, Cancel, Remote};
use crate::hide::{self, Hidden, Rule};
use crate::remotes::entry::{Entry, key as repo_key};
use crate::remotes::registry;
use app::{App, Flow, Load, Loaded, Selection};
use keys::KeyParser;
use select::{Row, Select, SelectFlow};
use theme::{Theme, ThemeName};

/// Which refs to leave out of the refs list.
#[derive(Debug, Clone, Default)]
pub struct HideRefs {
    /// Repositories whose refs are hidden.
    pub rules: Vec<Rule>,
    /// Refetch their lists even if the cache is recent.
    pub refresh: bool,
    /// Ref patterns that are always hidden, such as `refs/pull/*/merge`.
    pub patterns: Vec<String>,
}

/// Run the picker. Returns `None` if the user quit.
///
/// # Errors
/// Terminal I/O failures.
pub fn pick(
    remotes: Vec<Remote>,
    theme: ThemeName,
    notice: Option<String>,
    hide: &HideRefs,
) -> Result<Option<Selection>> {
    let theme = Theme::from(theme);
    let mut app = App::new(remotes).with_notice(notice);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &theme, hide);
    ratatui::restore();
    result
}

/// Rows of list on screen: the terminal height minus the search box, the
/// border, the column header and the status line.
fn list_rows(terminal: &ratatui::DefaultTerminal) -> Result<usize> {
    Ok(usize::from(terminal.size()?.height.saturating_sub(5)).max(1))
}

/// How often the spinner moves while refs are loading.
const SPIN: Duration = Duration::from_millis(80);

/// List the refs of `remote` and drop the ones other trees carry. The lists to
/// hide are fetched on the first call and kept in `hidden` after that.
fn list_refs(
    remote: &Remote,
    hide: &HideRefs,
    hidden: &mut Option<Hidden>,
    cancel: &Cancel,
) -> Result<Loaded> {
    let refs = git::remote_refs(&remote.url, cancel)?;
    if hidden.is_none() {
        let loaded = Hidden::load_real(&hide.rules, hide.refresh, cancel);
        if cancel.is_cancelled() {
            // Its fetches were cut short, so it would hide nothing. Do not keep it.
            return Err(Error::Cancelled {
                command: format!("hidden refs for {}", remote.name),
            });
        }
        *hidden = Some(loaded);
    }
    let hidden = hidden.get_or_insert_with(Hidden::default);
    let (refs, mut gone) = hidden.apply(&remote.url, refs);
    let (refs, noise) = hide::hide_matching(&hide.patterns, refs);
    gone.extend(noise);
    Ok(Loaded {
        refs,
        hidden: gone,
        note: hidden.note.take(),
    })
}

/// What the status line calls a scan.
fn scan_name(source: &AddSource) -> &'static str {
    match source {
        AddSource::Merges { .. } => "merges",
        AddSource::Maintainers { .. } => "MAINTAINERS",
        AddSource::Url { .. } => "url file",
    }
}

/// Scan a source, let the user choose among the new finds in the multi-select,
/// and write the choice to the default file. Returns the entries added, which
/// is none if there was nothing new or the user backed out.
fn scan_and_choose(
    terminal: &mut ratatui::DefaultTerminal,
    theme: &Theme,
    source: &AddSource,
) -> Result<Vec<Entry>> {
    let prepared = commands::prepare(source, commands::default_target()?)?;
    if prepared.plan.new.is_empty() {
        return Ok(Vec::new());
    }
    let mut state = Select::new(commands::picker_rows(&prepared.plan));
    match run_select(terminal, &mut state, theme)? {
        Some(chosen) => commands::write_chosen(&prepared, &chosen),
        None => Ok(Vec::new()),
    }
}

/// Lists refs on its own thread, so the screen keeps redrawing and a slow
/// remote can be given up on.
struct Worker {
    requests: Sender<(Load, Cancel)>,
    answers: Receiver<(u64, Result<Loaded>)>,
    running: Option<Cancel>,
}

impl Worker {
    fn spawn(hide: HideRefs) -> Self {
        let (requests, wanted) = mpsc::channel::<(Load, Cancel)>();
        let (reply, answers) = mpsc::channel();
        thread::spawn(move || {
            let mut hidden: Option<Hidden> = None;
            while let Ok((load, cancel)) = wanted.recv() {
                let result = list_refs(&load.remote, &hide, &mut hidden, &cancel);
                if reply.send((load.id, result)).is_err() {
                    break;
                }
            }
        });
        Self {
            requests,
            answers,
            running: None,
        }
    }

    fn start(&mut self, load: Load) {
        let cancel = Cancel::default();
        self.running = Some(cancel.clone());
        // The thread only ends when this struct goes away.
        let _ = self.requests.send((load, cancel));
    }

    fn cancel(&mut self) {
        if let Some(cancel) = self.running.take() {
            cancel.cancel();
        }
    }
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    theme: &Theme,
    hide: &HideRefs,
) -> Result<Option<Selection>> {
    let mut worker = Worker::spawn(hide.clone());
    let mut parser = KeyParser::default();
    loop {
        while let Ok((id, result)) = worker.answers.try_recv() {
            app.finish_load(id, result);
        }
        app.set_viewport(list_rows(terminal)?);
        terminal.draw(|frame| ui::draw(frame, app, theme, &parser.pending()))?;
        // Keys wait forever, except while loading, when the spinner and the
        // answer need a look every so often.
        if app.loading().is_some() && !event::poll(SPIN)? {
            app.tick();
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(command) = parser.feed(key, app.mode()) else {
            continue;
        };
        match app.step(command) {
            Flow::Continue => {}
            Flow::Load(load) => worker.start(load),
            Flow::CancelLoad => worker.cancel(),
            Flow::Scan(source) => {
                app.begin_scan(scan_name(&source));
                terminal.draw(|frame| ui::draw(frame, app, theme, ""))?;
                let result = scan_and_choose(terminal, theme, &source);
                app.finish_scan(result);
            }
            Flow::Promote { name, url } => {
                let result = git::Cmd::remote_add(&name, &url).output().map(|_| ());
                app.finish_promote(&url, &name, result);
            }
            Flow::Add(url) => {
                let entry = Entry::new(url, "manual");
                let result = registry::add_to_default(&entry).map(|()| entry);
                app.finish_add(result);
            }
            Flow::Remove(remote) => {
                let result = registry::remove_entry(&repo_key(&remote.url)).map(|_| ());
                app.finish_remove(&remote.url, result);
            }
            Flow::Quit => {
                worker.cancel();
                return Ok(None);
            }
            Flow::Done(selection) => return Ok(Some(selection)),
        }
    }
}

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
    let mut parser = KeyParser::default();
    loop {
        state.set_viewport(list_rows(terminal)?);
        terminal.draw(|frame| ui::draw_select(frame, state, theme, &parser.pending()))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(command) = parser.feed(key, state.mode()) else {
            continue;
        };
        match state.update_with(command) {
            SelectFlow::Continue => {}
            SelectFlow::Quit => return Ok(None),
            SelectFlow::Done(chosen) => return Ok(Some(chosen)),
        }
    }
}

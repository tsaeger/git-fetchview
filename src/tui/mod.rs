// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The ratatui front end.

pub mod app;
pub mod keys;
pub mod list;
pub mod select;
pub mod theme;
pub mod ui;

use ratatui::crossterm::event::{self, Event};

use crate::error::Result;
use crate::git::{self, Remote};
use app::{App, Flow, Selection};
use select::{Row, Select, SelectFlow};
use theme::{Theme, ThemeName};

/// Run the picker. Returns `None` if the user quit.
///
/// # Errors
/// Terminal I/O failures.
pub fn pick(
    remotes: Vec<Remote>,
    theme: ThemeName,
    notice: Option<String>,
) -> Result<Option<Selection>> {
    let theme = Theme::from(theme);
    let mut app = App::new(remotes).with_notice(notice);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &theme);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    theme: &Theme,
) -> Result<Option<Selection>> {
    let mut load = |remote: &Remote| git::remote_refs(&remote.url);
    loop {
        terminal.draw(|frame| ui::draw(frame, app, theme))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(action) = keys::map(key, app.typing()) else {
            continue;
        };
        match app.update(action, &mut load) {
            Flow::Continue => {}
            Flow::Quit => return Ok(None),
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

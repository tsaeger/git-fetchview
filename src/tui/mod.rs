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
use keys::KeyParser;
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

/// Rows of list on screen: the terminal height minus the border and status line.
fn list_rows(terminal: &ratatui::DefaultTerminal) -> Result<usize> {
    Ok(usize::from(terminal.size()?.height.saturating_sub(3)).max(1))
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    theme: &Theme,
) -> Result<Option<Selection>> {
    let mut load = |remote: &Remote| git::remote_refs(&remote.url);
    let mut parser = KeyParser::default();
    loop {
        app.set_viewport(list_rows(terminal)?);
        terminal.draw(|frame| ui::draw(frame, app, theme, &parser.pending()))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(command) = parser.feed(key, app.typing()) else {
            continue;
        };
        match app.update_with(command, &mut load) {
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
    let mut parser = KeyParser::default();
    loop {
        state.set_viewport(list_rows(terminal)?);
        terminal.draw(|frame| ui::draw_select(frame, state, theme, &parser.pending()))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let Some(command) = parser.feed(key, state.typing()) else {
            continue;
        };
        match state.update_with(command) {
            SelectFlow::Continue => {}
            SelectFlow::Quit => return Ok(None),
            SelectFlow::Done(chosen) => return Ok(Some(chosen)),
        }
    }
}

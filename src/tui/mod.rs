// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The ratatui front end.

pub mod app;
pub mod keys;
pub mod list;
pub mod theme;
pub mod ui;

use ratatui::crossterm::event::{self, Event};

use crate::error::Result;
use crate::git::{self, Remote};
use app::{App, Flow, Selection};
use theme::{Theme, ThemeName};

/// Run the picker. Returns `None` if the user quit.
///
/// # Errors
/// Terminal I/O failures.
pub fn pick(remotes: Vec<Remote>, theme: ThemeName) -> Result<Option<Selection>> {
    let theme = Theme::from(theme);
    let mut app = App::new(remotes);
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
    let mut load = |remote: &Remote| git::remote_refs(&remote.name);
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

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List as ListWidget, ListItem, ListState, Paragraph};

use crate::tui::app::{App, Level};
use crate::tui::theme::Theme;

const HINTS: &str = "j/k move  l/enter open  h/esc back  / search  c clear  q quit";

/// Draw the picker.
pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme) {
    let base = Style::new().fg(theme.fg).bg(theme.bg);
    frame.render_widget(Block::new().style(base), frame.area());
    let [body, status] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let title = match (app.level(), app.chosen_remote()) {
        (Level::Refs, Some(remote)) => format!(" refs of {remote} "),
        _ => " remotes ".to_owned(),
    };
    let block = Block::bordered()
        .title(Span::styled(
            title,
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::new().fg(theme.dim))
        .style(base);

    let items: Vec<ListItem<'_>> = app
        .list()
        .visible_labels()
        .into_iter()
        .map(ListItem::new)
        .collect();
    let list = ListWidget::new(items)
        .block(block)
        .highlight_style(
            Style::new()
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(app.list().cursor()));
    frame.render_stateful_widget(list, body, &mut state);

    let line = if let Some(err) = app.error() {
        Line::from(Span::styled(err.to_owned(), Style::new().fg(theme.error)))
    } else if app.typing() {
        Line::from(vec![
            Span::styled("/", Style::new().fg(theme.accent)),
            Span::raw(app.list().filter().to_owned()),
            Span::styled("█", Style::new().fg(theme.accent)),
        ])
    } else if let Some(notice) = app.notice() {
        Line::from(Span::styled(
            notice.to_owned(),
            Style::new().fg(theme.accent),
        ))
    } else if app.list().filter().is_empty() {
        Line::from(Span::styled(HINTS, Style::new().fg(theme.dim)))
    } else {
        Line::from(vec![
            Span::styled(
                format!("/{}", app.list().filter()),
                Style::new().fg(theme.accent),
            ),
            Span::styled("  c clear", Style::new().fg(theme.dim)),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(base), status);
}

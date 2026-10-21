// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

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
        .highlight_style(
            Style::new()
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        )
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
        Line::from(Span::styled(
            notice.to_owned(),
            Style::new().fg(theme.accent),
        ))
    } else if app.list().filter().is_empty() {
        hint_line(HINTS, theme)
    } else {
        Line::from(vec![
            Span::styled(
                format!("/{}", app.list().filter()),
                Style::new().fg(theme.accent),
            ),
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
    render_list(
        frame,
        body,
        " add remotes ".to_owned(),
        items,
        select.list().cursor(),
        theme,
    );

    let line = if select.typing() {
        typing_line(select.list().filter(), theme)
    } else {
        let counts = format!(
            "{} checked of {}  ",
            select.checked_count(),
            select.selectable_count()
        );
        Line::from(vec![
            Span::styled(counts, Style::new().fg(theme.accent)),
            Span::styled(SELECT_HINTS, Style::new().fg(theme.dim)),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}

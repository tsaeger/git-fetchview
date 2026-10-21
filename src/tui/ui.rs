// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List as ListWidget, ListItem, ListState, Paragraph};

use crate::tui::app::{App, Level};
use crate::tui::list::{List, SortOrder};
use crate::tui::select::Select;
use crate::tui::theme::Theme;

const HINTS: &str = "j/k move  l/enter open  h/esc back  / search  c clear  s sort  q quit";
const SORT_HINTS: &str = " h/l column  r reverse  s/esc done";
const SELECT_HINTS: &str = "space toggle  a all  enter write  / search  c clear  s sort  q quit";

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

/// `sort: label  ` for the status line, or nothing in the default order.
fn sort_note(order: SortOrder, reversed: bool) -> Option<String> {
    if order == SortOrder::Default && !reversed {
        return None;
    }
    let arrow = if reversed { " \u{2193}" } else { "" };
    Some(format!("sort: {}{arrow}  ", order.name()))
}

/// `5g  ` for keys typed but not finished, or nothing.
fn pending_note(pending: &str) -> Option<String> {
    (!pending.is_empty()).then(|| format!("{pending}  "))
}

fn pending_span<'a>(pending: &str, theme: &Theme) -> Option<Span<'a>> {
    pending_note(pending).map(|note| {
        Span::styled(
            note,
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        )
    })
}

/// The columns as `(text, active)`, with the direction on the active one.
fn sort_bar_parts(columns: &[SortOrder], active: SortOrder, reversed: bool) -> Vec<(String, bool)> {
    columns
        .iter()
        .map(|&c| {
            if c == active {
                let arrow = if reversed { " \u{2193}" } else { "" };
                (format!("[{}{arrow}]", c.name()), true)
            } else {
                (c.name().to_owned(), false)
            }
        })
        .collect()
}

fn sort_bar<'a>(list: &List, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled("sort ", Style::new().fg(theme.accent))];
    for (text, active) in sort_bar_parts(list.columns(), list.sort_order(), list.reversed()) {
        let style = if active {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.dim)
        };
        spans.push(Span::styled(format!("{text} "), style));
    }
    spans.push(Span::styled(SORT_HINTS, Style::new().fg(theme.dim)));
    spans
}

fn sort_span<'a>(order: SortOrder, reversed: bool, theme: &Theme) -> Option<Span<'a>> {
    sort_note(order, reversed).map(|note| Span::styled(note, Style::new().fg(theme.accent)))
}

fn typing_line<'a>(filter: &str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled("/", Style::new().fg(theme.accent)),
        Span::raw(filter.to_owned()),
        Span::styled("█", Style::new().fg(theme.accent)),
    ])
}

/// Draw the picker.
pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme, pending: &str) {
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
    } else if app.sorting() {
        Line::from(sort_bar(app.list(), theme))
    } else if let Some(notice) = app.notice() {
        Line::from(Span::styled(
            notice.to_owned(),
            Style::new().fg(theme.accent),
        ))
    } else {
        let mut spans: Vec<Span<'_>> = pending_span(pending, theme).into_iter().collect();
        spans.extend(sort_span(
            app.list().sort_order(),
            app.list().reversed(),
            theme,
        ));
        if app.list().filter().is_empty() {
            spans.push(Span::styled(HINTS, Style::new().fg(theme.dim)));
        } else {
            spans.push(Span::styled(
                format!("/{}", app.list().filter()),
                Style::new().fg(theme.accent),
            ));
            spans.push(Span::styled("  c clear", Style::new().fg(theme.dim)));
        }
        Line::from(spans)
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}

/// Draw the multi-select.
pub fn draw_select(frame: &mut Frame<'_>, select: &Select, theme: &Theme, pending: &str) {
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
    } else if select.sorting() {
        Line::from(sort_bar(select.list(), theme))
    } else {
        let counts = format!(
            "{} checked of {}  ",
            select.checked_count(),
            select.selectable_count()
        );
        let mut spans: Vec<Span<'_>> = pending_span(pending, theme).into_iter().collect();
        spans.push(Span::styled(counts, Style::new().fg(theme.accent)));
        spans.extend(sort_span(
            select.list().sort_order(),
            select.list().reversed(),
            theme,
        ));
        spans.push(Span::styled(SELECT_HINTS, Style::new().fg(theme.dim)));
        Line::from(spans)
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sort_bar_marks_the_active_column_and_the_direction() {
        let columns = [
            SortOrder::Default,
            SortOrder::Label,
            SortOrder::Url,
            SortOrder::Tags,
        ];
        let parts = sort_bar_parts(&columns, SortOrder::Label, true);
        assert_eq!(
            parts,
            [
                ("default".to_owned(), false),
                ("[label \u{2193}]".to_owned(), true),
                ("url".to_owned(), false),
                ("tags".to_owned(), false),
            ]
        );
    }

    #[test]
    fn pending_input_shows_in_the_status_line() {
        assert_eq!(pending_note(""), None);
        assert_eq!(pending_note("5g"), Some("5g  ".to_owned()));
    }

    #[test]
    fn the_status_line_names_a_non_default_or_reversed_sort() {
        assert_eq!(sort_note(SortOrder::Default, false), None);
        assert_eq!(
            sort_note(SortOrder::Label, false),
            Some("sort: label  ".to_owned())
        );
        assert_eq!(
            sort_note(SortOrder::Default, true),
            Some("sort: default \u{2193}  ".to_owned())
        );
        assert_eq!(
            sort_note(SortOrder::Url, true),
            Some("sort: url \u{2193}  ".to_owned())
        );
    }
}

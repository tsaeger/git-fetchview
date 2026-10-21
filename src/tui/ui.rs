// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};

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

/// A column header and the sort order that picks it, if it can be sorted.
type Header = (&'static str, Option<SortOrder>);

const REMOTE_HEADERS: [Header; 3] = [
    ("name", Some(SortOrder::Label)),
    ("url", Some(SortOrder::Url)),
    ("tags", Some(SortOrder::Tags)),
];
const REF_HEADERS: [Header; 2] = [("name", Some(SortOrder::Label)), ("oid", None)];
const SELECT_HEADERS: [Header; 4] = [
    ("", None),
    ("name", Some(SortOrder::Label)),
    ("url", Some(SortOrder::Url)),
    ("tags", Some(SortOrder::Tags)),
];

/// Columns before the last one stop growing here; the last takes what is left.
const COLUMN_CAP: usize = 60;
const COLUMN_GAP: u16 = 2;

/// Width of each column: the widest of its header and cells, capped.
fn column_widths(headers: &[Header], rows: &[Vec<String>]) -> Vec<usize> {
    let width = |s: &str| Line::from(s).width();
    headers
        .iter()
        .enumerate()
        .map(|(i, (title, _))| {
            rows.iter()
                .filter_map(|r| r.get(i))
                .map(|c| width(c))
                .fold(width(title) + 2, usize::max)
                .min(COLUMN_CAP)
        })
        .collect()
}

/// A header title, with a direction arrow when it is the sorted column.
fn header_text(
    title: &str,
    column: Option<SortOrder>,
    active: SortOrder,
    reversed: bool,
) -> String {
    match column {
        Some(c) if c == active && reversed => format!("{title} \u{2193}"),
        _ => title.to_owned(),
    }
}

/// One row to draw: its cells and the style they get.
struct TableRow {
    cells: Vec<String>,
    style: Style,
}

fn render_table(
    frame: &mut Frame<'_>,
    area: Rect,
    title: String,
    headers: &[Header],
    list: &List,
    rows: Vec<TableRow>,
    theme: &Theme,
) {
    let block = Block::bordered()
        .title(Span::styled(
            title,
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::new().fg(theme.dim))
        .style(base(theme));

    let cells: Vec<Vec<String>> = rows.iter().map(|r| r.cells.clone()).collect();
    let widths = column_widths(headers, &cells);
    let constraints = widths.iter().enumerate().map(|(i, &w)| {
        if i + 1 == widths.len() {
            Constraint::Fill(1)
        } else {
            Constraint::Length(u16::try_from(w).unwrap_or(u16::MAX))
        }
    });

    let header = Row::new(headers.iter().map(|&(title, column)| {
        let sorted = column == Some(list.sort_order());
        let style = if sorted {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.dim)
        };
        Cell::from(header_text(
            title,
            column,
            list.sort_order(),
            list.reversed(),
        ))
        .style(style)
    }));
    let body = rows.into_iter().map(|r| Row::new(r.cells).style(r.style));

    let table = Table::new(body, constraints)
        .header(header)
        .column_spacing(COLUMN_GAP)
        .block(block)
        .row_highlight_style(
            Style::new()
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = TableState::default().with_selected(Some(list.cursor()));
    frame.render_stateful_widget(table, area, &mut state);
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
    let (headers, rows): (&[Header], Vec<TableRow>) = {
        let rows = app
            .list()
            .visible_cells()
            .into_iter()
            .map(|(_, cells)| TableRow {
                cells: cells.to_vec(),
                style: Style::new(),
            })
            .collect();
        match app.level() {
            Level::Refs => (&REF_HEADERS, rows),
            Level::Remotes => (&REMOTE_HEADERS, rows),
        }
    };
    render_table(frame, body, title, headers, app.list(), rows, theme);

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

    let rows = select
        .list()
        .visible_cells()
        .into_iter()
        .map(|(i, cells)| {
            let (mark, style) = if select.is_have(i) {
                ("(have)", Style::new().fg(theme.dim))
            } else if select.is_checked(i) {
                ("[x]", Style::new())
            } else {
                ("[ ]", Style::new())
            };
            TableRow {
                cells: std::iter::once(mark.to_owned())
                    .chain(cells.iter().cloned())
                    .collect(),
                style,
            }
        })
        .collect();
    render_table(
        frame,
        body,
        " add remotes ".to_owned(),
        &SELECT_HEADERS,
        select.list(),
        rows,
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
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::git::Remote;
    use crate::tui::keys::Action;
    use crate::tui::theme::ThemeName;

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

    /// Draw into a test terminal and return each line of the screen.
    fn screen(width: u16, height: u16, draw: impl FnOnce(&mut Frame<'_>)) -> Vec<String> {
        let mut terminal = ratatui::Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(draw).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    fn remote(name: &str, url: &str, tags: &[&str]) -> Remote {
        Remote {
            name: name.into(),
            url: url.into(),
            tags: tags.iter().map(|t| (*t).into()).collect(),
        }
    }

    #[test]
    fn remote_columns_line_up_whatever_the_name_length() {
        let app = App::new(vec![
            remote("origin", "https://h/o", &[]),
            remote("netdev/net-next", "https://h/n", &["merge"]),
        ]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw(f, &app, &theme, ""));
        let url_col = |needle: &str| {
            let line = lines.iter().find(|l| l.contains(needle)).unwrap();
            line.find("https://").unwrap()
        };
        assert_eq!(url_col("origin"), url_col("netdev/net-next"));
        // The tags column lines up too, and the header names the columns.
        let tags_at = lines
            .iter()
            .find(|l| l.contains("merge"))
            .unwrap()
            .find("merge");
        assert_eq!(tags_at, lines[1].find("tags"));
        assert!(lines[1].contains("name") && lines[1].contains("url"));
    }

    #[test]
    fn the_header_marks_the_sorted_column_and_direction() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let mut no_refs = |_: &Remote| Ok(Vec::new());
        for action in [
            Action::Sort,
            Action::ColumnNext,
            Action::ColumnNext,
            Action::Reverse,
        ] {
            app.update(action, &mut no_refs);
        }
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[1].contains("url \u{2193}"), "{:?}", lines[1]);
    }

    #[test]
    fn a_long_last_column_is_clipped_not_wrapped() {
        let app = App::new(vec![remote(
            "a",
            "https://h/a",
            &["x".repeat(100).as_str()],
        )]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(40, 6, |f| draw(f, &app, &theme, ""));
        assert_eq!(lines.iter().filter(|l| l.contains('x')).count(), 1);
    }
}

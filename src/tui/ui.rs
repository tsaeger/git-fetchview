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

const HINTS: &str = "j/k move  l/enter open  h/esc back  / i search  c clear  s sort  q quit";
/// The top level has nowhere to go back to, and offers removing.
const REMOTE_HINTS: &str =
    "j/k move  l/enter open  / i search  c clear  s sort  n new  d remove  q quit";
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SORT_HINTS: &str = " h/l column  r reverse  s/esc done";
const SELECT_HINTS: &str = "space toggle  a all  enter write  / i search  c clear  s sort  q quit";

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

/// The search box: always on the top line, so the filter in force is never
/// something to hunt for.
fn search_line<'a>(list: &List, typing: bool, theme: &Theme) -> Line<'a> {
    let label = Span::styled("search: ", Style::new().fg(theme.accent));
    if typing {
        return Line::from(vec![
            label,
            Span::raw(list.filter().to_owned()),
            Span::styled("█", Style::new().fg(theme.accent)),
        ]);
    }
    if list.filter().is_empty() {
        return Line::from(vec![
            label,
            Span::styled("press / or i to search", Style::new().fg(theme.dim)),
        ]);
    }
    Line::from(vec![
        label,
        Span::raw(list.filter().to_owned()),
        Span::styled(
            format!("  {} of {}", list.visible().len(), list.total()),
            Style::new().fg(theme.dim),
        ),
    ])
}

fn typing_hints(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        "type to filter  enter/esc done",
        Style::new().fg(theme.dim),
    ))
}

/// Draw the picker.
pub fn draw(frame: &mut Frame<'_>, app: &App, theme: &Theme, pending: &str) {
    frame.render_widget(Block::new().style(base(theme)), frame.area());
    let [search, body, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(search_line(app.list(), app.typing(), theme)).style(base(theme)),
        search,
    );

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

    let line = if let Some(name) = app.loading() {
        let frame = SPINNER[app.spinner() % SPINNER.len()];
        Line::from(vec![
            Span::styled(
                format!("{frame} loading {name}…  "),
                Style::new().fg(theme.accent),
            ),
            Span::styled("esc cancel", Style::new().fg(theme.dim)),
        ])
    } else if let Some((label, text)) = app.prompt() {
        let mut spans = vec![
            Span::styled(format!("{label}: "), Style::new().fg(theme.accent)),
            Span::styled(format!("{text}\u{2588}"), Style::new().fg(theme.fg)),
        ];
        match app.error() {
            Some(err) => spans.push(Span::styled(
                format!("  {err}"),
                Style::new().fg(theme.error),
            )),
            None => spans.push(Span::styled(
                "  enter add  esc cancel",
                Style::new().fg(theme.dim),
            )),
        }
        Line::from(spans)
    } else if let Some(name) = app.confirming() {
        Line::from(Span::styled(
            format!("remove {name}? y/n"),
            Style::new().fg(theme.accent),
        ))
    } else if let Some(err) = app.error() {
        Line::from(Span::styled(err.to_owned(), Style::new().fg(theme.error)))
    } else if app.typing() {
        typing_hints(theme)
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
        if app.level() == Level::Refs && app.hidden() > 0 {
            spans.push(Span::styled(
                format!("{} hidden  ", app.hidden()),
                Style::new().fg(theme.accent),
            ));
        }
        let hints = if app.level() == Level::Remotes {
            REMOTE_HINTS
        } else {
            HINTS
        };
        spans.push(Span::styled(hints, Style::new().fg(theme.dim)));
        Line::from(spans)
    };
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
}

/// Draw the multi-select.
pub fn draw_select(frame: &mut Frame<'_>, select: &Select, theme: &Theme, pending: &str) {
    frame.render_widget(Block::new().style(base(theme)), frame.area());
    let [search, body, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(search_line(select.list(), select.typing(), theme)).style(base(theme)),
        search,
    );

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
        typing_hints(theme)
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
    use crate::git::{Remote, RemoteRef};
    use crate::tui::app::Loaded;
    use crate::tui::keys::Action;
    use crate::tui::select::Row;
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
        assert_eq!(tags_at, lines[2].find("tags"));
        assert!(lines[2].contains("name") && lines[2].contains("url"));
    }

    #[test]
    fn the_status_line_says_what_is_loading_and_how_to_stop_it() {
        let mut app = App::new(vec![remote("origin", "https://h/o", &[])]);
        app.step(crate::tui::keys::Command::from(Action::Enter));
        let theme = Theme::from(ThemeName::default());
        let first = screen(60, 6, |f| draw(f, &app, &theme, ""));
        let status = &first[5];
        assert!(status.contains("loading origin"), "{status:?}");
        assert!(status.contains("esc cancel"), "{status:?}");
        // The spinner moves.
        app.tick();
        let second = screen(60, 6, |f| draw(f, &app, &theme, ""));
        assert_ne!(first[5], second[5]);
    }

    #[test]
    fn the_prompt_shows_its_text_a_cursor_and_any_problem() {
        let mut app = App::new(vec![remote("origin", "https://h/o", &[])]);
        app.step(crate::tui::keys::Command::from(Action::New));
        for c in "ext::x".chars() {
            app.step(crate::tui::keys::Command::from(Action::Char(c)));
        }
        let theme = Theme::from(ThemeName::default());
        let lines = screen(70, 6, |f| draw(f, &app, &theme, ""));
        assert!(
            lines[5].contains("add url: ext::x\u{2588}"),
            "{:?}",
            lines[5]
        );
        assert!(lines[5].contains("esc"), "{:?}", lines[5]);
        app.step(crate::tui::keys::Command::from(Action::Enter));
        let lines = screen(70, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[5].contains("add url: ext::x"), "{:?}", lines[5]);
        assert!(lines[5].contains("transport"), "{:?}", lines[5]);
    }

    #[test]
    fn the_status_line_asks_before_a_removal() {
        let mut app = App::new(vec![remote("netdev/net", "https://h/n", &["merge"])]);
        app.step(crate::tui::keys::Command::from(Action::Delete));
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 6, |f| draw(f, &app, &theme, ""));
        assert!(
            lines[5].contains("remove netdev/net? y/n"),
            "{:?}",
            lines[5]
        );
    }

    #[test]
    fn the_remote_hints_offer_d_and_fit_80_columns() {
        assert!(REMOTE_HINTS.contains("d remove"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("n new"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("/ i search"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.chars().count() <= 80, "{REMOTE_HINTS}");
    }

    #[test]
    fn a_password_in_a_remote_url_is_not_drawn() {
        let app = App::new(vec![remote("work", "https://dev:s3cret@h/o.git", &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 6, |f| draw(f, &app, &theme, ""));
        let text = lines.join("\n");
        assert!(!text.contains("s3cret"), "{text}");
        assert!(text.contains("https://dev@h/o.git"), "{text}");
    }

    #[test]
    fn the_header_marks_the_sorted_column_and_direction() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let mut no_refs = |_: &Remote| Ok(Loaded::default());
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
        assert!(lines[2].contains("url \u{2193}"), "{:?}", lines[2]);
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

    #[test]
    fn the_status_line_counts_hidden_refs() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let mut load = |_: &Remote| {
            Ok(Loaded {
                refs: vec![RemoteRef {
                    name: "refs/heads/dev".into(),
                    oid: "1234567890".into(),
                }],
                hidden: 412,
                note: None,
            })
        };
        app.update(Action::Enter, &mut load);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(90, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines.last().unwrap().contains("412 hidden"), "{lines:?}");
    }

    fn searched(typing: bool) -> App {
        let mut app = App::new(vec![
            remote("xray", "https://h/x", &[]),
            remote("beta", "https://h/b", &[]),
        ]);
        let mut no_refs = |_: &Remote| Ok(Loaded::default());
        app.update(Action::Search, &mut no_refs);
        app.update(Action::Char('x'), &mut no_refs);
        if !typing {
            app.update(Action::Back, &mut no_refs);
        }
        app
    }

    #[test]
    fn the_key_hints_name_both_search_keys_and_fit_80_columns() {
        for hints in [HINTS, SELECT_HINTS] {
            assert!(hints.contains("/ i search"), "{hints}");
            assert!(hints.chars().count() <= 80, "{hints}");
        }
    }

    #[test]
    fn the_search_box_sits_at_the_top_with_a_hint_while_empty() {
        let app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw(f, &app, &theme, ""));
        assert!(
            lines[0].contains("search") && lines[0].contains("/ or i"),
            "{:?}",
            lines[0]
        );
    }

    #[test]
    fn the_search_box_shows_the_text_and_how_many_rows_match() {
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw(f, &searched(false), &theme, ""));
        assert!(lines[0].contains("search: x"), "{:?}", lines[0]);
        assert!(lines[0].contains("1 of 2"), "{:?}", lines[0]);
        assert!(
            !lines[0].contains('\u{2588}'),
            "no cursor outside edit mode"
        );
        assert!(lines.iter().skip(1).all(|l| !l.contains("beta")));
    }

    #[test]
    fn editing_shows_a_cursor_and_the_keys_to_leave() {
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw(f, &searched(true), &theme, ""));
        assert!(lines[0].contains("search: x\u{2588}"), "{:?}", lines[0]);
        assert!(lines.last().unwrap().contains("esc"), "{lines:?}");
    }

    #[test]
    fn the_select_picker_has_the_box_too() {
        let mut s = Select::new(vec![Row {
            cells: vec!["x".into()],
            name: "x".into(),
            url: "u".into(),
            tags: String::new(),
            have: false,
        }]);
        s.update(Action::Search);
        s.update(Action::Char('x'));
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw_select(f, &s, &theme, ""));
        assert!(lines[0].contains("search: x"), "{:?}", lines[0]);
    }
}

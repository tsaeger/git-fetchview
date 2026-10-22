// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};

use crate::tui::app::{App, Level};
use crate::tui::list::{Direction, List, SortColumn};
use crate::tui::select::Select;
use crate::tui::theme::Theme;

/// Short, because the hidden-refs count can come first on the same line.
const HINTS: &str = "\u{21b5} open  esc back  h/l col  s sort  / i search  q quit";
/// The top level has nowhere to go back to, and offers removing.
const REMOTE_HINTS: &str =
    "/ i search  h/l col  s sort  n new  o once  A scan  p promote  d remove  q quit";
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SELECT_HINTS: &str =
    "space toggle  J/K range  a all  \u{21b5} write  / i search  h/l col  s sort  q quit";

fn base(theme: &Theme) -> Style {
    Style::new().fg(theme.fg).bg(theme.bg)
}

/// A column header and the sort column it stands for, if it can be sorted.
type Header = (&'static str, Option<SortColumn>);

const REMOTE_HEADERS: [Header; 3] = [
    ("name", Some(SortColumn::Label)),
    ("url", Some(SortColumn::Url)),
    ("source", Some(SortColumn::Source)),
];
const REF_HEADERS: [Header; 2] = [("name", Some(SortColumn::Label)), ("oid", None)];
const SELECT_HEADERS: [Header; 4] = [
    ("", None),
    ("name", Some(SortColumn::Label)),
    ("url", Some(SortColumn::Url)),
    ("source", Some(SortColumn::Source)),
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

/// A header title, with an arrow when it is the sorted column.
fn header_text(
    title: &str,
    column: Option<SortColumn>,
    sorted: Option<(SortColumn, Direction)>,
) -> String {
    match (column, sorted) {
        (Some(c), Some((s, Direction::Asc))) if c == s => format!("{title} \u{2191}"),
        (Some(c), Some((s, Direction::Desc))) if c == s => format!("{title} \u{2193}"),
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
        let sorted = column.is_some() && list.sort().map(|(c, _)| c) == column;
        let mut style = if sorted {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.dim)
        };
        // The column cursor: the column `s` will sort.
        if column == Some(list.column()) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        Cell::from(header_text(title, column, list.sort())).style(style)
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

/// The search box: always on the top line, so the filter in force is never
/// something to hunt for.
fn search_line<'a>(list: &List, typing: bool, theme: &Theme) -> Line<'a> {
    let label = Span::styled(
        format!("{}: ", list.mode().label()),
        Style::new().fg(theme.accent),
    );
    let problem = list
        .filter_error()
        .map(|why| Span::styled(format!("  {why}"), Style::new().fg(theme.error)));
    if typing {
        let mut spans = vec![
            label,
            Span::raw(list.filter().to_owned()),
            Span::styled("█", Style::new().fg(theme.accent)),
        ];
        spans.extend(problem);
        return Line::from(spans);
    }
    if list.filter().is_empty() {
        return Line::from(vec![
            label,
            Span::styled("press / or i to search", Style::new().fg(theme.dim)),
        ]);
    }
    let mut spans = vec![
        label,
        Span::raw(list.filter().to_owned()),
        Span::styled(
            format!("  {} of {}", list.visible().len(), list.total()),
            Style::new().fg(theme.dim),
        ),
    ];
    spans.extend(problem);
    Line::from(spans)
}

fn typing_hints(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        "type to filter (! not, | or)  ctrl-r regex  enter/esc done",
        Style::new().fg(theme.dim),
    ))
}

/// The bottom line of the picker: whatever the app is busy with or asking,
/// else the hidden count and the key hints.
fn status_line<'a>(app: &'a App, theme: &Theme, pending: &str) -> Line<'a> {
    if let Some(name) = app.loading() {
        let frame = SPINNER[app.spinner() % SPINNER.len()];
        Line::from(vec![
            Span::styled(
                format!("{frame} loading {name}…  "),
                Style::new().fg(theme.accent),
            ),
            Span::styled("esc cancel", Style::new().fg(theme.dim)),
        ])
    } else if let Some(what) = app.scanning() {
        Line::from(Span::styled(
            format!("scanning {what}\u{2026}"),
            Style::new().fg(theme.accent),
        ))
    } else if let Some(items) = app.menu() {
        Line::from(vec![
            Span::styled("collect from: ", Style::new().fg(theme.accent)),
            Span::styled(items, Style::new().fg(theme.fg)),
            Span::styled("  esc cancel", Style::new().fg(theme.dim)),
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
                "  enter ok  esc cancel",
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
    } else if let Some(notice) = app.notice() {
        Line::from(Span::styled(
            notice.to_owned(),
            Style::new().fg(theme.accent),
        ))
    } else {
        let mut spans: Vec<Span<'_>> = pending_span(pending, theme).into_iter().collect();
        if app.level() == Level::Refs && app.hidden() > 0 {
            spans.push(Span::styled(
                format!(
                    "{}{} hidden (H)  ",
                    if app.showing_hidden() { "showing " } else { "" },
                    app.hidden()
                ),
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
    }
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

    let line = status_line(app, theme, pending);
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

    let highlight = select.highlight();
    let rows = select
        .list()
        .visible_cells()
        .into_iter()
        .enumerate()
        .map(|(pos, (i, cells))| {
            let (mark, mut style) = if select.is_have(i) {
                ("(have)", Style::new().fg(theme.dim))
            } else if select.is_checked(i) {
                ("[x]", Style::new())
            } else {
                ("[ ]", Style::new())
            };
            if highlight.as_ref().is_some_and(|r| r.contains(&pos)) {
                style = style.bg(theme.selection);
            }
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
    } else {
        let counts = format!(
            "{} checked of {}  ",
            select.checked_count(),
            select.selectable_count()
        );
        let counts = match select.highlight() {
            Some(range) => format!("{} marked, space toggles  ", range.count()),
            None => counts,
        };
        let mut spans: Vec<Span<'_>> = pending_span(pending, theme).into_iter().collect();
        spans.push(Span::styled(counts, Style::new().fg(theme.accent)));
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
    fn pending_input_shows_in_the_status_line() {
        assert_eq!(pending_note(""), None);
        assert_eq!(pending_note("5g"), Some("5g  ".to_owned()));
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
        // The source column lines up too, and the header names the columns.
        let source_at = lines
            .iter()
            .find(|l| l.contains("merge"))
            .unwrap()
            .find("merge");
        assert_eq!(source_at, lines[2].find("source"));
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
    fn the_search_line_names_its_mode_and_shows_a_bad_pattern() {
        let mut app = App::new(vec![remote("origin", "https://h/o", &[])]);
        let theme = Theme::from(ThemeName::default());
        for action in [Action::Search, Action::ToggleRegex, Action::Char('(')] {
            app.step(crate::tui::keys::Command::from(action));
        }
        let lines = screen(80, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[0].starts_with("regex: ("), "{:?}", lines[0]);
        assert!(lines[0].contains("unclosed"), "{:?}", lines[0]);
        // The hint says how to switch back.
        assert!(lines[5].contains("ctrl-r"), "{:?}", lines[5]);

        let mut simple = App::new(vec![remote("origin", "https://h/o", &[])]);
        simple.step(crate::tui::keys::Command::from(Action::Search));
        let lines = screen(80, 6, |f| draw(f, &simple, &theme, ""));
        assert!(lines[0].starts_with("search: "), "{:?}", lines[0]);
    }

    #[test]
    fn the_status_line_shows_the_scan_menu_and_a_scan_in_progress() {
        let mut app = App::new(vec![remote("origin", "https://h/o", &[])]);
        app.step(crate::tui::keys::Command::from(Action::Scan));
        let theme = Theme::from(ThemeName::default());
        let lines = screen(80, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[5].contains("m merges"), "{:?}", lines[5]);
        assert!(lines[5].contains("t maintainers"), "{:?}", lines[5]);
        assert!(lines[5].contains("f url file"), "{:?}", lines[5]);
        app.step(crate::tui::keys::Command::from(Action::Back));
        app.begin_scan("merges");
        let lines = screen(80, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[5].contains("scanning merges"), "{:?}", lines[5]);
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
        assert!(REMOTE_HINTS.contains("p promote"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("o once"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("h/l col"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("s sort"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("A scan"), "{REMOTE_HINTS}");
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

    fn step(app: &mut App, actions: &[Action]) {
        let mut no_refs = |_: &Remote| Ok(Loaded::default());
        for &action in actions {
            app.update(action, &mut no_refs);
        }
    }

    #[test]
    fn the_header_shows_an_arrow_on_the_sorted_column_and_none_otherwise() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        let header = |app: &App| screen(60, 6, |f| draw(f, app, &theme, ""))[2].clone();
        assert!(!header(&app).contains('\u{2191}'), "{:?}", header(&app));
        assert!(!header(&app).contains('\u{2193}'), "{:?}", header(&app));
        step(&mut app, &[Action::ColumnNext, Action::Sort]);
        assert!(header(&app).contains("url \u{2191}"), "{:?}", header(&app));
        assert!(!header(&app).contains("name \u{2191}"));
        step(&mut app, &[Action::Sort]);
        assert!(header(&app).contains("url \u{2193}"), "{:?}", header(&app));
        step(&mut app, &[Action::Sort]);
        assert!(!header(&app).contains('\u{2191}'), "{:?}", header(&app));
        assert!(!header(&app).contains('\u{2193}'), "{:?}", header(&app));
    }

    #[test]
    fn the_sort_stays_on_its_column_while_the_column_cursor_moves() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        step(
            &mut app,
            &[Action::Sort, Action::ColumnNext, Action::ColumnNext],
        );
        let lines = screen(60, 6, |f| draw(f, &app, &theme, ""));
        assert!(lines[2].contains("name \u{2191}"), "{:?}", lines[2]);
    }

    #[test]
    fn the_header_underlines_the_column_under_the_cursor() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        let underlined = |app: &App| -> Vec<bool> {
            let mut terminal = ratatui::Terminal::new(TestBackend::new(60, 6)).unwrap();
            terminal.draw(|f| draw(f, app, &theme, "")).unwrap();
            let buffer = terminal.backend().buffer().clone();
            let line: String = (0..60).map(|x| buffer[(x, 2)].symbol()).collect();
            ["name", "url", "source"]
                .iter()
                .map(|title| {
                    let x = u16::try_from(line.find(title).unwrap()).unwrap();
                    buffer[(x, 2)].modifier.contains(Modifier::UNDERLINED)
                })
                .collect()
        };
        assert_eq!(underlined(&app), [true, false, false]);
        step(&mut app, &[Action::ColumnNext]);
        assert_eq!(underlined(&app), [false, true, false]);
    }

    #[test]
    fn the_bottom_line_does_not_report_the_sort() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        step(&mut app, &[Action::Sort]);
        let lines = screen(80, 6, |f| draw(f, &app, &theme, ""));
        assert!(!lines[5].contains("sort:"), "{:?}", lines[5]);
        assert!(lines[5].contains("s sort"), "{:?}", lines[5]);
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
                hidden: vec![
                    RemoteRef {
                        name: "refs/pull/1/merge".into(),
                        oid: "abcdef0123".into(),
                    };
                    412
                ],
                note: None,
            })
        };
        app.update(Action::Enter, &mut load);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(90, 6, |f| draw(f, &app, &theme, ""));
        assert!(
            lines.last().unwrap().contains("412 hidden (H)"),
            "{lines:?}"
        );
        app.step(crate::tui::keys::Command::from(Action::ShowHidden));
        let lines = screen(90, 6, |f| draw(f, &app, &theme, ""));
        assert!(
            lines.last().unwrap().contains("showing 412 hidden (H)"),
            "{lines:?}"
        );
    }

    #[test]
    fn the_refs_status_line_fits_80_columns_with_the_longest_hidden_count() {
        let counted = format!("showing {} hidden (H)  ", 99_999);
        assert!(counted.chars().count() + HINTS.chars().count() <= 80);
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
            assert!(hints.contains("h/l col"), "{hints}");
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
            source: String::new(),
            have: false,
        }]);
        s.update(Action::Search);
        s.update(Action::Char('x'));
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw_select(f, &s, &theme, ""));
        assert!(lines[0].contains("search: x"), "{:?}", lines[0]);
    }

    #[test]
    fn a_highlight_shows_its_size_and_that_space_toggles_it() {
        let row = |n: &str| Row {
            cells: vec![n.into()],
            name: n.into(),
            url: "u".into(),
            source: String::new(),
            have: false,
        };
        let mut s = Select::new(vec![row("a"), row("b"), row("c")]);
        let theme = Theme::from(ThemeName::default());
        let bottom = |s: &Select| {
            screen(100, 8, |f| draw_select(f, s, &theme, ""))
                .last()
                .cloned()
                .unwrap()
        };
        assert!(bottom(&s).contains("0 checked of 3"), "{}", bottom(&s));
        s.update(Action::ExtendDown);
        assert!(
            bottom(&s).contains("2 marked, space toggles"),
            "{}",
            bottom(&s)
        );
    }
}

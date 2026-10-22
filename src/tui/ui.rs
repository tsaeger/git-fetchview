// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Cell, Clear, Padding, Paragraph, Row, Table, TableState};

use crate::tui::app::{App, Level};
use crate::tui::keys::{HELP, HelpSection};
use crate::tui::list::{Direction, List, SortColumn};
use crate::tui::select::Select;
use crate::tui::theme::Theme;

/// Short, because the hidden-refs count can come first on the same line.
const HINTS: &str = "\u{21b5} open  esc back  / i search  s sort  ? help  q quit";
/// The top level has nowhere to go back to, and offers removing.
const REMOTE_HINTS: &str =
    "/ i search  f filter  s sort  n new  o once  A scan  d remove  ? help  q quit";
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SELECT_HINTS: &str =
    "space toggle  J/K range  a all  \u{21b5} write  / i search  f filter  ? help  q quit";

fn base(theme: &Theme) -> Style {
    Style::new().fg(theme.fg).bg(theme.bg)
}

/// A column header and the sort column it stands for, if it can be sorted.
type Header = (&'static str, Option<SortColumn>);

/// Marks a real git remote; collected ones get a blank.
const REAL_DOT: &str = "\u{25cf}";

const REMOTE_HEADERS: [Header; 4] = [
    ("", None),
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

/// Blank columns between data columns.
const COLUMN_GAP: u16 = 2;
/// The cursor symbol the table reserves room for.
const HIGHLIGHT_WIDTH: usize = 2;
/// A column is never squeezed below this.
const COLUMN_MIN: usize = 4;
/// The gutter (the column with no title) is at least this wide.
const GUTTER_MIN: usize = 2;

/// Width each column wants: the widest of its header and cells.
fn natural_widths(headers: &[Header], rows: &[Vec<String>]) -> Vec<usize> {
    let width = |s: &str| Line::from(s).width();
    headers
        .iter()
        .enumerate()
        .map(|(i, (title, _))| {
            let widest = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| width(c))
                .max()
                .unwrap_or(0);
            if title.is_empty() {
                widest.max(GUTTER_MIN)
            } else {
                widest.max(width(title) + 2)
            }
        })
        .collect()
}

/// Squeeze the widest titled columns until the lot fits in `room`. The
/// gutter never shrinks, and no column goes below `COLUMN_MIN`.
fn fit_widths(natural: &[usize], titled: &[bool], room: usize) -> Vec<usize> {
    let sum = |cap: usize| -> usize {
        natural
            .iter()
            .zip(titled)
            .map(|(&w, &t)| if t { w.min(cap) } else { w })
            .sum()
    };
    let mut cap = natural.iter().copied().max().unwrap_or(0);
    while cap > COLUMN_MIN && sum(cap) > room {
        cap -= 1;
    }
    natural
        .iter()
        .zip(titled)
        .map(|(&w, &t)| if t { w.min(cap.max(COLUMN_MIN)) } else { w })
        .collect()
}

/// `s` cut to `width` columns with the middle replaced by an ellipsis, so
/// both ends (the scheme and the repository name of a URL) stay readable.
fn elide(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_owned();
    }
    if width <= 1 {
        return "\u{2026}".repeat(width);
    }
    let keep = width - 1;
    let head = keep / 2;
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('\u{2026}');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// The panel is shown only when the list keeps at least this many rows.
const INFO_MIN_LIST: u16 = 6;

/// Split the body into the list and, when it is on and there is room, the
/// info panel under it: one line per titled column plus the border.
fn info_split(area: Rect, headers: &[Header], on: bool) -> (Rect, Option<Rect>) {
    let lines = headers.iter().filter(|h| !h.0.is_empty()).count();
    let height = u16::try_from(lines + 2).unwrap_or(u16::MAX);
    if !on || area.height < height + INFO_MIN_LIST {
        return (area, None);
    }
    let [list, panel] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(height)]).areas(area);
    (list, Some(panel))
}

/// The cursor row in full, one `name: value` line per column, the names
/// right-justified and the values left-justified.
fn draw_info(frame: &mut Frame<'_>, area: Rect, headers: &[Header], list: &List, theme: &Theme) {
    let block = Block::bordered()
        .border_style(Style::new().fg(theme.dim))
        .style(base(theme));
    let labels: Vec<&str> = headers
        .iter()
        .map(|h| h.0.trim())
        .filter(|t| !t.is_empty())
        .collect();
    let row = list.visible_cells().into_iter().nth(list.cursor());
    let label_width = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let lines: Vec<Line<'_>> = row
        .map(|(_, cells)| {
            labels
                .iter()
                .zip(cells)
                .map(|(label, value)| {
                    Line::from(vec![
                        Span::styled(
                            format!("{label:>label_width$}: "),
                            Style::new().fg(theme.dim),
                        ),
                        Span::raw(value.as_str()),
                    ])
                })
                .collect()
        })
        .unwrap_or_default();
    frame.render_widget(Paragraph::new(lines).block(block), area);
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
    let natural = natural_widths(headers, &cells);
    let titled: Vec<bool> = headers.iter().map(|h| !h.0.is_empty()).collect();
    // Room for the data after the border, the cursor symbol and the gaps
    // between data columns (the gutter has none after it).
    let gaps = titled[..titled.len() - 1].iter().filter(|&&t| t).count() * usize::from(COLUMN_GAP);
    let room = usize::from(area.width)
        .saturating_sub(2 + HIGHLIGHT_WIDTH + gaps)
        .max(COLUMN_MIN);
    let widths = fit_widths(&natural, &titled, room);
    // Each column is followed by a gap, except the gutter, and a last empty
    // column takes whatever width is left.
    let spacer = || Cell::from("");
    let mut constraints = Vec::new();
    for (i, &w) in widths.iter().enumerate() {
        constraints.push(Constraint::Length(u16::try_from(w).unwrap_or(u16::MAX)));
        if titled[i] && i + 1 < widths.len() {
            constraints.push(Constraint::Length(COLUMN_GAP));
        }
    }
    constraints.push(Constraint::Fill(1));

    let boxes = list.filterable();
    let mut header_cells = Vec::new();
    for (i, &(title, column)) in headers.iter().enumerate() {
        let sorted = column.is_some() && list.sort().map(|(c, _)| c) == column;
        let mut style = if sorted {
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.dim)
        };
        // The column cursor: the column `s` will sort. With filter boxes it is
        // underlined there, where the eye is; without them, on the header.
        if !boxes && column == Some(list.column()) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        let mut lines = vec![Line::styled(header_text(title, column, list.sort()), style)];
        if boxes {
            lines.push(filter_box(column, list, theme));
        }
        header_cells.push(Cell::from(Text::from(lines)));
        if titled[i] && i + 1 < headers.len() {
            header_cells.push(spacer());
        }
    }
    header_cells.push(spacer());
    let header = Row::new(header_cells).height(if boxes { 2 } else { 1 });
    let body = rows.into_iter().map(|r| {
        let mut out = Vec::new();
        for (i, cell) in r.cells.iter().enumerate() {
            out.push(Cell::from(elide(cell, widths[i])));
            if titled[i] && i + 1 < widths.len() {
                out.push(spacer());
            }
        }
        out.push(spacer());
        Row::new(out).style(r.style)
    });

    let table = Table::new(body, constraints)
        .header(header)
        .column_spacing(0)
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

/// Space between the help border and its text, and how far the panel is lifted
/// from the background, in percent.
const HELP_PAD_X: u16 = 2;
const HELP_PAD_Y: u16 = 1;
const HELP_TINT: u16 = 8;

/// The lines of one help section, keys first, then a blank line.
fn help_lines<'a>(section: &HelpSection, key_width: usize, theme: &Theme) -> Vec<Line<'a>> {
    let mut lines = vec![Line::styled(
        section.title,
        Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
    )];
    for entry in section.entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:key_width$}  ", entry.keys),
                Style::new().add_modifier(Modifier::BOLD),
            ),
            Span::styled(entry.does, Style::new().fg(theme.dim)),
        ]));
    }
    lines.push(Line::default());
    lines
}

/// Every key, in sections, over whatever is on screen. Any key closes it.
fn draw_help(frame: &mut Frame<'_>, theme: &Theme) {
    let area = frame.area();
    let room = usize::from(area.height.saturating_sub(2 + 2 * HELP_PAD_Y)).max(1);

    // Whole sections go into a column until the next one would not fit.
    let mut columns: Vec<Vec<&HelpSection>> = vec![Vec::new()];
    let mut used = 0;
    for section in HELP {
        let height = section.entries.len() + 2;
        if used > 0 && used + height - 1 > room {
            columns.push(Vec::new());
            used = 0;
        }
        columns
            .last_mut()
            .expect("starts with one column")
            .push(section);
        used += height;
    }
    // Each column is as wide as its own keys and descriptions.
    let widths: Vec<(usize, usize)> = columns
        .iter()
        .map(|c| {
            let entries = || c.iter().flat_map(|s| s.entries);
            let keys = entries().map(|e| e.keys.chars().count()).max().unwrap_or(0);
            let does = entries().map(|e| e.does.chars().count()).max().unwrap_or(0);
            (keys, keys + 2 + does)
        })
        .collect();
    let gap = 2;
    let content: usize = widths.iter().map(|w| w.1).sum::<usize>() + gap * (columns.len() - 1);
    let tallest = columns
        .iter()
        .map(|c| {
            c.iter()
                .map(|s| s.entries.len() + 2)
                .sum::<usize>()
                .saturating_sub(1)
        })
        .max()
        .unwrap_or(0);
    let width = u16::try_from(content + 2 + usize::from(2 * HELP_PAD_X))
        .unwrap_or(u16::MAX)
        .min(area.width);
    let height = u16::try_from(tallest + 2 + usize::from(2 * HELP_PAD_Y))
        .unwrap_or(u16::MAX)
        .min(area.height);
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    // Dim everything behind the popup so the keys are what the eye lands on.
    let dimmed = Style::new().fg(theme.dim).remove_modifier(Modifier::BOLD);
    for cell in &mut frame.buffer_mut().content {
        cell.set_style(dimmed);
    }
    frame.render_widget(Clear, popup);
    let panel = Style::new()
        .fg(theme.fg)
        .bg(mix(theme.bg, theme.fg, HELP_TINT));
    let block = Block::bordered()
        .title(Span::styled(
            " keys \u{2014} any key closes ",
            Style::new().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::new().fg(theme.accent))
        .padding(Padding::symmetric(HELP_PAD_X, HELP_PAD_Y))
        .style(panel);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let slots = Layout::horizontal(
        widths
            .iter()
            .map(|w| Constraint::Length(u16::try_from(w.1).unwrap_or(u16::MAX))),
    )
    .spacing(u16::try_from(gap).unwrap_or(2))
    .split(inner);
    for ((sections, (key_width, _)), slot) in columns.iter().zip(&widths).zip(slots.iter()) {
        let lines: Vec<Line<'_>> = sections
            .iter()
            .flat_map(|s| help_lines(s, *key_width, theme))
            .collect();
        frame.render_widget(Paragraph::new(lines).style(panel), *slot);
    }
}

/// Mix `over` into `base`, `pct` percent of the way. Colours that are not RGB
/// cannot be mixed, so `over` stands in.
fn mix(base: Color, over: Color, pct: u16) -> Color {
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (base, over) else {
        return over;
    };
    let part = |a: u8, b: u8| {
        let v = (u16::from(a) * (100 - pct) + u16::from(b) * pct) / 100;
        u8::try_from(v).unwrap_or(u8::MAX)
    };
    Color::Rgb(part(r1, r2), part(g1, g2), part(b1, b2))
}

/// The filter box under a column header: darker when blank (with a dim
/// placeholder), tinted when it has text, brighter while it is being edited.
/// The column cursor underlines its box.
fn filter_box<'a>(column: Option<SortColumn>, list: &List, theme: &Theme) -> Line<'a> {
    let Some(k) = column.and_then(|c| list.columns().iter().position(|&x| x == c)) else {
        return Line::default();
    };
    let text = list.column_filter(k);
    let editing = list.editing_column() == Some(k);
    let (content, fg, bg) = if editing {
        (format!("{text}\u{2588}"), theme.fg, theme.selection)
    } else if text.is_empty() {
        ("filter".to_owned(), theme.dim, mix(theme.bg, theme.fg, 10))
    } else {
        (text.to_owned(), theme.fg, mix(theme.bg, theme.accent, 25))
    };
    let mut style = Style::new().fg(fg).bg(bg);
    if k == list.column_index() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    Line::styled(content, style)
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
        Paragraph::new(search_line(
            app.list(),
            app.typing() && app.list().editing_column().is_none(),
            theme,
        ))
        .style(base(theme)),
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
            .map(|(i, cells)| {
                let mut row = cells.to_vec();
                if app.level() == Level::Remotes {
                    let dot = if app.is_real(i) { REAL_DOT } else { " " };
                    row.insert(0, dot.to_owned());
                }
                TableRow {
                    cells: row,
                    style: Style::new(),
                }
            })
            .collect();
        match app.level() {
            Level::Refs => (&REF_HEADERS, rows),
            Level::Remotes => (&REMOTE_HEADERS, rows),
        }
    };
    let (body, info) = info_split(body, headers, app.info());
    render_table(frame, body, title, headers, app.list(), rows, theme);
    if let Some(panel) = info {
        draw_info(frame, panel, headers, app.list(), theme);
    }

    let line = status_line(app, theme, pending);
    frame.render_widget(Paragraph::new(line).style(base(theme)), status);
    if app.help() {
        draw_help(frame, theme);
    }
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
        Paragraph::new(search_line(
            select.list(),
            select.typing() && select.list().editing_column().is_none(),
            theme,
        ))
        .style(base(theme)),
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
    let (body, info) = info_split(body, &SELECT_HEADERS, select.info());
    render_table(
        frame,
        body,
        " add remotes ".to_owned(),
        &SELECT_HEADERS,
        select.list(),
        rows,
        theme,
    );
    if let Some(panel) = info {
        draw_info(frame, panel, &SELECT_HEADERS, select.list(), theme);
    }

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
    if select.help() {
        draw_help(frame, theme);
    }
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
    fn elide_keeps_both_ends_and_fits_the_width() {
        assert_eq!(elide("short", 10), "short");
        let out = elide("git://git.kernel.org/pub/scm/linux/x.git", 15);
        assert_eq!(out.chars().count(), 15);
        assert!(out.starts_with("git://"), "{out}");
        assert!(out.ends_with("x.git"), "{out}");
        assert!(out.contains('\u{2026}'), "{out}");
        assert_eq!(elide("abcdef", 1), "\u{2026}");
    }

    #[test]
    fn only_the_widest_titled_columns_are_squeezed_and_never_the_gutter() {
        let natural = [2, 10, 60, 8];
        let titled = [false, true, true, true];
        assert_eq!(fit_widths(&natural, &titled, 100), vec![2, 10, 60, 8]);
        assert_eq!(fit_widths(&natural, &titled, 40), vec![2, 10, 20, 8]);
    }

    #[test]
    fn the_gutter_is_two_wide_and_the_filter_box_starts_at_the_names() {
        let app = App::new(vec![remote("origin", "https://h/o", &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(80, 8, |f| draw(f, &app, &theme, ""));
        let row = lines.iter().find(|l| l.contains("origin")).unwrap();
        let col = |l: &str, needle: &str| l[..l.find(needle).unwrap()].chars().count();
        // Border, then the two-wide cursor symbol, then a two-wide gutter.
        assert_eq!(col(row, "origin"), 5);
        assert!(row.contains("\u{25cf} origin"), "{row:?}");
        assert_eq!(col(&lines[2], "name"), col(row, "origin"));
        assert_eq!(col(&lines[3], "filter"), col(row, "origin"));
    }

    #[test]
    fn source_is_only_as_wide_as_its_content_and_the_rest_is_blank() {
        let app = App::new(vec![remote("a", "https://h/a", &["merge"])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(120, 8, |f| draw(f, &app, &theme, ""));
        let header = &lines[2];
        let source_at = header[..header.find("source").unwrap()].chars().count();
        assert!(source_at < 40, "{header:?}");
        let filters = &lines[3];
        assert!(filters.trim_end_matches(['\u{2502}', ' ']).chars().count() < 60);
    }

    #[test]
    fn a_long_url_loses_its_middle_not_its_end() {
        let url = "git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git";
        let app = App::new(vec![remote("torvalds", url, &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(50, 8, |f| draw(f, &app, &theme, ""));
        let row = lines.iter().find(|l| l.contains("torvalds")).unwrap();
        assert!(row.contains("git://"), "{row:?}");
        assert!(row.contains('\u{2026}'), "{row:?}");
        assert!(row.contains("linux.git"), "{row:?}");
    }

    #[test]
    fn the_info_panel_shows_the_cursor_row_with_names_right_justified() {
        let mut app = App::new(vec![
            remote("origin", "https://h/o", &[]),
            remote("netdev/net-next", "https://h/n", &["merge"]),
        ]);
        let theme = Theme::from(ThemeName::default());
        step(&mut app, &[Action::Down]);
        let lines = screen(80, 20, |f| draw(f, &app, &theme, ""));
        assert!(
            lines.iter().any(|l| l.contains("  name: netdev/net-next")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("   url: https://h/n")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("source: merge")),
            "{lines:?}"
        );
    }

    #[test]
    fn v_hides_the_info_panel_and_a_short_screen_has_none() {
        let mut app = App::new(vec![remote("origin", "https://h/o", &[])]);
        let theme = Theme::from(ThemeName::default());
        let shown = |app: &App, h| {
            screen(80, h, |f| draw(f, app, &theme, ""))
                .iter()
                .any(|l| l.contains("source: "))
        };
        assert!(shown(&app, 20));
        assert!(!shown(&app, 10), "no room");
        step(&mut app, &[Action::Info]);
        assert!(!shown(&app, 20));
    }

    #[test]
    fn the_add_picker_has_the_info_panel_too() {
        let s = Select::new(vec![Row {
            cells: vec!["linux".into(), "https://h/linux.git".into(), "merge".into()],
            name: "linux".into(),
            url: "https://h/linux.git".into(),
            source: "merge".into(),
            have: false,
        }]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(80, 20, |f| draw_select(f, &s, &theme, ""));
        assert!(
            lines.iter().any(|l| l.contains("url: https://h/linux.git")),
            "{lines:?}"
        );
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
            // Columns, not bytes: the dot is wider than a space in UTF-8.
            line[..line.find("https://").unwrap()].chars().count()
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
        assert!(REMOTE_HINTS.contains("o once"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("? help"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("s sort"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("A scan"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.contains("/ i search"), "{REMOTE_HINTS}");
        assert!(REMOTE_HINTS.chars().count() <= 80, "{REMOTE_HINTS}");
    }

    #[test]
    fn a_password_in_a_remote_url_is_not_drawn() {
        let app = App::new(vec![remote("work", "https://dev:s3cret@h/o.git", &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(60, 8, |f| draw(f, &app, &theme, ""));
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
            // The box under the header is where the cursor shows.
            assert!(!buffer[(2, 2)].modifier.contains(Modifier::UNDERLINED));
            ["name", "url", "source"]
                .iter()
                .map(|title| {
                    let x = u16::try_from(line.find(title).unwrap()).unwrap();
                    buffer[(x, 3)].modifier.contains(Modifier::UNDERLINED)
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
        let lines = screen(40, 8, |f| draw(f, &app, &theme, ""));
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
            assert!(hints.contains("? help"), "{hints}");
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

    #[test]
    fn real_remotes_get_a_gutter_dot_and_collected_ones_do_not() {
        let app = App::new(vec![
            remote("origin", "https://h/o", &[]),
            remote("net", "https://h/net", &["maintainers"]),
        ]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(80, 8, |f| draw(f, &app, &theme, ""));
        let origin = lines.iter().find(|l| l.contains("origin")).unwrap();
        let net = lines.iter().find(|l| l.contains("net")).unwrap();
        assert!(origin.contains(REAL_DOT), "{origin:?}");
        assert!(!net.contains(REAL_DOT), "{net:?}");
    }

    #[test]
    fn blank_filter_boxes_show_a_placeholder_until_one_is_edited() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(70, 8, |f| draw(f, &app, &theme, ""));
        assert_eq!(lines[3].matches("filter").count(), 3, "{:?}", lines[3]);
        step(&mut app, &[Action::Filter]);
        let lines = screen(70, 8, |f| draw(f, &app, &theme, ""));
        // The box being edited drops its placeholder and shows a cursor.
        assert_eq!(lines[3].matches("filter").count(), 2, "{:?}", lines[3]);
        assert!(lines[3].contains('\u{2588}'), "{:?}", lines[3]);
        step(&mut app, &[Action::Char('z')]);
        let lines = screen(70, 8, |f| draw(f, &app, &theme, ""));
        assert!(lines[3].contains("z\u{2588}"), "{:?}", lines[3]);
        assert!(
            !lines[0].contains('\u{2588}'),
            "the search box has no cursor"
        );
    }

    #[test]
    fn the_refs_list_has_no_filter_boxes() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let mut load = |_: &Remote| {
            Ok(Loaded {
                refs: vec![RemoteRef {
                    name: "refs/heads/dev".into(),
                    oid: "1234567890".into(),
                }],
                ..Loaded::default()
            })
        };
        app.update(Action::Enter, &mut load);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(70, 8, |f| draw(f, &app, &theme, ""));
        assert!(
            lines.iter().any(|l| l.contains("refs/heads/dev")),
            "{lines:?}"
        );
        assert!(lines.iter().all(|l| !l.contains("filter")), "{lines:?}");
    }

    #[test]
    fn the_help_shows_every_key_at_80_by_24_and_leaves_the_list_behind_it() {
        let mut app = App::new(vec![remote("a", "https://h/a", &[])]);
        let theme = Theme::from(ThemeName::default());
        step(&mut app, &[Action::Help]);
        assert!(app.help());
        let lines = screen(80, 24, |f| draw(f, &app, &theme, ""));
        let all = lines.join("\n");
        for section in HELP {
            assert!(all.contains(section.title), "{}\n{all}", section.title);
            for entry in section.entries {
                assert!(all.contains(entry.keys), "{}\n{all}", entry.keys);
                assert!(all.contains(entry.does), "{}\n{all}", entry.does);
            }
        }
        assert!(lines.iter().all(|l| l.chars().count() <= 80));
    }

    #[test]
    fn the_help_is_over_the_add_picker_too() {
        let mut s = Select::new(vec![Row {
            cells: vec!["x".into()],
            name: "x".into(),
            url: "u".into(),
            source: String::new(),
            have: false,
        }]);
        s.update(Action::Help);
        let theme = Theme::from(ThemeName::default());
        let lines = screen(80, 24, |f| draw_select(f, &s, &theme, ""));
        assert!(lines.join("\n").contains("any key closes"));
    }
}

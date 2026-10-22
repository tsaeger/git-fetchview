// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! A filterable list with a cursor.

use std::cmp::Ordering;

use crate::search::{Matcher, Mode};
use crate::tui::keys::Action;

/// A column a list can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    /// By name, case-insensitive, within each group.
    Label,
    /// By URL, case-insensitive, within each group.
    Url,
    /// By source, case-insensitive, within each group.
    Source,
}

impl SortColumn {
    /// Every column, in the order they are shown.
    pub const ALL: [Self; 3] = [Self::Label, Self::Url, Self::Source];
}

/// Which way a sorted column runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// A to Z, and `v6.9` before `v6.10`.
    Asc,
    /// The reverse of [`Direction::Asc`].
    Desc,
}

/// What a row is sorted by. Sorting never moves a row out of its group.
#[derive(Debug, Clone, Default)]
pub struct SortKeys {
    /// Name to sort by in label order.
    pub name: String,
    /// URL to sort by in URL order.
    pub url: String,
    /// Source to sort by in source order.
    pub source: String,
    /// Rows sort within their group; lower groups come first.
    pub group: u8,
}

/// What typed characters go into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Target {
    /// The search box on the top line.
    #[default]
    Search,
    /// The filter box under this column's header.
    Column(usize),
}

/// One column's filter: the text and what it compiled to.
#[derive(Debug, Clone, Default)]
struct ColumnFilter {
    text: String,
    matcher: Matcher,
}

/// Rows of display text plus a search filter and a cursor into the visible rows.
#[derive(Debug, Clone, Default)]
pub struct List {
    labels: Vec<String>,
    cells: Vec<Vec<String>>,
    keys: Vec<SortKeys>,
    /// The sorted column and its direction. `None` is the order given.
    sorted: Option<(SortColumn, Direction)>,
    /// Which of [`List::columns`] the column cursor is on.
    column: usize,
    columns: Vec<SortColumn>,
    filter: String,
    mode: Mode,
    /// The last filter that compiled, which is what the rows show.
    matcher: Matcher,
    /// Why the filter as typed does not compile, in regex mode.
    filter_error: Option<String>,
    /// Per-column filters, by column index. Filled the first time one is edited.
    column_filters: Vec<ColumnFilter>,
    target: Target,
    cursor: usize,
}

impl List {
    /// Build a list from display labels. Rows sort by their text.
    #[must_use]
    pub fn new(labels: Vec<String>) -> Self {
        let keys = labels
            .iter()
            .map(|label| SortKeys {
                name: label.clone(),
                url: label.clone(),
                source: String::new(),
                group: 0,
            })
            .collect();
        Self::with_keys(labels, keys)
    }

    /// Build a list whose rows sort by the given keys.
    #[must_use]
    pub fn with_keys(labels: Vec<String>, keys: Vec<SortKeys>) -> Self {
        debug_assert_eq!(labels.len(), keys.len());
        let cells = labels.iter().map(|l| vec![l.clone()]).collect();
        Self {
            labels,
            cells,
            keys,
            ..Self::default()
        }
    }

    /// Build a list whose rows are columns of cells. Filtering matches the
    /// cells joined with two spaces, so a filter sees the whole row.
    #[must_use]
    pub fn with_cells(cells: Vec<Vec<String>>, keys: Vec<SortKeys>) -> Self {
        debug_assert_eq!(cells.len(), keys.len());
        let labels = cells.iter().map(|row| join_cells(row)).collect();
        Self {
            labels,
            cells,
            keys,
            ..Self::default()
        }
    }

    /// Limit the sortable columns to these, in this order. By default every
    /// column is available.
    #[must_use]
    pub fn with_columns(mut self, columns: Vec<SortColumn>) -> Self {
        self.columns = columns;
        self
    }

    /// The sortable columns this list offers, in the order they are shown.
    #[must_use]
    pub fn columns(&self) -> &[SortColumn] {
        if self.columns.is_empty() {
            &SortColumn::ALL
        } else {
            &self.columns
        }
    }

    /// The column under the column cursor.
    #[must_use]
    pub fn column(&self) -> SortColumn {
        self.columns()[self.column.min(self.columns().len() - 1)]
    }

    /// The sorted column and its direction, or `None` for the order given.
    #[must_use]
    pub fn sort(&self) -> Option<(SortColumn, Direction)> {
        self.sorted
    }

    /// Keep the cursor on the row it was on after the order changed.
    fn follow(&mut self, selected: Option<usize>) {
        self.cursor = selected
            .and_then(|row| self.visible().iter().position(|&i| i == row))
            .unwrap_or(0);
    }

    /// Move the column cursor `delta` columns along, stopping at either end.
    /// The sort does not change.
    pub fn move_column(&mut self, delta: isize) {
        let last = self.columns().len() - 1;
        self.column = self.column.min(last).saturating_add_signed(delta).min(last);
    }

    /// Step the sort of the column under the cursor: ascending, descending,
    /// none. A column that is not the sorted one starts at ascending. The
    /// cursor stays on its row, except that it goes to the top when the sort
    /// turns descending, so the end that matters is in view.
    pub fn cycle_sort(&mut self) {
        let selected = self.selected();
        let column = self.column();
        self.sorted = match self.sorted {
            Some((c, Direction::Asc)) if c == column => Some((column, Direction::Desc)),
            Some((c, Direction::Desc)) if c == column => None,
            _ => Some((column, Direction::Asc)),
        };
        if matches!(self.sorted, Some((_, Direction::Desc))) {
            self.cursor = 0;
        } else {
            self.follow(selected);
        }
    }

    /// How many rows there are, before filtering.
    #[must_use]
    pub fn total(&self) -> usize {
        self.labels.len()
    }

    /// Drop row `index` (an original index). The cursor stays on the same
    /// position, so it lands on the next row, and is pulled back if that was
    /// the last one.
    pub fn remove(&mut self, index: usize) {
        self.labels.remove(index);
        self.cells.remove(index);
        self.keys.remove(index);
        let last = self.visible().len().saturating_sub(1);
        self.cursor = self.cursor.min(last);
    }

    /// Put a row at an original index, shifting the ones after it up. The
    /// label is the cells joined as in [`List::with_cells`].
    pub fn insert(&mut self, index: usize, cells: Vec<String>, keys: SortKeys) {
        self.labels.insert(index, join_cells(&cells));
        self.cells.insert(index, cells);
        self.keys.insert(index, keys);
    }

    /// Move the cursor to the row at original index `index`. A filter that
    /// hides it is cleared first.
    pub fn focus(&mut self, index: usize) {
        if !self.visible().contains(&index) {
            self.clear();
        }
        if let Some(position) = self.visible().iter().position(|&i| i == index) {
            self.cursor = position;
        }
    }

    /// The current filter text.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Indices (into the original labels) of rows matching the filter, in
    /// the current sort order.
    #[must_use]
    pub fn visible(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = (0..self.labels.len())
            .filter(|&i| self.matcher.is_row_match(&self.labels[i], &self.cells[i]))
            .filter(|&i| self.column_filters_match(i))
            .collect();
        let keys = &self.keys;
        if let Some((column, direction)) = self.sorted {
            let key: fn(&SortKeys) -> &str = match column {
                SortColumn::Label => |k| &k.name,
                SortColumn::Url => |k| &k.url,
                SortColumn::Source => |k| &k.source,
            };
            rows.sort_by(|&a, &b| {
                let order = natural_cmp(key(&keys[a]), key(&keys[b]))
                    .then_with(|| natural_cmp(&keys[a].name, &keys[b].name))
                    .then_with(|| a.cmp(&b));
                keys[a].group.cmp(&keys[b].group).then(match direction {
                    Direction::Asc => order,
                    Direction::Desc => order.reverse(),
                })
            });
        }
        rows
    }

    /// Labels of the visible rows, in order.
    #[must_use]
    pub fn visible_labels(&self) -> Vec<&str> {
        self.visible()
            .into_iter()
            .map(|i| self.labels[i].as_str())
            .collect()
    }

    /// Visible rows as `(original index, label)`.
    #[must_use]
    pub fn visible_rows(&self) -> Vec<(usize, &str)> {
        self.visible()
            .into_iter()
            .map(|i| (i, self.labels[i].as_str()))
            .collect()
    }

    /// Visible rows as `(original index, cells)`.
    #[must_use]
    pub fn visible_cells(&self) -> Vec<(usize, &[String])> {
        self.visible()
            .into_iter()
            .map(|i| (i, self.cells[i].as_slice()))
            .collect()
    }

    /// Cursor position among the visible rows.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Original index of the row under the cursor.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.visible().get(self.cursor).copied()
    }

    /// Move the cursor down one row, stopping at the end.
    pub fn down(&mut self) {
        let last = self.visible().len().saturating_sub(1);
        self.cursor = (self.cursor + 1).min(last);
    }

    /// Move the cursor up one row, stopping at the start.
    pub fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor by `delta` rows, stopping at either end.
    pub fn move_by(&mut self, delta: isize) {
        let last = self.visible().len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add_signed(delta).min(last);
    }

    /// Put the cursor on a 1-based row, or the last row if it is past the end.
    pub fn goto(&mut self, row: usize) {
        let last = self.visible().len().saturating_sub(1);
        self.cursor = row.saturating_sub(1).min(last);
    }

    /// Apply a motion. Returns false if `action` is not a motion.
    ///
    /// `viewport` is how many rows are on screen; paging moves by it.
    pub fn motion(&mut self, action: Action, count: Option<usize>, viewport: usize) -> bool {
        let rows = |n: usize| isize::try_from(n).unwrap_or(isize::MAX);
        let times = rows(count.map_or(1, |c| c.max(1)));
        let half = rows((viewport / 2).max(1));
        let page = rows(viewport.saturating_sub(1).max(1));
        match action {
            Action::Down => self.move_by(times),
            Action::Up => self.move_by(-times),
            Action::HalfPageDown => self.move_by(half),
            Action::HalfPageUp => self.move_by(-half),
            Action::PageDown => self.move_by(page),
            Action::PageUp => self.move_by(-page),
            Action::Top => match count {
                Some(row) => self.goto(row),
                None => self.top(),
            },
            Action::Bottom => match count {
                Some(row) => self.goto(row),
                None => self.bottom(),
            },
            _ => return false,
        }
        true
    }

    /// Jump to the first row.
    pub fn top(&mut self) {
        self.cursor = 0;
    }

    /// Jump to the last row.
    pub fn bottom(&mut self) {
        self.cursor = self.visible().len().saturating_sub(1);
    }

    /// Append to whatever is being edited: the search or a column's filter.
    pub fn push(&mut self, c: char) {
        match self.target {
            Target::Search => self.filter.push(c),
            Target::Column(k) => self.column_filters[k].text.push(c),
        }
        self.refilter();
    }

    /// Remove the last character of whatever is being edited.
    pub fn pop(&mut self) {
        match self.target {
            Target::Search => self.filter.pop(),
            Target::Column(k) => self.column_filters[k].text.pop(),
        };
        self.refilter();
    }

    /// Whether this list has more than one column to filter.
    #[must_use]
    pub fn filterable(&self) -> bool {
        self.columns().len() > 1
    }

    /// Start editing the filter of the column under the column cursor.
    /// Returns false if the list has no per-column filters.
    pub fn begin_column_edit(&mut self) -> bool {
        if !self.filterable() {
            return false;
        }
        let columns = self.columns().len();
        self.column_filters
            .resize_with(columns, ColumnFilter::default);
        self.target = Target::Column(self.column_index());
        true
    }

    /// Start editing the search box.
    pub fn begin_search(&mut self) {
        self.target = Target::Search;
    }

    /// Stop editing a column filter; typed characters go to the search again.
    pub fn end_edit(&mut self) {
        self.target = Target::Search;
    }

    /// The column whose filter is being edited, if any.
    #[must_use]
    pub fn editing_column(&self) -> Option<usize> {
        match self.target {
            Target::Column(k) => Some(k),
            Target::Search => None,
        }
    }

    /// Where the column cursor is, as an index into [`List::columns`].
    #[must_use]
    pub fn column_index(&self) -> usize {
        self.column.min(self.columns().len() - 1)
    }

    /// The filter text of column `k`; empty if it has none.
    #[must_use]
    pub fn column_filter(&self, k: usize) -> &str {
        self.column_filters.get(k).map_or("", |f| f.text.as_str())
    }

    fn column_filters_match(&self, row: usize) -> bool {
        self.column_filters.iter().enumerate().all(|(k, f)| {
            f.text.is_empty() || {
                let cell = self.cells[row].get(k).map_or("", String::as_str);
                f.matcher.is_match(cell)
            }
        })
    }

    /// Clear the search, the column filters and the sort. The column cursor
    /// stays where it is.
    pub fn clear(&mut self) {
        self.filter.clear();
        self.column_filters.iter_mut().for_each(|f| f.text.clear());
        self.target = Target::Search;
        self.sorted = None;
        self.refilter();
    }

    /// Read the same text as the other kind of search.
    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.toggled();
        self.refilter();
    }

    /// How the filter text is read.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Why the filter does not compile, if it is a bad regex. The rows are
    /// those of the last filter that did.
    #[must_use]
    pub fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    fn refilter(&mut self) {
        let mode = self.mode;
        let mut error = None;
        match Matcher::new(mode, &self.filter) {
            Ok(matcher) => self.matcher = matcher,
            Err(why) => error = Some(why),
        }
        for f in &mut self.column_filters {
            match Matcher::new(mode, &f.text) {
                Ok(matcher) => f.matcher = matcher,
                Err(why) => error = Some(why),
            }
        }
        self.filter_error = error;
        self.cursor = 0;
    }
}

/// A row's filter text: its non-empty cells, two spaces apart.
fn join_cells(cells: &[String]) -> String {
    cells
        .iter()
        .filter(|c| !c.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("  ")
}

/// Compare strings the way people read them: case-insensitively, with runs
/// of digits compared as numbers, so `v6.9` sorts before `v6.10`.
#[must_use]
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a = a.chars().peekable();
    let mut b = b.chars().peekable();
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na = take_number(&mut a);
                let nb = take_number(&mut b);
                let (na, nb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let order = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut digits = String::new();
    while let Some(&c) = chars.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        digits.push(c);
        chars.next();
    }
    digits
}

#[cfg(test)]
mod tests {
    fn typed(l: &mut List, text: &str) {
        for c in text.chars() {
            l.push(c);
        }
    }

    fn source() -> List {
        List::new(
            ["v6.1", "v6.1-rc1", "v6.2", "v6.2-rc1", "net-next"]
                .map(String::from)
                .to_vec(),
        )
    }

    #[test]
    fn the_filter_understands_and_not_and_or() {
        let mut l = source();
        typed(&mut l, "v6 !rc");
        assert_eq!(l.visible_labels(), ["v6.1", "v6.2"]);
        l.clear();
        typed(&mut l, "v6.1 !rc | net");
        assert_eq!(l.visible_labels(), ["v6.1", "net-next"]);
    }

    #[test]
    fn regex_mode_filters_by_pattern_and_toggling_keeps_the_text() {
        let mut l = source();
        typed(&mut l, "^v6\\.[12]$");
        // In simple mode that is a literal string, which matches nothing.
        assert_eq!(l.visible().len(), 0);
        l.toggle_mode();
        assert_eq!(l.mode(), crate::search::Mode::Regex);
        assert_eq!(l.filter(), "^v6\\.[12]$");
        assert_eq!(l.visible_labels(), ["v6.1", "v6.2"]);
        l.toggle_mode();
        assert_eq!(l.mode(), crate::search::Mode::Simple);
        assert_eq!(l.visible().len(), 0);
    }

    #[test]
    fn a_bad_regex_keeps_the_last_list_and_says_why() {
        let mut l = source();
        l.toggle_mode();
        typed(&mut l, "rc1");
        assert_eq!(l.visible().len(), 2);
        l.push('(');
        assert!(l.filter_error().is_some());
        // The rows are the ones for the last pattern that compiled.
        assert_eq!(l.visible().len(), 2);
        l.pop();
        assert_eq!(l.filter_error(), None);
        assert_eq!(l.visible().len(), 2);
        l.push('(');
        l.clear();
        assert_eq!(l.filter_error(), None);
        assert_eq!(l.visible().len(), 5);
    }

    #[test]
    fn remove_drops_a_row_and_keeps_the_cursor_in_range() {
        let mut l = List::new(vec!["a".into(), "b".into(), "c".into()]);
        l.motion(Action::Bottom, None, 10);
        assert_eq!(l.selected(), Some(2));
        l.remove(2);
        assert_eq!(l.total(), 2);
        assert_eq!(l.selected(), Some(1));
        l.remove(0);
        assert_eq!(l.visible_labels(), ["b"]);
        assert_eq!(l.selected(), Some(0));
        l.remove(0);
        assert_eq!(l.selected(), None);
    }

    #[test]
    fn insert_puts_a_row_at_an_original_index_and_focus_moves_the_cursor_to_it() {
        let mut l = List::new(vec!["a".into(), "c".into()]);
        l.insert(1, vec!["b".into()], SortKeys::default());
        assert_eq!(l.visible_labels(), ["a", "b", "c"]);
        l.focus(1);
        assert_eq!(l.selected(), Some(1));
        l.focus(2);
        assert_eq!(l.selected(), Some(2));
    }

    #[test]
    fn focus_clears_a_filter_that_hides_the_row() {
        let mut l = List::new(vec!["apple".into(), "berry".into()]);
        l.push('a');
        assert_eq!(l.visible().len(), 1);
        l.focus(1);
        assert_eq!(l.filter(), "");
        assert_eq!(l.selected(), Some(1));
    }

    #[test]
    fn total_counts_rows_the_filter_hides() {
        let mut l = List::new(vec!["a".into(), "b".into()]);
        l.push('a');
        assert_eq!((l.visible().len(), l.total()), (1, 2));
    }

    use super::*;
    use crate::tui::keys::Action;

    fn list() -> List {
        List::new(vec![
            "refs/heads/main".into(),
            "refs/heads/Dev".into(),
            "refs/source/v1".into(),
        ])
    }

    #[test]
    fn filter_is_case_insensitive_substring() {
        let mut l = list();
        "dev".chars().for_each(|c| l.push(c));
        assert_eq!(l.visible(), [1]);
    }

    #[test]
    fn selection_maps_back_to_original_index() {
        let mut l = list();
        "source".chars().for_each(|c| l.push(c));
        assert_eq!(l.selected(), Some(2));
    }

    #[test]
    fn cursor_clamps_and_clear_restores() {
        let mut l = list();
        l.bottom();
        l.down();
        assert_eq!(l.cursor(), 2);
        l.push('m');
        assert_eq!(l.cursor(), 0);
        l.clear();
        assert_eq!(l.visible().len(), 3);
    }

    fn keyed() -> List {
        let key = |name: &str, url: &str, group: u8| SortKeys {
            name: name.into(),
            url: url.into(),
            source: String::new(),
            group,
        };
        List::with_keys(
            vec![
                "origin  https://z/o".into(),
                "zeta  https://a/z".into(),
                "Alpha  https://c/a".into(),
                "mid  https://b/m".into(),
            ],
            vec![
                key("origin", "https://z/o", 0),
                key("zeta", "https://a/z", 1),
                key("Alpha", "https://c/a", 1),
                key("mid", "https://b/m", 1),
            ],
        )
    }

    use Direction::{Asc, Desc};
    use SortColumn::{Label, Source, Url};

    #[test]
    fn a_new_list_is_unsorted_with_the_cursor_on_the_first_column() {
        let l = keyed();
        assert_eq!(l.sort(), None);
        assert_eq!(l.column(), Label);
        assert_eq!(l.columns(), [Label, Url, Source]);
        assert_eq!(l.visible(), [0, 1, 2, 3]);
    }

    #[test]
    fn s_cycles_ascending_descending_none() {
        let mut l = keyed();
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Label, Asc)));
        assert_eq!(l.visible(), [0, 2, 3, 1]);
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Label, Desc)));
        assert_eq!(l.visible(), [0, 1, 3, 2]);
        l.cycle_sort();
        assert_eq!(l.sort(), None);
        assert_eq!(l.visible(), [0, 1, 2, 3]);
    }

    #[test]
    fn moving_the_column_does_not_change_the_sort() {
        let mut l = keyed();
        l.cycle_sort();
        l.move_column(1);
        assert_eq!(l.column(), Url);
        assert_eq!(l.sort(), Some((Label, Asc)));
        assert_eq!(l.visible(), [0, 2, 3, 1]);
    }

    #[test]
    fn s_on_a_new_column_starts_ascending_and_replaces_the_old_sort() {
        let mut l = keyed();
        l.cycle_sort();
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Label, Desc)));
        l.move_column(1);
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Url, Asc)));
        assert_eq!(l.visible(), [0, 1, 3, 2]);
    }

    #[test]
    fn s_back_on_the_first_column_starts_ascending_again() {
        let mut l = keyed();
        l.cycle_sort();
        l.move_column(1);
        l.cycle_sort();
        l.move_column(-1);
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Label, Asc)));
    }

    #[test]
    fn the_column_cursor_stops_at_both_ends() {
        let mut l = keyed();
        l.move_column(-1);
        assert_eq!(l.column(), Label);
        l.move_column(10);
        assert_eq!(l.column(), Source);
        l.move_column(1);
        assert_eq!(l.column(), Source);
    }

    #[test]
    fn tags_order_sorts_by_the_tag_text() {
        let key = |source: &str| SortKeys {
            name: "n".into(),
            url: "u".into(),
            source: source.into(),
            group: 0,
        };
        let mut l = List::with_keys(
            vec!["a".into(), "b".into(), "c".into()],
            vec![key("merge"), key("maintainers"), key("manual")],
        );
        l.move_column(2);
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Source, Asc)));
        assert_eq!(l.visible(), [1, 2, 0]);
    }

    #[test]
    fn the_cursor_follows_its_row_when_the_sort_changes_to_ascending_or_none() {
        let mut l = keyed();
        l.down();
        assert_eq!(l.selected(), Some(1));
        l.cycle_sort();
        assert_eq!(l.selected(), Some(1));
        assert_eq!(l.cursor(), 3);
    }

    #[test]
    fn the_cursor_goes_to_the_top_when_the_sort_turns_descending() {
        let mut l = keyed();
        l.cycle_sort();
        l.bottom();
        l.cycle_sort();
        assert_eq!(l.sort(), Some((Label, Desc)));
        assert_eq!(l.cursor(), 0);
    }

    #[test]
    fn sort_and_filter_compose() {
        let mut l = keyed();
        l.push('a');
        l.cycle_sort();
        assert_eq!(l.visible(), [2, 1]);
    }

    #[test]
    fn a_plain_list_sorts_by_its_text() {
        let mut l = List::new(vec!["b".into(), "a".into()]);
        l.cycle_sort();
        assert_eq!(l.visible(), [1, 0]);
    }

    #[test]
    fn descending_flips_rows_inside_each_group_only() {
        let mut l = keyed();
        l.cycle_sort();
        l.cycle_sort();
        assert_eq!(l.visible(), [0, 1, 3, 2]);
    }

    #[test]
    fn a_list_can_offer_fewer_columns() {
        let mut l = List::new(vec![
            "refs/source/v6.10".into(),
            "refs/source/v6.9".into(),
            "refs/heads/main".into(),
            "HEAD".into(),
        ])
        .with_columns(vec![Label]);
        assert_eq!(l.columns(), [Label]);
        l.move_column(1);
        assert_eq!(l.column(), Label);
        l.cycle_sort();
        assert_eq!(l.visible(), [3, 2, 1, 0]);
        l.cycle_sort();
        assert_eq!(l.visible(), [0, 1, 2, 3]);
    }

    fn numbered(n: usize) -> List {
        List::new((0..n).map(|i| format!("row{i:02}")).collect())
    }

    fn remotes() -> List {
        let row = |n: &str, u: &str, src: &str| {
            (
                vec![n.to_owned(), u.to_owned(), src.to_owned()],
                SortKeys {
                    name: n.into(),
                    url: u.into(),
                    source: src.into(),
                    group: 0,
                },
            )
        };
        let (cells, keys): (Vec<_>, Vec<_>) = [
            row("net", "https://k/net", "maintainers"),
            row("mm", "https://k/mm", "merges"),
            row("usb", "https://k/net-usb", "maintainers"),
        ]
        .into_iter()
        .unzip();
        List::with_cells(cells, keys)
    }

    fn type_into(l: &mut List, text: &str) {
        text.chars().for_each(|c| l.push(c));
    }

    #[test]
    fn a_column_filter_only_looks_at_its_own_column() {
        let mut l = remotes();
        l.move_column(2);
        assert!(l.begin_column_edit());
        type_into(&mut l, "maint");
        assert_eq!(l.visible(), [0, 2]);
        // "net" is in the url of the third row, but this filter is on source.
        l.end_edit();
        l.move_column(-1);
        assert!(l.begin_column_edit());
        type_into(&mut l, "net");
        assert_eq!(l.visible(), [0, 2]);
        l.end_edit();
        l.move_column(-1);
        assert!(l.begin_column_edit());
        type_into(&mut l, "usb");
        assert_eq!(l.visible(), [2]);
    }

    #[test]
    fn search_runs_over_what_the_column_filters_left() {
        let mut l = remotes();
        l.move_column(2);
        l.begin_column_edit();
        type_into(&mut l, "maint");
        l.end_edit();
        type_into(&mut l, "usb");
        assert_eq!(l.visible(), [2]);
        assert_eq!(l.filter(), "usb");
        assert_eq!(l.column_filter(2), "maint");
    }

    #[test]
    fn column_filters_take_not_and_or_like_search() {
        let mut l = remotes();
        l.begin_column_edit();
        type_into(&mut l, "!mm");
        assert_eq!(l.visible(), [0, 2]);
    }

    #[test]
    fn typing_goes_to_the_column_until_the_edit_ends() {
        let mut l = remotes();
        assert_eq!(l.editing_column(), None);
        l.move_column(1);
        l.begin_column_edit();
        assert_eq!(l.editing_column(), Some(1));
        type_into(&mut l, "k");
        l.pop();
        assert_eq!(l.column_filter(1), "");
        l.end_edit();
        type_into(&mut l, "x");
        assert_eq!(l.filter(), "x");
    }

    #[test]
    fn clear_also_empties_the_column_filters() {
        let mut l = remotes();
        l.begin_column_edit();
        type_into(&mut l, "mm");
        l.end_edit();
        l.clear();
        assert_eq!(l.column_filter(0), "");
        assert_eq!(l.visible(), [0, 1, 2]);
    }

    #[test]
    fn a_list_with_one_column_has_no_column_filters() {
        let mut l = List::new(vec!["a".into(), "b".into()]).with_columns(vec![SortColumn::Label]);
        assert!(!l.filterable());
        assert!(!l.begin_column_edit());
        assert_eq!(l.editing_column(), None);
    }

    #[test]
    fn a_bad_regex_in_a_column_filter_is_reported() {
        let mut l = remotes();
        l.toggle_mode();
        l.begin_column_edit();
        l.push('(');
        assert!(l.filter_error().is_some());
        l.pop();
        assert!(l.filter_error().is_none());
    }

    #[test]
    fn clear_drops_the_filter_and_the_sort_but_not_the_column_cursor() {
        let key = |n: &str| SortKeys {
            name: n.into(),
            ..SortKeys::default()
        };
        let mut l = List::with_keys(
            vec!["b".into(), "a".into(), "c".into()],
            vec![key("b"), key("a"), key("c")],
        );
        l.move_column(1);
        l.cycle_sort();
        for c in "a".chars() {
            l.push(c);
        }
        assert!(l.sort().is_some());
        l.clear();
        assert_eq!(l.sort(), None);
        assert_eq!(l.filter(), "");
        assert_eq!(l.column(), SortColumn::Url);
        assert_eq!(l.visible(), [0, 1, 2]);
    }

    #[test]
    fn move_by_clamps_at_both_ends() {
        let mut l = numbered(10);
        l.move_by(4);
        assert_eq!(l.cursor(), 4);
        l.move_by(100);
        assert_eq!(l.cursor(), 9);
        l.move_by(-3);
        assert_eq!(l.cursor(), 6);
        l.move_by(-100);
        assert_eq!(l.cursor(), 0);
    }

    #[test]
    fn goto_takes_a_one_based_row_and_clamps() {
        let mut l = numbered(10);
        l.goto(5);
        assert_eq!(l.cursor(), 4);
        l.goto(0);
        assert_eq!(l.cursor(), 0);
        l.goto(99);
        assert_eq!(l.cursor(), 9);
    }

    #[test]
    fn paging_uses_the_viewport_height() {
        let mut l = numbered(100);
        assert!(l.motion(Action::HalfPageDown, None, 20));
        assert_eq!(l.cursor(), 10);
        assert!(l.motion(Action::PageDown, None, 20));
        assert_eq!(l.cursor(), 29);
        assert!(l.motion(Action::HalfPageUp, None, 20));
        assert_eq!(l.cursor(), 19);
        assert!(l.motion(Action::PageUp, None, 20));
        assert_eq!(l.cursor(), 0);
    }

    #[test]
    fn motion_applies_counts_and_ignores_other_actions() {
        let mut l = numbered(10);
        assert!(l.motion(Action::Down, Some(3), 20));
        assert_eq!(l.cursor(), 3);
        assert!(l.motion(Action::Bottom, Some(2), 20));
        assert_eq!(l.cursor(), 1);
        assert!(l.motion(Action::Top, None, 20));
        assert_eq!(l.cursor(), 0);
        assert!(!l.motion(Action::Search, None, 20));
    }

    #[test]
    fn natural_order_compares_numbers_as_numbers_and_ignores_case() {
        use std::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(natural_cmp("v6.9", "v6.10"), Less);
        assert_eq!(natural_cmp("v6.10", "v6.9"), Greater);
        assert_eq!(natural_cmp("V6.9", "v6.9"), Equal);
        assert_eq!(natural_cmp("a01", "a1"), Equal);
        assert_eq!(natural_cmp("rc2", "rc10"), Less);
        assert_eq!(natural_cmp("a", "ab"), Less);
        assert_eq!(natural_cmp("abc", "abd"), Less);
    }

    #[test]
    fn cell_rows_filter_on_the_whole_row_and_skip_empty_cells() {
        let cells = |a: &str, b: &str, c: &str| vec![a.to_owned(), b.to_owned(), c.to_owned()];
        let mut l = List::with_cells(
            vec![
                cells("origin", "https://h/o", ""),
                cells("net", "https://h/n", "merge"),
            ],
            vec![SortKeys::default(), SortKeys::default()],
        );
        assert_eq!(
            l.visible_labels(),
            ["origin  https://h/o", "net  https://h/n  merge"]
        );
        l.push('m');
        l.push('e');
        l.push('r');
        let rows = l.visible_cells();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 1);
        assert_eq!(rows[0].1, ["net", "https://h/n", "merge"]);
    }

    #[test]
    fn visible_rows_pair_original_index_with_label() {
        let mut l = list();
        "heads".chars().for_each(|c| l.push(c));
        assert_eq!(
            l.visible_rows(),
            [(0, "refs/heads/main"), (1, "refs/heads/Dev")]
        );
    }

    #[test]
    fn empty_result_has_no_selection() {
        let mut l = list();
        "zzz".chars().for_each(|c| l.push(c));
        assert_eq!(l.selected(), None);
    }
}

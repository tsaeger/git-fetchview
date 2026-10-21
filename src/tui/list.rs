// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! A filterable list with a cursor.

use std::cmp::{Ordering, Reverse};

use crate::tui::keys::Action;

/// How the rows are ordered: the column being sorted by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    /// The order the rows were given in.
    #[default]
    Default,
    /// By name, case-insensitive, within each group.
    Label,
    /// By URL, case-insensitive, within each group.
    Url,
    /// By tags, case-insensitive, within each group.
    Tags,
}

impl SortOrder {
    /// Every column, in the order they are shown.
    pub const ALL: [Self; 4] = [Self::Default, Self::Label, Self::Url, Self::Tags];

    /// A short name for the status line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Label => "label",
            Self::Url => "url",
            Self::Tags => "tags",
        }
    }
}

/// What a row is sorted by. Sorting never moves a row out of its group.
#[derive(Debug, Clone, Default)]
pub struct SortKeys {
    /// Name to sort by in label order.
    pub name: String,
    /// URL to sort by in URL order.
    pub url: String,
    /// Tags to sort by in tags order.
    pub tags: String,
    /// Rows sort within their group; lower groups come first.
    pub group: u8,
}

/// Rows of display text plus a search filter and a cursor into the visible rows.
#[derive(Debug, Clone, Default)]
pub struct List {
    labels: Vec<String>,
    cells: Vec<Vec<String>>,
    keys: Vec<SortKeys>,
    sort: SortOrder,
    reversed: bool,
    columns: Vec<SortOrder>,
    filter: String,
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
                tags: String::new(),
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
        let labels = cells
            .iter()
            .map(|row| {
                row.iter()
                    .filter(|c| !c.is_empty())
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .collect();
        Self {
            labels,
            cells,
            keys,
            ..Self::default()
        }
    }

    /// Limit the sort columns to these, in this order. By default every
    /// column is available.
    #[must_use]
    pub fn with_cycle(mut self, columns: Vec<SortOrder>) -> Self {
        self.columns = columns;
        self
    }

    /// The sort columns this list offers, in the order they are shown.
    #[must_use]
    pub fn columns(&self) -> &[SortOrder] {
        if self.columns.is_empty() {
            &SortOrder::ALL
        } else {
            &self.columns
        }
    }

    /// The current sort order.
    #[must_use]
    pub fn sort_order(&self) -> SortOrder {
        self.sort
    }

    /// Whether the order is flipped.
    #[must_use]
    pub fn reversed(&self) -> bool {
        self.reversed
    }

    /// Keep the cursor on the row it was on after the order changed.
    fn follow(&mut self, selected: Option<usize>) {
        self.cursor = selected
            .and_then(|row| self.visible().iter().position(|&i| i == row))
            .unwrap_or(0);
    }

    /// Move the sort `delta` columns along, stopping at either end. The
    /// cursor stays on the same row.
    pub fn shift_sort(&mut self, delta: isize) {
        let selected = self.selected();
        let columns = self.columns();
        let at = columns.iter().position(|&c| c == self.sort).unwrap_or(0);
        let to = at.saturating_add_signed(delta).min(columns.len() - 1);
        self.sort = columns[to];
        self.follow(selected);
    }

    /// Flip the order inside each group and go to the top of the list.
    pub fn toggle_reverse(&mut self) {
        self.reversed = !self.reversed;
        self.cursor = 0;
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
            .filter(|&i| matches(&self.labels[i], &self.filter))
            .collect();
        let reversed = self.reversed;
        let keys = &self.keys;
        let by_key = |a: usize, b: usize, key: fn(&SortKeys) -> &str| {
            let order = natural_cmp(key(&keys[a]), key(&keys[b]))
                .then_with(|| natural_cmp(&keys[a].name, &keys[b].name))
                .then_with(|| a.cmp(&b));
            keys[a]
                .group
                .cmp(&keys[b].group)
                .then(if reversed { order.reverse() } else { order })
        };
        match self.sort {
            SortOrder::Default if reversed => {
                rows.sort_by_key(|&i| (keys[i].group, Reverse(i)));
            }
            SortOrder::Default => {}
            SortOrder::Label => rows.sort_by(|&a, &b| by_key(a, b, |k| &k.name)),
            SortOrder::Url => rows.sort_by(|&a, &b| by_key(a, b, |k| &k.url)),
            SortOrder::Tags => rows.sort_by(|&a, &b| by_key(a, b, |k| &k.tags)),
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

    /// Append to the filter.
    pub fn push(&mut self, c: char) {
        self.filter.push(c);
        self.cursor = 0;
    }

    /// Remove the last filter character.
    pub fn pop(&mut self) {
        self.filter.pop();
        self.cursor = 0;
    }

    /// Clear the filter.
    pub fn clear(&mut self) {
        self.filter.clear();
        self.cursor = 0;
    }
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

/// Case-insensitive substring match. Regex support will slot in here.
fn matches(label: &str, filter: &str) -> bool {
    filter.is_empty() || label.to_lowercase().contains(&filter.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keys::Action;

    fn list() -> List {
        List::new(vec![
            "refs/heads/main".into(),
            "refs/heads/Dev".into(),
            "refs/tags/v1".into(),
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
        "tags".chars().for_each(|c| l.push(c));
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
            tags: String::new(),
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

    #[test]
    fn label_order_sorts_each_group_case_insensitively() {
        let mut l = keyed();
        l.shift_sort(1);
        assert_eq!(l.sort_order(), SortOrder::Label);
        assert_eq!(l.visible(), [0, 2, 3, 1]);
    }

    #[test]
    fn url_order_sorts_each_group_by_url() {
        let mut l = keyed();
        l.shift_sort(1);
        l.shift_sort(1);
        assert_eq!(l.sort_order(), SortOrder::Url);
        assert_eq!(l.visible(), [0, 1, 3, 2]);
    }

    #[test]
    fn moving_back_to_the_first_column_restores_the_original_order() {
        let mut l = keyed();
        l.shift_sort(1);
        l.shift_sort(1);
        l.shift_sort(-1);
        l.shift_sort(-1);
        assert_eq!(l.sort_order(), SortOrder::Default);
        assert_eq!(l.visible(), [0, 1, 2, 3]);
    }

    #[test]
    fn shifting_stops_at_the_first_and_last_column() {
        let mut l = keyed();
        l.shift_sort(-1);
        assert_eq!(l.sort_order(), SortOrder::Default);
        l.shift_sort(10);
        assert_eq!(l.sort_order(), SortOrder::Tags);
        l.shift_sort(1);
        assert_eq!(l.sort_order(), SortOrder::Tags);
        assert_eq!(
            l.columns(),
            [
                SortOrder::Default,
                SortOrder::Label,
                SortOrder::Url,
                SortOrder::Tags
            ]
        );
    }

    #[test]
    fn tags_order_sorts_by_the_tag_text() {
        let key = |tags: &str| SortKeys {
            name: "n".into(),
            url: "u".into(),
            tags: tags.into(),
            group: 0,
        };
        let mut l = List::with_keys(
            vec!["a".into(), "b".into(), "c".into()],
            vec![key("merge"), key("maintainers"), key("manual")],
        );
        l.shift_sort(3);
        assert_eq!(l.sort_order(), SortOrder::Tags);
        assert_eq!(l.visible(), [1, 2, 0]);
    }

    #[test]
    fn the_cursor_stays_on_the_same_row_after_a_sort() {
        let mut l = keyed();
        l.down();
        assert_eq!(l.selected(), Some(1));
        l.shift_sort(1);
        assert_eq!(l.selected(), Some(1));
        assert_eq!(l.cursor(), 3);
    }

    #[test]
    fn sort_and_filter_compose() {
        let mut l = keyed();
        l.push('a');
        l.shift_sort(1);
        assert_eq!(l.visible(), [2, 1]);
    }

    #[test]
    fn a_plain_list_sorts_by_its_text() {
        let mut l = List::new(vec!["b".into(), "a".into()]);
        l.shift_sort(1);
        assert_eq!(l.visible(), [1, 0]);
    }

    fn numbered(n: usize) -> List {
        List::new((0..n).map(|i| format!("row{i:02}")).collect())
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
    fn reversing_flips_rows_inside_each_group_only() {
        let mut l = keyed();
        l.shift_sort(1);
        assert_eq!(l.visible(), [0, 2, 3, 1]);
        l.toggle_reverse();
        assert!(l.reversed());
        assert_eq!(l.visible(), [0, 1, 3, 2]);
        l.toggle_reverse();
        assert_eq!(l.visible(), [0, 2, 3, 1]);
    }

    #[test]
    fn the_default_order_can_be_reversed_too() {
        let mut l = keyed();
        l.toggle_reverse();
        assert_eq!(l.sort_order(), SortOrder::Default);
        assert_eq!(l.visible(), [0, 3, 2, 1]);
    }

    #[test]
    fn reversing_puts_the_cursor_at_the_top() {
        let mut l = keyed();
        l.down();
        l.down();
        assert_eq!(l.cursor(), 2);
        l.toggle_reverse();
        assert_eq!(l.cursor(), 0);
        l.bottom();
        l.toggle_reverse();
        assert_eq!(l.cursor(), 0);
    }

    #[test]
    fn a_custom_set_of_columns_limits_where_shifting_goes() {
        let mut l = List::new(vec![
            "refs/tags/v6.10".into(),
            "refs/tags/v6.9".into(),
            "refs/heads/main".into(),
            "HEAD".into(),
        ])
        .with_cycle(vec![SortOrder::Default, SortOrder::Label]);
        assert_eq!(l.visible(), [0, 1, 2, 3]);
        l.shift_sort(1);
        assert_eq!(l.sort_order(), SortOrder::Label);
        assert_eq!(l.visible(), [3, 2, 1, 0]);
        l.toggle_reverse();
        assert_eq!(l.visible(), [0, 1, 2, 3]);
        l.shift_sort(1);
        assert_eq!(l.sort_order(), SortOrder::Label);
        l.shift_sort(-1);
        assert_eq!(l.sort_order(), SortOrder::Default);
        assert_eq!(l.visible(), [3, 2, 1, 0]);
    }

    #[test]
    fn sort_orders_have_names() {
        assert_eq!(SortOrder::Default.name(), "default");
        assert_eq!(SortOrder::Label.name(), "label");
        assert_eq!(SortOrder::Url.name(), "url");
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

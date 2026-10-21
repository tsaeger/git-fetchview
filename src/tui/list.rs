// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! A filterable list with a cursor.

/// How the rows are ordered. `s` cycles through them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    /// The order the rows were given in.
    #[default]
    Default,
    /// By name, case-insensitive, within each group.
    Label,
    /// By URL, case-insensitive, within each group.
    Url,
}

impl SortOrder {
    /// The next order in the cycle.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Label,
            Self::Label => Self::Url,
            Self::Url => Self::Default,
        }
    }

    /// A short name for the status line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Label => "label",
            Self::Url => "url",
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
    /// Rows sort within their group; lower groups come first.
    pub group: u8,
}

/// Rows of display text plus a search filter and a cursor into the visible rows.
#[derive(Debug, Clone, Default)]
pub struct List {
    labels: Vec<String>,
    keys: Vec<SortKeys>,
    sort: SortOrder,
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
                group: 0,
            })
            .collect();
        Self::with_keys(labels, keys)
    }

    /// Build a list whose rows sort by the given keys.
    #[must_use]
    pub fn with_keys(labels: Vec<String>, keys: Vec<SortKeys>) -> Self {
        debug_assert_eq!(labels.len(), keys.len());
        Self {
            labels,
            keys,
            ..Self::default()
        }
    }

    /// The current sort order.
    #[must_use]
    pub fn sort_order(&self) -> SortOrder {
        self.sort
    }

    /// Move to the next sort order. The cursor stays on the same row.
    pub fn cycle_sort(&mut self) {
        let selected = self.selected();
        self.sort = self.sort.next();
        self.cursor = selected
            .and_then(|row| self.visible().iter().position(|&i| i == row))
            .unwrap_or(0);
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
        match self.sort {
            SortOrder::Default => {}
            SortOrder::Label => rows
                .sort_by_cached_key(|&i| (self.keys[i].group, self.keys[i].name.to_lowercase(), i)),
            SortOrder::Url => rows.sort_by_cached_key(|&i| {
                (
                    self.keys[i].group,
                    self.keys[i].url.to_lowercase(),
                    self.keys[i].name.to_lowercase(),
                    i,
                )
            }),
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

/// Case-insensitive substring match. Regex support will slot in here.
fn matches(label: &str, filter: &str) -> bool {
    filter.is_empty() || label.to_lowercase().contains(&filter.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        l.cycle_sort();
        assert_eq!(l.sort_order(), SortOrder::Label);
        assert_eq!(l.visible(), [0, 2, 3, 1]);
    }

    #[test]
    fn url_order_sorts_each_group_by_url() {
        let mut l = keyed();
        l.cycle_sort();
        l.cycle_sort();
        assert_eq!(l.sort_order(), SortOrder::Url);
        assert_eq!(l.visible(), [0, 1, 3, 2]);
    }

    #[test]
    fn a_third_press_restores_the_original_order() {
        let mut l = keyed();
        for _ in 0..3 {
            l.cycle_sort();
        }
        assert_eq!(l.sort_order(), SortOrder::Default);
        assert_eq!(l.visible(), [0, 1, 2, 3]);
    }

    #[test]
    fn the_cursor_stays_on_the_same_row_after_a_sort() {
        let mut l = keyed();
        l.down();
        assert_eq!(l.selected(), Some(1));
        l.cycle_sort();
        assert_eq!(l.selected(), Some(1));
        assert_eq!(l.cursor(), 3);
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
    fn sort_orders_have_names() {
        assert_eq!(SortOrder::Default.name(), "default");
        assert_eq!(SortOrder::Label.name(), "label");
        assert_eq!(SortOrder::Url.name(), "url");
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

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! A filterable list with a cursor.

/// Rows of display text plus a search filter and a cursor into the visible rows.
#[derive(Debug, Clone, Default)]
pub struct List {
    labels: Vec<String>,
    filter: String,
    cursor: usize,
}

impl List {
    /// Build a list from display labels.
    #[must_use]
    pub fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            ..Self::default()
        }
    }

    /// The current filter text.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Indices (into the original labels) of rows matching the filter.
    #[must_use]
    pub fn visible(&self) -> Vec<usize> {
        (0..self.labels.len())
            .filter(|&i| matches(&self.labels[i], &self.filter))
            .collect()
    }

    /// Labels of the visible rows, in order.
    #[must_use]
    pub fn visible_labels(&self) -> Vec<&str> {
        self.visible()
            .into_iter()
            .map(|i| self.labels[i].as_str())
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

    #[test]
    fn empty_result_has_no_selection() {
        let mut l = list();
        "zzz".chars().for_each(|c| l.push(c));
        assert_eq!(l.selected(), None);
    }
}

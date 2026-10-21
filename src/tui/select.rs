// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Multi-select list used by `add`.

use std::collections::BTreeSet;

use crate::tui::keys::Action;
use crate::tui::list::{List, SortKeys};

/// One selectable row.
#[derive(Debug, Clone)]
pub struct Row {
    /// Text to show and filter on.
    pub label: String,
    /// Name to sort by in label order.
    pub name: String,
    /// URL to sort by in URL order.
    pub url: String,
    /// Already present: shown, but cannot be checked.
    pub have: bool,
}

/// What the event loop should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectFlow {
    /// Keep going.
    Continue,
    /// Leave without choosing anything.
    Quit,
    /// Confirmed: the checked rows, as original indexes in order.
    Done(Vec<usize>),
}

/// Multi-select state.
#[derive(Debug)]
pub struct Select {
    list: List,
    have: Vec<bool>,
    checked: BTreeSet<usize>,
    typing: bool,
}

impl Select {
    /// Build from rows.
    #[must_use]
    pub fn new(rows: Vec<Row>) -> Self {
        let have = rows.iter().map(|r| r.have).collect();
        let keys = rows
            .iter()
            .map(|r| SortKeys {
                name: r.name.clone(),
                url: r.url.clone(),
                // Rows already present stay below the new ones.
                group: u8::from(r.have),
            })
            .collect();
        let list = List::with_keys(rows.into_iter().map(|r| r.label).collect(), keys);
        Self {
            list,
            have,
            checked: BTreeSet::new(),
            typing: false,
        }
    }

    /// The underlying list.
    #[must_use]
    pub fn list(&self) -> &List {
        &self.list
    }

    /// True while a search is being typed.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.typing
    }

    /// Whether row `i` (original index) is checked.
    #[must_use]
    pub fn is_checked(&self, i: usize) -> bool {
        self.checked.contains(&i)
    }

    /// Whether row `i` is already present.
    #[must_use]
    pub fn is_have(&self, i: usize) -> bool {
        self.have[i]
    }

    /// How many rows are checked.
    #[must_use]
    pub fn checked_count(&self) -> usize {
        self.checked.len()
    }

    /// How many rows can be checked at all.
    #[must_use]
    pub fn selectable_count(&self) -> usize {
        self.have.iter().filter(|h| !**h).count()
    }

    /// Apply one action.
    pub fn update(&mut self, action: Action) -> SelectFlow {
        if action == Action::Quit {
            return SelectFlow::Quit;
        }
        if self.typing {
            self.update_typing(action);
            return SelectFlow::Continue;
        }
        match action {
            Action::Down => self.list.down(),
            Action::Up => self.list.up(),
            Action::Top => self.list.top(),
            Action::Bottom => self.list.bottom(),
            Action::Search => self.typing = true,
            Action::Clear => self.list.clear(),
            Action::Sort => self.list.cycle_sort(),
            Action::Toggle => {
                if let Some(i) = self.list.selected() {
                    self.toggle(i);
                }
            }
            Action::ToggleAll => self.toggle_all(),
            Action::Enter => return SelectFlow::Done(self.checked.iter().copied().collect()),
            Action::Back | Action::Quit | Action::Char(_) | Action::Backspace => {}
        }
        SelectFlow::Continue
    }

    fn update_typing(&mut self, action: Action) {
        match action {
            Action::Char(c) => self.list.push(c),
            Action::Backspace => self.list.pop(),
            Action::Enter => self.typing = false,
            Action::Back => {
                self.list.clear();
                self.typing = false;
            }
            _ => {}
        }
    }

    fn toggle(&mut self, i: usize) {
        if self.have[i] {
            return;
        }
        if !self.checked.remove(&i) {
            self.checked.insert(i);
        }
    }

    fn toggle_all(&mut self) {
        let visible: Vec<usize> = self
            .list
            .visible()
            .into_iter()
            .filter(|&i| !self.have[i])
            .collect();
        let all_checked = !visible.is_empty() && visible.iter().all(|i| self.checked.contains(i));
        for i in visible {
            if all_checked {
                self.checked.remove(&i);
            } else {
                self.checked.insert(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        vec![
            Row {
                label: "net  https://h/net".into(),
                name: "net".into(),
                url: "https://h/net".into(),
                have: false,
            },
            Row {
                label: "mm  https://h/mm".into(),
                name: "mm".into(),
                url: "https://h/mm".into(),
                have: true,
            },
            Row {
                label: "usb  https://h/usb".into(),
                name: "usb".into(),
                url: "https://h/usb".into(),
                have: false,
            },
        ]
    }

    #[test]
    fn space_toggles_the_row_under_the_cursor() {
        let mut s = Select::new(rows());
        s.update(Action::Toggle);
        assert!(s.is_checked(0));
        s.update(Action::Toggle);
        assert!(!s.is_checked(0));
    }

    #[test]
    fn rows_already_present_cannot_be_checked() {
        let mut s = Select::new(rows());
        s.update(Action::Down);
        s.update(Action::Toggle);
        assert!(!s.is_checked(1));
        assert_eq!(s.checked_count(), 0);
        assert_eq!(s.selectable_count(), 2);
    }

    #[test]
    fn a_checks_all_visible_then_unchecks_them() {
        let mut s = Select::new(rows());
        s.update(Action::ToggleAll);
        assert_eq!(
            (s.is_checked(0), s.is_checked(1), s.is_checked(2)),
            (true, false, true)
        );
        s.update(Action::ToggleAll);
        assert_eq!(s.checked_count(), 0);
    }

    #[test]
    fn a_only_touches_rows_the_filter_shows() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        s.update(Action::Char('u'));
        s.update(Action::Char('s'));
        s.update(Action::Char('b'));
        s.update(Action::Enter);
        s.update(Action::ToggleAll);
        assert!(s.is_checked(2));
        assert!(!s.is_checked(0));
    }

    #[test]
    fn enter_confirms_with_checked_indexes_in_order() {
        let mut s = Select::new(rows());
        s.update(Action::ToggleAll);
        assert_eq!(s.update(Action::Enter), SelectFlow::Done(vec![0, 2]));
    }

    #[test]
    fn enter_while_typing_only_confirms_the_search() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        assert_eq!(s.update(Action::Enter), SelectFlow::Continue);
        assert!(!s.typing());
    }

    #[test]
    fn q_quits_and_esc_does_nothing_at_the_top_level() {
        let mut s = Select::new(rows());
        assert_eq!(s.update(Action::Back), SelectFlow::Continue);
        assert_eq!(s.update(Action::Quit), SelectFlow::Quit);
    }

    #[test]
    fn esc_while_typing_abandons_the_search() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        s.update(Action::Char('x'));
        s.update(Action::Back);
        assert!(!s.typing());
        assert_eq!(s.list().filter(), "");
    }

    #[test]
    fn s_sorts_new_rows_and_keeps_present_rows_last() {
        let mut s = Select::new(rows());
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [0, 2, 1]);
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [0, 2, 1]);
    }

    #[test]
    fn checks_survive_a_sort() {
        let mut s = Select::new(rows());
        s.update(Action::Down);
        s.update(Action::Down);
        s.update(Action::Toggle);
        s.update(Action::Sort);
        s.update(Action::Sort);
        assert!(s.is_checked(2));
        assert_eq!(s.list().selected(), Some(2));
    }

    #[test]
    fn checks_survive_a_filter_change() {
        let mut s = Select::new(rows());
        s.update(Action::Toggle);
        s.update(Action::Search);
        s.update(Action::Char('u'));
        s.update(Action::Enter);
        s.update(Action::Clear);
        assert!(s.is_checked(0));
    }
}

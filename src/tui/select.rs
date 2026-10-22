// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Multi-select list used by `add`.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use crate::tui::keys::{Action, Command, Mode};
use crate::tui::list::{List, SortKeys};

/// One selectable row.
#[derive(Debug, Clone)]
pub struct Row {
    /// Columns to show; filtering matches all of them.
    pub cells: Vec<String>,
    /// Name to sort by in label order.
    pub name: String,
    /// URL to sort by in URL order.
    pub url: String,
    /// Source to sort by in source order.
    pub source: String,
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
    viewport: usize,
    /// Visible position where the highlight started; the cursor is its other end.
    anchor: Option<usize>,
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
                source: r.source.clone(),
                // Rows already present stay below the new ones.
                group: u8::from(r.have),
            })
            .collect();
        let list = List::with_cells(rows.into_iter().map(|r| r.cells).collect(), keys);
        Self {
            list,
            have,
            checked: BTreeSet::new(),
            typing: false,
            viewport: 10,
            anchor: None,
        }
    }

    /// The underlying list.
    #[must_use]
    pub fn list(&self) -> &List {
        &self.list
    }

    /// Which keymap is in force.
    #[must_use]
    pub fn mode(&self) -> Mode {
        if self.typing {
            Mode::Typing
        } else {
            Mode::Normal
        }
    }

    /// True while a search is being typed.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.typing
    }

    /// The highlighted rows as visible positions, if a range is being extended.
    /// Highlighting only marks rows; `Toggle` is what changes the selection.
    #[must_use]
    pub fn highlight(&self) -> Option<RangeInclusive<usize>> {
        let anchor = self.anchor?;
        let cursor = self.list.cursor();
        Some(anchor.min(cursor)..=anchor.max(cursor))
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

    /// Tell the list how many rows fit on screen, for paging.
    pub fn set_viewport(&mut self, rows: usize) {
        self.viewport = rows.max(1);
    }

    /// Apply one action.
    pub fn update(&mut self, action: Action) -> SelectFlow {
        self.update_with(Command::from(action))
    }

    /// Apply one action with the count typed before it.
    pub fn update_with(&mut self, command: Command) -> SelectFlow {
        let Command { action, count } = command;
        if matches!(action, Action::Quit | Action::Interrupt) {
            return SelectFlow::Quit;
        }
        if self.typing {
            self.update_typing(action);
            return SelectFlow::Continue;
        }
        // Only extending and toggling keep a highlight alive; anything else
        // (a plain move, Esc, a search) drops it.
        if !matches!(
            action,
            Action::ExtendDown | Action::ExtendUp | Action::Toggle
        ) {
            self.anchor = None;
        }
        if self.list.motion(action, count, self.viewport) {
            return SelectFlow::Continue;
        }
        match action {
            Action::Search => self.typing = true,
            Action::Clear => self.list.clear(),
            Action::Sort => self.list.cycle_sort(),
            Action::ColumnPrev => self.list.move_column(-1),
            Action::ColumnNext => self.list.move_column(1),
            Action::ExtendDown | Action::ExtendUp => {
                let motion = if action == Action::ExtendDown {
                    Action::Down
                } else {
                    Action::Up
                };
                self.anchor.get_or_insert(self.list.cursor());
                self.list.motion(motion, count, self.viewport);
            }
            Action::Toggle => self.toggle_here(),
            Action::ToggleAll => self.toggle_all(),
            Action::Enter => return SelectFlow::Done(self.checked.iter().copied().collect()),
            Action::Back
            | Action::Quit
            | Action::Interrupt
            | Action::Delete
            | Action::New
            | Action::Promote
            | Action::Open
            | Action::Scan
            | Action::ShowHidden
            | Action::ToggleRegex
            | Action::Confirm
            | Action::Char(_)
            | Action::Backspace
            | Action::Down
            | Action::Up
            | Action::Top
            | Action::Bottom
            | Action::HalfPageDown
            | Action::HalfPageUp
            | Action::PageDown
            | Action::PageUp => {}
        }
        SelectFlow::Continue
    }

    fn update_typing(&mut self, action: Action) {
        match action {
            Action::Char(c) => self.list.push(c),
            Action::Backspace => self.list.pop(),
            Action::ToggleRegex => self.list.toggle_mode(),
            // Both leave the box. The text stays, so the filter stays.
            Action::Enter | Action::Back => self.typing = false,
            _ => {}
        }
    }

    /// Space: flip the row under the cursor and move on, or, with a highlight,
    /// set every highlighted row the opposite of the row it started on.
    fn toggle_here(&mut self) {
        let Some(range) = self.highlight() else {
            if let Some(i) = self.list.selected() {
                self.toggle(i);
            }
            self.list.down();
            return;
        };
        let visible = self.list.visible();
        // A row you already have is never checked, so an anchor on one selects.
        let on = visible
            .get(self.anchor.unwrap_or(0))
            .is_none_or(|a| !self.checked.contains(a));
        self.anchor = None;
        for &i in visible.iter().take(*range.end() + 1).skip(*range.start()) {
            if self.have[i] {
                continue;
            }
            if on {
                self.checked.insert(i);
            } else {
                self.checked.remove(&i);
            }
        }
        let past = isize::try_from(*range.end() + 1).unwrap_or(isize::MAX);
        let here = isize::try_from(self.list.cursor()).unwrap_or(0);
        self.list.move_by(past - here);
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
    use crate::tui::keys::{Command, Mode};

    fn rows() -> Vec<Row> {
        vec![
            Row {
                cells: vec!["net".into(), "https://h/net".into(), String::new()],
                source: String::new(),
                name: "net".into(),
                url: "https://h/net".into(),
                have: false,
            },
            Row {
                cells: vec!["mm".into(), "https://h/mm".into(), String::new()],
                source: String::new(),
                name: "mm".into(),
                url: "https://h/mm".into(),
                have: true,
            },
            Row {
                cells: vec!["usb".into(), "https://h/usb".into(), String::new()],
                source: String::new(),
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
        s.update(Action::Up);
        s.update(Action::Toggle);
        assert!(!s.is_checked(0));
    }

    fn rows5() -> Vec<Row> {
        (0..5)
            .map(|i| Row {
                cells: vec![format!("r{i}")],
                source: String::new(),
                name: format!("r{i}"),
                url: String::new(),
                have: false,
            })
            .collect()
    }

    #[test]
    fn space_selects_and_advances_and_deselects_and_advances() {
        let mut s = Select::new(rows5());
        s.update(Action::Toggle);
        assert!(s.is_checked(0));
        assert_eq!(s.list().cursor(), 1);
        s.update(Action::Up);
        s.update(Action::Toggle);
        assert!(!s.is_checked(0));
        assert_eq!(s.list().cursor(), 1);
    }

    #[test]
    fn space_on_the_last_row_stays_there() {
        let mut s = Select::new(rows5());
        s.update_with(Command {
            action: Action::Bottom,
            count: None,
        });
        s.update(Action::Toggle);
        assert!(s.is_checked(4));
        assert_eq!(s.list().cursor(), 4);
    }

    #[test]
    fn extending_highlights_without_selecting() {
        let mut s = Select::new(rows5());
        s.update(Action::Down);
        s.update(Action::ExtendDown);
        s.update(Action::ExtendDown);
        assert_eq!(s.highlight(), Some(1..=3));
        assert_eq!(s.checked_count(), 0);
        s.update(Action::ExtendUp);
        assert_eq!(s.highlight(), Some(1..=2));
    }

    #[test]
    fn extending_up_past_the_start_highlights_above_it() {
        let mut s = Select::new(rows5());
        s.update(Action::Down);
        s.update(Action::Down);
        s.update(Action::ExtendUp);
        s.update(Action::ExtendUp);
        assert_eq!(s.highlight(), Some(0..=2));
    }

    #[test]
    fn space_selects_the_highlight_when_the_anchor_is_unselected() {
        let mut s = Select::new(rows5());
        s.update(Action::ExtendDown);
        s.update(Action::ExtendDown);
        s.update(Action::Toggle);
        assert_eq!(s.highlight(), None);
        assert_eq!(
            (0..5).map(|i| s.is_checked(i)).collect::<Vec<_>>(),
            [true, true, true, false, false]
        );
        assert_eq!(s.list().cursor(), 3);
    }

    #[test]
    fn space_deselects_the_highlight_when_the_anchor_is_selected() {
        let mut s = Select::new(rows5());
        s.update(Action::ToggleAll);
        s.update(Action::Down);
        s.update(Action::ExtendDown);
        s.update(Action::ExtendDown);
        s.update(Action::Toggle);
        assert_eq!(
            (0..5).map(|i| s.is_checked(i)).collect::<Vec<_>>(),
            [true, false, false, false, true]
        );
        assert_eq!(s.list().cursor(), 4);
    }

    #[test]
    fn an_upward_highlight_toggles_by_its_anchor_too() {
        let mut s = Select::new(rows5());
        s.update(Action::Bottom);
        s.update(Action::ExtendUp);
        s.update(Action::ExtendUp);
        s.update(Action::Toggle);
        assert_eq!(s.checked_count(), 3);
        assert!(s.is_checked(2) && s.is_checked(3) && s.is_checked(4));
        // Nothing below the range, so the cursor stays at the end.
        assert_eq!(s.list().cursor(), 4);
    }

    #[test]
    fn a_plain_move_or_esc_drops_the_highlight() {
        for drop in [Action::Down, Action::Up, Action::Back, Action::Search] {
            let mut s = Select::new(rows5());
            s.update(Action::ExtendDown);
            s.update(drop);
            assert_eq!(s.highlight(), None, "{drop:?}");
            assert_eq!(s.checked_count(), 0);
        }
    }

    #[test]
    fn the_highlight_skips_rows_already_present() {
        let mut s = Select::new(rows());
        s.update(Action::ExtendDown);
        s.update(Action::ExtendDown);
        s.update(Action::Toggle);
        assert!(s.is_checked(0) && s.is_checked(2));
        assert!(!s.is_checked(1));
    }

    #[test]
    fn a_count_extends_by_that_many_rows() {
        let mut s = Select::new(rows5());
        s.update_with(Command {
            action: Action::ExtendDown,
            count: Some(3),
        });
        assert_eq!(s.highlight(), Some(0..=3));
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
    fn esc_while_typing_leaves_the_box_and_keeps_the_text() {
        let mut s = Select::new(rows());
        s.update(Action::Search);
        s.update(Action::Char('x'));
        s.update(Action::Back);
        assert!(!s.typing());
        assert_eq!(s.list().filter(), "x");
    }

    #[test]
    fn sorting_has_no_mode_and_enter_still_confirms() {
        let mut s = Select::new(rows());
        s.update(Action::Sort);
        assert_eq!(s.mode(), Mode::Normal);
        assert_eq!(s.update(Action::Enter), SelectFlow::Done(Vec::new()));
    }

    #[test]
    fn counts_and_paging_move_the_cursor() {
        let mut s = Select::new(rows());
        s.set_viewport(2);
        s.update_with(Command {
            action: Action::Bottom,
            count: None,
        });
        assert_eq!(s.list().cursor(), 2);
        s.update_with(Command {
            action: Action::Top,
            count: Some(2),
        });
        assert_eq!(s.list().cursor(), 1);
        s.update_with(Command {
            action: Action::PageUp,
            count: None,
        });
        assert_eq!(s.list().cursor(), 0);
    }

    #[test]
    fn s_sorts_new_rows_and_keeps_present_rows_last() {
        let mut s = Select::new(rows());
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [0, 2, 1]);
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [2, 0, 1]);
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [0, 1, 2]);
    }

    #[test]
    fn the_column_keys_move_the_cursor_and_s_sorts_that_column() {
        let mut s = Select::new(rows());
        s.update(Action::ColumnNext);
        assert_eq!(s.list().sort(), None);
        s.update(Action::Sort);
        assert_eq!(s.list().visible(), [0, 2, 1]);
    }

    #[test]
    fn checks_survive_a_descending_sort_and_the_cursor_goes_to_the_top() {
        let mut s = Select::new(rows());
        s.update(Action::Toggle);
        s.update(Action::Sort);
        s.update(Action::Sort);
        assert!(s.is_checked(0));
        assert_eq!(s.list().cursor(), 0);
        assert_eq!(s.list().selected(), Some(2));
    }

    #[test]
    fn checks_survive_a_sort() {
        let mut s = Select::new(rows());
        s.update(Action::Down);
        s.update(Action::Down);
        s.update(Action::Toggle);
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

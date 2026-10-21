// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The keymap. One place, so every level behaves the same.
//!
//! | key                     | action                              |
//! |-------------------------|-------------------------------------|
//! | `j` / `Down`            | move down                           |
//! | `k` / `Up`              | move up                             |
//! | `g` / `G`               | top / bottom                        |
//! | `l` / `Right` / `Enter` | drop in (select, go to next level)  |
//! | `h` / `Left` / `Esc`    | drop out (never exits at top level) |
//! | `/`                     | search / filter                     |
//! | `c`                     | clear search / filter               |
//! | `Space`                 | toggle a row (multi-select)         |
//! | `a`                     | toggle all visible rows (multi-select) |
//! | `q` / `Ctrl-c`          | quit, from any level                |
//!
//! While typing a search, characters are text; `Enter` confirms the filter
//! and `Esc` abandons it.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What a key press means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move the cursor down.
    Down,
    /// Move the cursor up.
    Up,
    /// Jump to the first row.
    Top,
    /// Jump to the last row.
    Bottom,
    /// Drop in: select and go to the next level.
    Enter,
    /// Drop out one level (or abandon a search being typed).
    Back,
    /// Quit.
    Quit,
    /// Start typing a search.
    Search,
    /// Clear the search / filter.
    Clear,
    /// Toggle the row under the cursor (multi-select).
    Toggle,
    /// Toggle every visible row (multi-select).
    ToggleAll,
    /// A character typed into the search.
    Char(char),
    /// Delete the last search character.
    Backspace,
}

/// Translate a key press; `typing` is true while a search is being entered.
#[must_use]
#[allow(clippy::match_same_arms)] // arms differ by `typing`, which is the point
pub fn map(key: KeyEvent, typing: bool) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    match (typing, key.code) {
        (_, KeyCode::Enter) => Some(Action::Enter),
        (_, KeyCode::Esc) => Some(Action::Back),
        (true, KeyCode::Backspace) => Some(Action::Backspace),
        (true, KeyCode::Char(c)) => Some(Action::Char(c)),
        (true, _) => None,
        (false, KeyCode::Char('j') | KeyCode::Down) => Some(Action::Down),
        (false, KeyCode::Char('k') | KeyCode::Up) => Some(Action::Up),
        (false, KeyCode::Char('g') | KeyCode::Home) => Some(Action::Top),
        (false, KeyCode::Char('G') | KeyCode::End) => Some(Action::Bottom),
        (false, KeyCode::Char('l') | KeyCode::Right) => Some(Action::Enter),
        (false, KeyCode::Char('h') | KeyCode::Left) => Some(Action::Back),
        (false, KeyCode::Char(' ')) => Some(Action::Toggle),
        (false, KeyCode::Char('a')) => Some(Action::ToggleAll),
        (false, KeyCode::Char('/')) => Some(Action::Search),
        (false, KeyCode::Char('c')) => Some(Action::Clear),
        (false, KeyCode::Char('q')) => Some(Action::Quit),
        (false, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn hjkl_and_arrows_agree() {
        for (a, b) in [
            (KeyCode::Char('j'), KeyCode::Down),
            (KeyCode::Char('k'), KeyCode::Up),
            (KeyCode::Char('h'), KeyCode::Left),
            (KeyCode::Char('l'), KeyCode::Right),
        ] {
            assert_eq!(map(key(a), false), map(key(b), false));
        }
    }

    #[test]
    fn q_is_text_while_typing_but_quits_otherwise() {
        assert_eq!(map(key(KeyCode::Char('q')), true), Some(Action::Char('q')));
        assert_eq!(map(key(KeyCode::Char('q')), false), Some(Action::Quit));
    }

    #[test]
    fn space_and_a_toggle_but_only_when_not_typing() {
        assert_eq!(map(key(KeyCode::Char(' ')), false), Some(Action::Toggle));
        assert_eq!(map(key(KeyCode::Char('a')), false), Some(Action::ToggleAll));
        assert_eq!(map(key(KeyCode::Char(' ')), true), Some(Action::Char(' ')));
        assert_eq!(map(key(KeyCode::Char('a')), true), Some(Action::Char('a')));
    }

    #[test]
    fn ctrl_c_always_quits() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map(k, true), Some(Action::Quit));
        assert_eq!(map(k, false), Some(Action::Quit));
    }
}

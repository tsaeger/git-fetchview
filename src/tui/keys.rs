// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The keymap. One place, so every level behaves the same.
//!
//! | key                     | action                              |
//! |-------------------------|-------------------------------------|
//! | `j` / `Down`            | move down                           |
//! | `k` / `Up`              | move up                             |
//! | `gg` / `G`              | top / bottom (`5gg`, `5G`: row 5)   |
//! | `5j` / `5k`             | move 5 rows (a count works on moves) |
//! | `Ctrl-d` / `Ctrl-u`     | half a page down / up               |
//! | `Ctrl-f` / `Ctrl-b`     | a page down / up (also PageDown/Up) |
//! | `l` / `Right` / `Enter` | drop in (select, go to next level)  |
//! | `h` / `Left` / `Esc`    | drop out (never exits at top level) |
//! | `/`                     | search / filter                     |
//! | `c`                     | clear search / filter               |
//! | `s`                     | cycle the sort order                |
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
    /// Move down half a page.
    HalfPageDown,
    /// Move up half a page.
    HalfPageUp,
    /// Move down a page.
    PageDown,
    /// Move up a page.
    PageUp,
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
    /// Cycle the sort order.
    Sort,
    /// Toggle the row under the cursor (multi-select).
    Toggle,
    /// Toggle every visible row (multi-select).
    ToggleAll,
    /// A character typed into the search.
    Char(char),
    /// Delete the last search character.
    Backspace,
}

/// An action with the count typed before it (`5j`, `3G`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// What to do.
    pub action: Action,
    /// The number typed before it, if any.
    pub count: Option<usize>,
}

impl From<Action> for Command {
    fn from(action: Action) -> Self {
        Self {
            action,
            count: None,
        }
    }
}

/// Largest count we keep, so a held digit key cannot overflow.
const MAX_COUNT: usize = 1_000_000;

/// Turns key presses into commands, remembering the vim-style prefixes: a
/// count (`5j`) and a pending `g` (`gg`).
#[derive(Debug, Default)]
pub struct KeyParser {
    count: Option<usize>,
    pending_g: bool,
}

impl KeyParser {
    /// The keys typed so far that are still waiting for the rest, e.g. `5g`.
    #[must_use]
    pub fn pending(&self) -> String {
        let count = self.count.map_or_else(String::new, |c| c.to_string());
        format!("{count}{}", if self.pending_g { "g" } else { "" })
    }

    fn reset(&mut self) {
        self.count = None;
        self.pending_g = false;
    }

    /// Feed one key press. `typing` is true while a search is being entered,
    /// when nothing is a prefix.
    pub fn feed(&mut self, key: KeyEvent, typing: bool) -> Option<Command> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if typing {
            self.reset();
            return map(key, true).map(Command::from);
        }
        let plain = !key.modifiers.contains(KeyModifiers::CONTROL);
        if self.pending_g {
            self.pending_g = false;
            let count = self.count.take();
            return (plain && key.code == KeyCode::Char('g')).then_some(Command {
                action: Action::Top,
                count,
            });
        }
        match key.code {
            KeyCode::Esc if self.count.is_some() => {
                self.count = None;
                return None;
            }
            KeyCode::Char(c)
                if plain && c.is_ascii_digit() && (c != '0' || self.count.is_some()) =>
            {
                let digit = c
                    .to_digit(10)
                    .and_then(|d| usize::try_from(d).ok())
                    .unwrap_or(0);
                let count = self
                    .count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit);
                self.count = Some(count.min(MAX_COUNT));
                return None;
            }
            KeyCode::Char('g') if plain => {
                self.pending_g = true;
                return None;
            }
            _ => {}
        }
        let count = self.count.take();
        map(key, false).map(|action| Command { action, count })
    }
}

/// Translate one key press on its own; `typing` is true while a search is
/// being entered. Prefixes (counts, `gg`) are handled by [`KeyParser`].
#[must_use]
#[allow(clippy::match_same_arms)] // arms differ by `typing`, which is the point
pub fn map(key: KeyEvent, typing: bool) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match (typing, key.code) {
            (_, KeyCode::Char('c')) => Some(Action::Quit),
            (false, KeyCode::Char('d')) => Some(Action::HalfPageDown),
            (false, KeyCode::Char('u')) => Some(Action::HalfPageUp),
            (false, KeyCode::Char('f')) => Some(Action::PageDown),
            (false, KeyCode::Char('b')) => Some(Action::PageUp),
            _ => None,
        };
    }
    match (typing, key.code) {
        (_, KeyCode::Enter) => Some(Action::Enter),
        (_, KeyCode::Esc) => Some(Action::Back),
        (true, KeyCode::Backspace) => Some(Action::Backspace),
        (true, KeyCode::Char(c)) => Some(Action::Char(c)),
        (true, _) => None,
        (false, KeyCode::Char('j') | KeyCode::Down) => Some(Action::Down),
        (false, KeyCode::Char('k') | KeyCode::Up) => Some(Action::Up),
        (false, KeyCode::Home) => Some(Action::Top),
        (false, KeyCode::PageDown) => Some(Action::PageDown),
        (false, KeyCode::PageUp) => Some(Action::PageUp),
        (false, KeyCode::Char('G') | KeyCode::End) => Some(Action::Bottom),
        (false, KeyCode::Char('l') | KeyCode::Right) => Some(Action::Enter),
        (false, KeyCode::Char('h') | KeyCode::Left) => Some(Action::Back),
        (false, KeyCode::Char(' ')) => Some(Action::Toggle),
        (false, KeyCode::Char('a')) => Some(Action::ToggleAll),
        (false, KeyCode::Char('s')) => Some(Action::Sort),
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

    fn feed_all(parser: &mut KeyParser, keys: &[KeyCode]) -> Option<Command> {
        let mut last = None;
        for &code in keys {
            last = parser.feed(key(code), false);
        }
        last
    }

    #[allow(clippy::unnecessary_wraps)] // reads as the parser's return type
    fn cmd(action: Action, count: Option<usize>) -> Option<Command> {
        Some(Command { action, count })
    }

    #[test]
    fn a_lone_g_is_a_prefix_not_a_motion() {
        let mut p = KeyParser::default();
        assert_eq!(p.feed(key(KeyCode::Char('g')), false), None);
        assert_eq!(p.pending(), "g");
    }

    #[test]
    fn gg_goes_to_the_top() {
        let mut p = KeyParser::default();
        let got = feed_all(&mut p, &[KeyCode::Char('g'), KeyCode::Char('g')]);
        assert_eq!(got, cmd(Action::Top, None));
        assert_eq!(p.pending(), "");
    }

    #[test]
    fn capital_g_goes_to_the_bottom_and_counts_pick_a_row() {
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Char('G')), false),
            cmd(Action::Bottom, None)
        );
        let got = feed_all(&mut p, &[KeyCode::Char('7'), KeyCode::Char('G')]);
        assert_eq!(got, cmd(Action::Bottom, Some(7)));
        let got = feed_all(
            &mut p,
            &[KeyCode::Char('5'), KeyCode::Char('g'), KeyCode::Char('g')],
        );
        assert_eq!(got, cmd(Action::Top, Some(5)));
    }

    #[test]
    fn counts_apply_to_moves_and_zero_continues_but_never_starts_one() {
        let mut p = KeyParser::default();
        let got = feed_all(&mut p, &[KeyCode::Char('3'), KeyCode::Char('j')]);
        assert_eq!(got, cmd(Action::Down, Some(3)));
        let got = feed_all(
            &mut p,
            &[KeyCode::Char('1'), KeyCode::Char('0'), KeyCode::Down],
        );
        assert_eq!(got, cmd(Action::Down, Some(10)));
        assert_eq!(p.feed(key(KeyCode::Char('0')), false), None);
        assert_eq!(p.pending(), "");
    }

    #[test]
    fn pending_input_shows_as_typed() {
        let mut p = KeyParser::default();
        feed_all(
            &mut p,
            &[KeyCode::Char('1'), KeyCode::Char('2'), KeyCode::Char('g')],
        );
        assert_eq!(p.pending(), "12g");
    }

    #[test]
    fn esc_cancels_pending_input_without_going_back() {
        let mut p = KeyParser::default();
        feed_all(&mut p, &[KeyCode::Char('g')]);
        assert_eq!(p.feed(key(KeyCode::Esc), false), None);
        assert_eq!(p.pending(), "");
        feed_all(&mut p, &[KeyCode::Char('5')]);
        assert_eq!(p.feed(key(KeyCode::Esc), false), None);
        assert_eq!(p.pending(), "");
        assert_eq!(p.feed(key(KeyCode::Esc), false), cmd(Action::Back, None));
    }

    #[test]
    fn g_then_another_key_cancels_the_chord() {
        let mut p = KeyParser::default();
        assert_eq!(
            feed_all(&mut p, &[KeyCode::Char('g'), KeyCode::Char('j')]),
            None
        );
        assert_eq!(p.pending(), "");
        assert_eq!(
            p.feed(key(KeyCode::Char('j')), false),
            cmd(Action::Down, None)
        );
    }

    #[test]
    fn nothing_is_a_chord_while_typing() {
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Char('g')), true),
            cmd(Action::Char('g'), None)
        );
        assert_eq!(
            p.feed(key(KeyCode::Char('5')), true),
            cmd(Action::Char('5'), None)
        );
        assert_eq!(p.pending(), "");
    }

    #[test]
    fn control_keys_page_and_are_ignored_while_typing() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let mut p = KeyParser::default();
        assert_eq!(p.feed(ctrl('d'), false), cmd(Action::HalfPageDown, None));
        assert_eq!(p.feed(ctrl('u'), false), cmd(Action::HalfPageUp, None));
        assert_eq!(p.feed(ctrl('f'), false), cmd(Action::PageDown, None));
        assert_eq!(p.feed(ctrl('b'), false), cmd(Action::PageUp, None));
        assert_eq!(
            p.feed(key(KeyCode::PageDown), false),
            cmd(Action::PageDown, None)
        );
        assert_eq!(
            p.feed(key(KeyCode::PageUp), false),
            cmd(Action::PageUp, None)
        );
        assert_eq!(p.feed(ctrl('d'), true), None);
        assert_eq!(p.feed(ctrl('c'), true), cmd(Action::Quit, None));
    }

    #[test]
    fn home_and_end_still_jump() {
        let mut p = KeyParser::default();
        assert_eq!(p.feed(key(KeyCode::Home), false), cmd(Action::Top, None));
        assert_eq!(p.feed(key(KeyCode::End), false), cmd(Action::Bottom, None));
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
    fn s_sorts_but_only_when_not_typing() {
        assert_eq!(map(key(KeyCode::Char('s')), false), Some(Action::Sort));
        assert_eq!(map(key(KeyCode::Char('s')), true), Some(Action::Char('s')));
    }

    #[test]
    fn ctrl_c_always_quits() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map(k, true), Some(Action::Quit));
        assert_eq!(map(k, false), Some(Action::Quit));
    }
}

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
//! | `Enter`                 | drop in (select, go to next level)  |
//! | `Esc`                   | drop out (never exits at top level) |
//! | `h` / `Left`            | column cursor one column left       |
//! | `l` / `Right`           | column cursor one column right      |
//! | `/` / `i`               | edit the search box                 |
//! | `f`                     | filter the column under the column cursor |
//! | `c`                     | clear search, column filters and sort |
//! | `s`                     | sort the column: ascending, descending, none |
//! | `Space`                 | toggle a row and advance, or the highlighted rows (multi-select) |
//! | `J` / `K`, Shift-`↓`/`↑` | move and highlight a range (multi-select; Shift-arrows not in Terminal.app) |
//! | `a`                     | toggle all visible rows (multi-select) |
//! | `?`                     | show every key, in sections         |
//! | `q` / `Ctrl-c`          | quit, from any level                |
//!
//! While typing a search, characters are text; `Enter` and `Esc` both leave
//! the box and keep the filter.

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
    /// Drop out one level (or leave the search box, keeping its text).
    Back,
    /// Quit.
    Quit,
    /// Ctrl-C: quits, except while a load is running, where it cancels it.
    Interrupt,
    /// Start editing the search box.
    Search,
    /// Start editing the filter box of the column under the column cursor.
    Filter,
    /// Ask to remove the collected remote under the cursor.
    Delete,
    /// Open a prompt for a new remote's URL.
    New,
    /// Make the collected remote under the cursor a real git remote.
    Promote,
    /// Open a prompt for a URL to browse once, without saving it.
    Open,
    /// Open the menu of places to collect remotes from.
    Scan,
    /// Show or hide the refs that were left out of the refs list.
    ShowHidden,
    /// Ctrl-r in the search box: switch between simple and regex matching.
    ToggleRegex,
    /// Say yes to the question being asked.
    Confirm,
    /// Clear the search / filter.
    Clear,
    /// Sort the column under the column cursor: ascending, descending, none.
    Sort,
    /// Move the column cursor one column left.
    ColumnPrev,
    /// Move the column cursor one column right.
    ColumnNext,
    /// Move down and highlight from where the highlight started (multi-select).
    ExtendDown,
    /// Move up and highlight from where the highlight started (multi-select).
    ExtendUp,
    /// Show the key help.
    Help,
    /// Toggle the row under the cursor, or the highlighted rows (multi-select).
    Toggle,
    /// Toggle every visible row (multi-select).
    ToggleAll,
    /// A character typed into the search.
    Char(char),
    /// Delete the last search character.
    Backspace,
}

/// Which keymap is in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Browsing: the normal keymap.
    #[default]
    Normal,
    /// Typing a search: characters are text.
    Typing,
    /// A yes/no question is open: `y` says yes, any other key says no.
    Confirm,
    /// A one-line prompt is open: characters are text, like the search box.
    Prompt,
    /// A menu is open: a letter picks an item, any other key closes it.
    Menu,
    /// The key help is showing: any key closes it.
    Help,
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

    /// Feed one key press. While a search is being typed nothing is a prefix.
    pub fn feed(&mut self, key: KeyEvent, mode: Mode) -> Option<Command> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if matches!(
            mode,
            Mode::Typing | Mode::Confirm | Mode::Prompt | Mode::Menu
        ) {
            self.reset();
            return map(key, mode).map(Command::from);
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
        map(key, mode).map(|action| Command { action, count })
    }
}

/// Translate one key press on its own. Prefixes (counts, `gg`) are handled
/// by [`KeyParser`].
#[must_use]
pub fn map(key: KeyEvent, mode: Mode) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match (mode, key.code) {
            (_, KeyCode::Char('c')) => Some(Action::Interrupt),
            (Mode::Help, _) => Some(Action::Back),
            (Mode::Typing, KeyCode::Char('r')) => Some(Action::ToggleRegex),
            (Mode::Typing | Mode::Prompt | Mode::Menu, _) => None,
            (_, KeyCode::Char('d')) => Some(Action::HalfPageDown),
            (_, KeyCode::Char('u')) => Some(Action::HalfPageUp),
            (_, KeyCode::Char('f')) => Some(Action::PageDown),
            (_, KeyCode::Char('b')) => Some(Action::PageUp),
            _ => None,
        };
    }
    match mode {
        Mode::Typing | Mode::Prompt => map_typing(key.code),
        Mode::Normal => map_normal(key.code, key.modifiers),
        Mode::Confirm => Some(map_confirm(key.code)),
        Mode::Menu => Some(map_menu(key.code)),
        Mode::Help => Some(Action::Back),
    }
}

/// A letter picks the menu item. Any other key closes the menu.
fn map_menu(code: KeyCode) -> Action {
    match code {
        KeyCode::Char(c) => Action::Char(c),
        _ => Action::Back,
    }
}

/// `y` is yes and every other key is no, so a stray key never confirms.
fn map_confirm(code: KeyCode) -> Action {
    match code {
        KeyCode::Char('y' | 'Y') => Action::Confirm,
        _ => Action::Back,
    }
}

fn map_typing(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Enter => Some(Action::Enter),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char(c) => Some(Action::Char(c)),
        _ => None,
    }
}

/// Motions for browsing.
fn map_motion(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Home => Some(Action::Top),
        KeyCode::Char('G') | KeyCode::End => Some(Action::Bottom),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::PageUp => Some(Action::PageUp),
        _ => None,
    }
}

fn map_normal(code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    // Shift-arrows only arrive as such in some terminals; J and K always do.
    if modifiers.contains(KeyModifiers::SHIFT) {
        match code {
            KeyCode::Down => return Some(Action::ExtendDown),
            KeyCode::Up => return Some(Action::ExtendUp),
            _ => {}
        }
    }
    map_motion(code).or(match code {
        KeyCode::Char('J') => Some(Action::ExtendDown),
        KeyCode::Char('K') => Some(Action::ExtendUp),
        KeyCode::Enter => Some(Action::Enter),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Char('h') | KeyCode::Left => Some(Action::ColumnPrev),
        KeyCode::Char('l') | KeyCode::Right => Some(Action::ColumnNext),
        KeyCode::Char(' ') => Some(Action::Toggle),
        KeyCode::Char('a') => Some(Action::ToggleAll),
        KeyCode::Char('s') => Some(Action::Sort),
        KeyCode::Char('/' | 'i') => Some(Action::Search),
        KeyCode::Char('f') => Some(Action::Filter),
        KeyCode::Char('?') => Some(Action::Help),
        KeyCode::Char('c') => Some(Action::Clear),
        KeyCode::Char('d') => Some(Action::Delete),
        KeyCode::Char('n') => Some(Action::New),
        KeyCode::Char('p') => Some(Action::Promote),
        KeyCode::Char('o') => Some(Action::Open),
        KeyCode::Char('A') => Some(Action::Scan),
        KeyCode::Char('H') => Some(Action::ShowHidden),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    })
}

/// One key that must do `action` in `mode`. The help table lists these so a
/// test can check what it says against what the keymap does.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// The key.
    pub code: KeyCode,
    /// Held with it.
    pub modifiers: KeyModifiers,
    /// Which keymap to ask.
    pub mode: Mode,
    /// What the keymap should answer.
    pub action: Action,
}

/// One line of the help: the keys as shown, what they do, and a check.
#[derive(Debug, Clone, Copy)]
pub struct HelpEntry {
    /// The keys, as shown.
    pub keys: &'static str,
    /// What they do.
    pub does: &'static str,
    /// Keys from the line that the keymap must agree with.
    pub samples: &'static [Sample],
}

/// A titled group of help lines.
#[derive(Debug, Clone, Copy)]
pub struct HelpSection {
    /// The heading.
    pub title: &'static str,
    /// The lines under it.
    pub entries: &'static [HelpEntry],
}

const fn plain(code: KeyCode, action: Action) -> Sample {
    Sample {
        code,
        modifiers: KeyModifiers::NONE,
        mode: Mode::Normal,
        action,
    }
}

const fn ctrl(c: char, mode: Mode, action: Action) -> Sample {
    Sample {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        mode,
        action,
    }
}

const fn entry(keys: &'static str, does: &'static str, samples: &'static [Sample]) -> HelpEntry {
    HelpEntry {
        keys,
        does,
        samples,
    }
}

/// Every key, for the `?` overlay. A test checks each sample against [`map`]
/// and that no key in the keymap is missing, so this cannot drift from it.
pub const HELP: &[HelpSection] = &[
    HelpSection {
        title: "Move",
        entries: &[
            entry(
                "j k \u{2193} \u{2191}",
                "move; 5j counts",
                &[
                    plain(KeyCode::Char('j'), Action::Down),
                    plain(KeyCode::Char('k'), Action::Up),
                ],
            ),
            entry(
                "gg G",
                "top, bottom; 5G row 5",
                &[
                    plain(KeyCode::Home, Action::Top),
                    plain(KeyCode::Char('G'), Action::Bottom),
                ],
            ),
            entry(
                "^d ^u ^f ^b",
                "half page, page",
                &[
                    ctrl('d', Mode::Normal, Action::HalfPageDown),
                    ctrl('u', Mode::Normal, Action::HalfPageUp),
                    ctrl('f', Mode::Normal, Action::PageDown),
                    ctrl('b', Mode::Normal, Action::PageUp),
                ],
            ),
        ],
    },
    HelpSection {
        title: "Levels",
        entries: &[
            entry(
                "Enter",
                "open (write in add)",
                &[plain(KeyCode::Enter, Action::Enter)],
            ),
            entry(
                "Esc",
                "back; cancels input",
                &[plain(KeyCode::Esc, Action::Back)],
            ),
            entry(
                "q ^c",
                "quit",
                &[
                    plain(KeyCode::Char('q'), Action::Quit),
                    ctrl('c', Mode::Normal, Action::Interrupt),
                ],
            ),
            entry("?", "this help", &[plain(KeyCode::Char('?'), Action::Help)]),
        ],
    },
    HelpSection {
        title: "Search and filter",
        entries: &[
            entry(
                "/ i",
                "search; Esc leaves",
                &[
                    plain(KeyCode::Char('/'), Action::Search),
                    plain(KeyCode::Char('i'), Action::Search),
                ],
            ),
            entry(
                "^r",
                "in a box: plain or regex",
                &[ctrl('r', Mode::Typing, Action::ToggleRegex)],
            ),
            entry(
                "f",
                "filter the column",
                &[plain(KeyCode::Char('f'), Action::Filter)],
            ),
            entry(
                "c",
                "clear search, filters, sort",
                &[plain(KeyCode::Char('c'), Action::Clear)],
            ),
        ],
    },
    HelpSection {
        title: "Columns",
        entries: &[
            entry(
                "h l \u{2190} \u{2192}",
                "column cursor",
                &[
                    plain(KeyCode::Char('h'), Action::ColumnPrev),
                    plain(KeyCode::Char('l'), Action::ColumnNext),
                ],
            ),
            entry(
                "s",
                "sort: up, down, off",
                &[plain(KeyCode::Char('s'), Action::Sort)],
            ),
        ],
    },
    HelpSection {
        title: "Remote list",
        entries: &[
            entry("n", "add a URL", &[plain(KeyCode::Char('n'), Action::New)]),
            entry(
                "o",
                "browse a URL once",
                &[plain(KeyCode::Char('o'), Action::Open)],
            ),
            entry(
                "p",
                "make it a git remote",
                &[plain(KeyCode::Char('p'), Action::Promote)],
            ),
            entry(
                "d",
                "remove a collected one",
                &[plain(KeyCode::Char('d'), Action::Delete)],
            ),
            entry(
                "A",
                "scan: m t f",
                &[plain(KeyCode::Char('A'), Action::Scan)],
            ),
        ],
    },
    HelpSection {
        title: "Refs list",
        entries: &[entry(
            "H",
            "show hidden refs",
            &[plain(KeyCode::Char('H'), Action::ShowHidden)],
        )],
    },
    HelpSection {
        title: "Add picker",
        entries: &[
            entry(
                "space",
                "mark and move on",
                &[plain(KeyCode::Char(' '), Action::Toggle)],
            ),
            entry(
                "J K",
                "highlight a range",
                &[
                    plain(KeyCode::Char('J'), Action::ExtendDown),
                    plain(KeyCode::Char('K'), Action::ExtendUp),
                ],
            ),
            entry(
                "a",
                "mark all shown",
                &[plain(KeyCode::Char('a'), Action::ToggleAll)],
            ),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn feed_all(parser: &mut KeyParser, keys: &[KeyCode]) -> Option<Command> {
        let mut last = None;
        for &code in keys {
            last = parser.feed(key(code), Mode::Normal);
        }
        last
    }

    #[test]
    fn d_asks_to_remove_and_the_confirm_keymap_is_y_or_anything_else() {
        assert_eq!(
            map(key(KeyCode::Char('d')), Mode::Normal),
            Some(Action::Delete)
        );
        assert_eq!(
            map(key(KeyCode::Char('y')), Mode::Confirm),
            Some(Action::Confirm)
        );
        for code in [
            KeyCode::Char('n'),
            KeyCode::Esc,
            KeyCode::Char('x'),
            KeyCode::Enter,
        ] {
            assert_eq!(
                map(key(code), Mode::Confirm),
                Some(Action::Back),
                "{code:?}"
            );
        }
        // No counts or prefixes while a question is open.
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Char('5')), Mode::Confirm),
            cmd(Action::Back, None)
        );
    }

    #[test]
    fn ctrl_r_switches_the_search_mode_only_while_typing() {
        let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(map(ctrl_r, Mode::Typing), Some(Action::ToggleRegex));
        assert_eq!(map(ctrl_r, Mode::Normal), None);
        assert_eq!(map(ctrl_r, Mode::Prompt), None);
        // A plain r is text while typing.
        assert_eq!(
            map(key(KeyCode::Char('r')), Mode::Typing),
            Some(Action::Char('r'))
        );
    }

    #[test]
    fn capital_h_shows_the_hidden_refs() {
        assert_eq!(
            map(key(KeyCode::Char('H')), Mode::Normal),
            Some(Action::ShowHidden)
        );
        assert_eq!(
            map(key(KeyCode::Char('H')), Mode::Typing),
            Some(Action::Char('H'))
        );
    }

    #[test]
    fn capital_a_opens_the_scan_menu_and_the_menu_takes_letters() {
        assert_eq!(
            map(key(KeyCode::Char('A')), Mode::Normal),
            Some(Action::Scan)
        );
        assert_eq!(
            map(key(KeyCode::Char('m')), Mode::Menu),
            Some(Action::Char('m'))
        );
        assert_eq!(map(key(KeyCode::Esc), Mode::Menu), Some(Action::Back));
        assert_eq!(map(key(KeyCode::Down), Mode::Menu), Some(Action::Back));
    }

    #[test]
    fn o_opens_a_url_in_the_normal_keymap_only() {
        assert_eq!(
            map(key(KeyCode::Char('o')), Mode::Normal),
            Some(Action::Open)
        );
        assert_eq!(
            map(key(KeyCode::Char('o')), Mode::Prompt),
            Some(Action::Char('o'))
        );
    }

    #[test]
    fn p_promotes_in_the_normal_keymap_only() {
        assert_eq!(
            map(key(KeyCode::Char('p')), Mode::Normal),
            Some(Action::Promote)
        );
        assert_eq!(
            map(key(KeyCode::Char('p')), Mode::Prompt),
            Some(Action::Char('p'))
        );
    }

    #[test]
    fn n_starts_a_new_url_and_the_prompt_takes_text_like_the_search_box() {
        assert_eq!(
            map(key(KeyCode::Char('n')), Mode::Normal),
            Some(Action::New)
        );
        assert_eq!(
            map(key(KeyCode::Char('n')), Mode::Prompt),
            Some(Action::Char('n'))
        );
        assert_eq!(map(key(KeyCode::Enter), Mode::Prompt), Some(Action::Enter));
        assert_eq!(map(key(KeyCode::Esc), Mode::Prompt), Some(Action::Back));
        assert_eq!(
            map(key(KeyCode::Backspace), Mode::Prompt),
            Some(Action::Backspace)
        );
        // No counts: a digit is text.
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Char('5')), Mode::Prompt),
            cmd(Action::Char('5'), None)
        );
    }

    #[test]
    fn i_and_slash_both_start_editing_the_search() {
        for c in ['i', '/'] {
            assert_eq!(
                map(key(KeyCode::Char(c)), Mode::Normal),
                Some(Action::Search)
            );
        }
        // While editing, `i` is just a letter.
        assert_eq!(
            map(key(KeyCode::Char('i')), Mode::Typing),
            Some(Action::Char('i'))
        );
    }

    #[allow(clippy::unnecessary_wraps)] // reads as the parser's return type
    fn cmd(action: Action, count: Option<usize>) -> Option<Command> {
        Some(Command { action, count })
    }

    #[test]
    fn a_lone_g_is_a_prefix_not_a_motion() {
        let mut p = KeyParser::default();
        assert_eq!(p.feed(key(KeyCode::Char('g')), Mode::Normal), None);
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
            p.feed(key(KeyCode::Char('G')), Mode::Normal),
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
        assert_eq!(p.feed(key(KeyCode::Char('0')), Mode::Normal), None);
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
        assert_eq!(p.feed(key(KeyCode::Esc), Mode::Normal), None);
        assert_eq!(p.pending(), "");
        feed_all(&mut p, &[KeyCode::Char('5')]);
        assert_eq!(p.feed(key(KeyCode::Esc), Mode::Normal), None);
        assert_eq!(p.pending(), "");
        assert_eq!(
            p.feed(key(KeyCode::Esc), Mode::Normal),
            cmd(Action::Back, None)
        );
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
            p.feed(key(KeyCode::Char('j')), Mode::Normal),
            cmd(Action::Down, None)
        );
    }

    #[test]
    fn nothing_is_a_chord_while_typing() {
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Char('g')), Mode::Typing),
            cmd(Action::Char('g'), None)
        );
        assert_eq!(
            p.feed(key(KeyCode::Char('5')), Mode::Typing),
            cmd(Action::Char('5'), None)
        );
        assert_eq!(p.pending(), "");
    }

    #[test]
    fn control_keys_page_and_are_ignored_while_typing() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(ctrl('d'), Mode::Normal),
            cmd(Action::HalfPageDown, None)
        );
        assert_eq!(
            p.feed(ctrl('u'), Mode::Normal),
            cmd(Action::HalfPageUp, None)
        );
        assert_eq!(p.feed(ctrl('f'), Mode::Normal), cmd(Action::PageDown, None));
        assert_eq!(p.feed(ctrl('b'), Mode::Normal), cmd(Action::PageUp, None));
        assert_eq!(
            p.feed(key(KeyCode::PageDown), Mode::Normal),
            cmd(Action::PageDown, None)
        );
        assert_eq!(
            p.feed(key(KeyCode::PageUp), Mode::Normal),
            cmd(Action::PageUp, None)
        );
        assert_eq!(p.feed(ctrl('d'), Mode::Typing), None);
        assert_eq!(
            p.feed(ctrl('c'), Mode::Typing),
            cmd(Action::Interrupt, None)
        );
    }

    #[test]
    fn home_and_end_still_jump() {
        let mut p = KeyParser::default();
        assert_eq!(
            p.feed(key(KeyCode::Home), Mode::Normal),
            cmd(Action::Top, None)
        );
        assert_eq!(
            p.feed(key(KeyCode::End), Mode::Normal),
            cmd(Action::Bottom, None)
        );
    }

    #[test]
    fn hjkl_and_arrows_agree() {
        for (a, b) in [
            (KeyCode::Char('j'), KeyCode::Down),
            (KeyCode::Char('k'), KeyCode::Up),
            (KeyCode::Char('h'), KeyCode::Left),
            (KeyCode::Char('l'), KeyCode::Right),
        ] {
            assert_eq!(map(key(a), Mode::Normal), map(key(b), Mode::Normal));
        }
    }

    #[test]
    fn j_and_k_in_capitals_and_shift_arrows_extend() {
        let shift = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
        assert_eq!(
            map(shift(KeyCode::Char('J')), Mode::Normal),
            Some(Action::ExtendDown)
        );
        assert_eq!(
            map(shift(KeyCode::Char('K')), Mode::Normal),
            Some(Action::ExtendUp)
        );
        assert_eq!(
            map(shift(KeyCode::Down), Mode::Normal),
            Some(Action::ExtendDown)
        );
        assert_eq!(
            map(shift(KeyCode::Up), Mode::Normal),
            Some(Action::ExtendUp)
        );
        // A plain arrow is still a plain move, and typing is unaffected.
        assert_eq!(map(key(KeyCode::Down), Mode::Normal), Some(Action::Down));
        assert_eq!(
            map(shift(KeyCode::Char('J')), Mode::Typing),
            Some(Action::Char('J'))
        );
    }

    #[test]
    fn q_is_text_while_typing_but_quits_otherwise() {
        assert_eq!(
            map(key(KeyCode::Char('q')), Mode::Typing),
            Some(Action::Char('q'))
        );
        assert_eq!(
            map(key(KeyCode::Char('q')), Mode::Normal),
            Some(Action::Quit)
        );
    }

    #[test]
    fn space_and_a_toggle_but_only_when_not_typing() {
        assert_eq!(
            map(key(KeyCode::Char(' ')), Mode::Normal),
            Some(Action::Toggle)
        );
        assert_eq!(
            map(key(KeyCode::Char('a')), Mode::Normal),
            Some(Action::ToggleAll)
        );
        assert_eq!(
            map(key(KeyCode::Char(' ')), Mode::Typing),
            Some(Action::Char(' '))
        );
        assert_eq!(
            map(key(KeyCode::Char('a')), Mode::Typing),
            Some(Action::Char('a'))
        );
    }

    #[test]
    fn h_and_l_and_the_arrows_move_the_column_cursor() {
        let m = |code| map(key(code), Mode::Normal);
        assert_eq!(m(KeyCode::Char('h')), Some(Action::ColumnPrev));
        assert_eq!(m(KeyCode::Left), Some(Action::ColumnPrev));
        assert_eq!(m(KeyCode::Char('l')), Some(Action::ColumnNext));
        assert_eq!(m(KeyCode::Right), Some(Action::ColumnNext));
    }

    #[test]
    fn only_enter_and_esc_change_levels() {
        let m = |code| map(key(code), Mode::Normal);
        assert_eq!(m(KeyCode::Enter), Some(Action::Enter));
        assert_eq!(m(KeyCode::Esc), Some(Action::Back));
    }

    #[test]
    fn r_does_nothing_while_browsing_and_is_text_while_typing() {
        assert_eq!(map(key(KeyCode::Char('r')), Mode::Normal), None);
        assert_eq!(
            map(key(KeyCode::Char('r')), Mode::Typing),
            Some(Action::Char('r'))
        );
    }

    #[test]
    fn s_sorts_but_only_when_not_typing() {
        assert_eq!(
            map(key(KeyCode::Char('s')), Mode::Normal),
            Some(Action::Sort)
        );
        assert_eq!(
            map(key(KeyCode::Char('s')), Mode::Typing),
            Some(Action::Char('s'))
        );
    }

    #[test]
    fn ctrl_c_is_an_interrupt_in_every_mode() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        for mode in [Mode::Typing, Mode::Normal] {
            assert_eq!(map(k, mode), Some(Action::Interrupt));
        }
    }

    #[test]
    fn every_help_sample_is_what_the_keymap_does() {
        for section in HELP {
            for line in section.entries {
                assert!(!line.samples.is_empty(), "{}", line.keys);
                for sample in line.samples {
                    let got = map(KeyEvent::new(sample.code, sample.modifiers), sample.mode);
                    assert_eq!(got, Some(sample.action), "{}: {sample:?}", line.keys);
                }
            }
        }
    }

    #[test]
    fn every_key_in_the_keymap_is_in_the_help() {
        let documented: Vec<Action> = HELP
            .iter()
            .flat_map(|s| s.entries)
            .flat_map(|e| e.samples)
            .map(|s| s.action)
            .collect();
        let mut keys: Vec<KeyEvent> = Vec::new();
        for c in ' '..='~' {
            keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
        }
        for code in [
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ] {
            keys.push(KeyEvent::new(code, KeyModifiers::NONE));
            keys.push(KeyEvent::new(code, KeyModifiers::SHIFT));
        }
        for key in keys {
            for mode in [Mode::Normal, Mode::Typing] {
                // Typing keys are text, and Backspace is not a command.
                if let Some(action) = map(key, mode)
                    && !matches!(action, Action::Char(_) | Action::Backspace)
                {
                    assert!(
                        documented.contains(&action)
                            || (mode == Mode::Typing && action == Action::Enter),
                        "{key:?} in {mode:?} does {action:?}, which the help does not list"
                    );
                }
            }
        }
    }

    #[test]
    fn any_key_closes_the_help_but_ctrl_c_still_quits() {
        for code in [
            KeyCode::Char('x'),
            KeyCode::Enter,
            KeyCode::Up,
            KeyCode::Esc,
        ] {
            assert_eq!(map(key(code), Mode::Help), Some(Action::Back));
        }
        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(map(ctrl_d, Mode::Help), Some(Action::Back));
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map(ctrl_c, Mode::Help), Some(Action::Interrupt));
    }
}

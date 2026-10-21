// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Picker state machine: remotes, then refs. Pure, so it is easy to test.

use crate::error::{Error, Result};
use crate::git::{Remote, RemoteRef};
use crate::tui::keys::{Action, Command, Mode};
use crate::tui::list::{List, SortKeys, SortOrder};

/// Which list is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Choosing a remote.
    Remotes,
    /// Choosing a ref of the chosen remote.
    Refs,
}

/// What the event loop should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flow {
    /// Keep going.
    Continue,
    /// Leave without a selection.
    Quit,
    /// A remote and ref were chosen.
    Done(Selection),
}

/// The final choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Remote name.
    pub remote: String,
    /// The remote's fetch URL, with any `insteadOf` rewrites applied.
    pub url: String,
    /// Full ref name.
    pub refname: String,
}

/// The refs of a remote to list, after hiding the ones other trees carry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loaded {
    /// Refs to show.
    pub refs: Vec<RemoteRef>,
    /// How many refs were left out.
    pub hidden: usize,
    /// A problem worth a line in the status bar, such as a stale cache.
    pub note: Option<String>,
}

impl From<Vec<RemoteRef>> for Loaded {
    fn from(refs: Vec<RemoteRef>) -> Self {
        Self {
            refs,
            ..Self::default()
        }
    }
}

/// Fetches the refs of a remote (`git ls-remote`).
pub type Loader<'a> = dyn FnMut(&Remote) -> Result<Loaded> + 'a;

fn remote_cells(r: &Remote) -> Vec<String> {
    vec![r.name.clone(), r.url.clone(), r.tags.join(",")]
}

/// Picker state.
#[derive(Debug)]
pub struct App {
    remotes: Vec<Remote>,
    remote_list: List,
    refs: Vec<RemoteRef>,
    hidden: usize,
    ref_list: List,
    level: Level,
    typing: bool,
    sorting: bool,
    error: Option<String>,
    notice: Option<String>,
    viewport: usize,
    chosen: Option<usize>,
}

impl App {
    /// Start at the remote picker.
    #[must_use]
    pub fn new(remotes: Vec<Remote>) -> Self {
        let cells = remotes.iter().map(remote_cells).collect();
        let keys = remotes
            .iter()
            .map(|r| SortKeys {
                name: r.name.clone(),
                url: r.url.clone(),
                tags: r.tags.join(","),
                // Real remotes stay above collected ones.
                group: u8::from(!r.tags.is_empty()),
            })
            .collect();
        Self {
            remotes,
            remote_list: List::with_cells(cells, keys),
            refs: Vec::new(),
            hidden: 0,
            ref_list: List::default(),
            level: Level::Remotes,
            typing: false,
            sorting: false,
            error: None,
            notice: None,
            viewport: 10,
            chosen: None,
        }
    }

    /// Show a message in the status line until the next key press.
    #[must_use]
    pub fn with_notice(mut self, notice: Option<String>) -> Self {
        self.notice = notice;
        self
    }

    /// The pending notice, if any.
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// How many refs of the open remote are hidden.
    #[must_use]
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// Current level.
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
    }

    /// True while a sort column is being chosen.
    #[must_use]
    pub fn sorting(&self) -> bool {
        self.sorting
    }

    /// Which keymap is in force.
    #[must_use]
    pub fn mode(&self) -> Mode {
        if self.typing {
            Mode::Typing
        } else if self.sorting {
            Mode::Sort
        } else {
            Mode::Normal
        }
    }

    /// True while a search is being typed.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.typing
    }

    /// The last error, shown in the status line.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The list for the current level.
    #[must_use]
    pub fn list(&self) -> &List {
        match self.level {
            Level::Remotes => &self.remote_list,
            Level::Refs => &self.ref_list,
        }
    }

    /// Name of the remote whose refs are showing, if any.
    #[must_use]
    pub fn chosen_remote(&self) -> Option<&str> {
        self.chosen.map(|i| self.remotes[i].name.as_str())
    }

    fn list_mut(&mut self) -> &mut List {
        match self.level {
            Level::Remotes => &mut self.remote_list,
            Level::Refs => &mut self.ref_list,
        }
    }

    /// Tell the picker how many rows fit on screen, for paging.
    pub fn set_viewport(&mut self, rows: usize) {
        self.viewport = rows.max(1);
    }

    /// Apply one action.
    pub fn update(&mut self, action: Action, load: &mut Loader<'_>) -> Flow {
        self.update_with(Command::from(action), load)
    }

    /// Apply one action with the count typed before it.
    pub fn update_with(&mut self, command: Command, load: &mut Loader<'_>) -> Flow {
        let Command { action, count } = command;
        self.error = None;
        self.notice = None;
        if action == Action::Quit {
            return Flow::Quit;
        }
        if self.typing {
            self.update_typing(action);
            return Flow::Continue;
        }
        let viewport = self.viewport;
        if self.list_mut().motion(action, count, viewport) {
            return Flow::Continue;
        }
        if self.sorting {
            self.update_sorting(action);
            return Flow::Continue;
        }
        match action {
            Action::Search => self.typing = true,
            Action::Clear => self.list_mut().clear(),
            Action::Sort => self.sorting = true,
            Action::Reverse => self.list_mut().toggle_reverse(),
            Action::Back => self.back(),
            Action::Enter => return self.enter(load),
            Action::Quit
            | Action::Char(_)
            | Action::Backspace
            | Action::Toggle
            | Action::ToggleAll
            | Action::ColumnPrev
            | Action::ColumnNext
            | Action::Down
            | Action::Up
            | Action::Top
            | Action::Bottom
            | Action::HalfPageDown
            | Action::HalfPageUp
            | Action::PageDown
            | Action::PageUp => {}
        }
        Flow::Continue
    }

    fn update_sorting(&mut self, action: Action) {
        match action {
            Action::ColumnPrev => self.list_mut().shift_sort(-1),
            Action::ColumnNext => self.list_mut().shift_sort(1),
            Action::Reverse => self.list_mut().toggle_reverse(),
            // Leaving the mode is all `s`, Esc and Enter do here.
            Action::Sort | Action::Back | Action::Enter => self.sorting = false,
            _ => {}
        }
    }

    fn update_typing(&mut self, action: Action) {
        match action {
            Action::Char(c) => self.list_mut().push(c),
            Action::Backspace => self.list_mut().pop(),
            // Both leave the box. The text stays, so the filter stays.
            Action::Enter | Action::Back => self.typing = false,
            _ => {}
        }
    }

    /// Drop out one level. At the top level this does nothing.
    fn back(&mut self) {
        self.sorting = false;
        if self.level == Level::Refs {
            self.level = Level::Remotes;
            self.chosen = None;
            self.refs.clear();
            self.ref_list = List::default();
        }
    }

    fn enter(&mut self, load: &mut Loader<'_>) -> Flow {
        let Some(index) = self.list().selected() else {
            return Flow::Continue;
        };
        match self.level {
            Level::Remotes => {
                match load(&self.remotes[index]) {
                    Ok(loaded) if loaded.refs.is_empty() && loaded.hidden == 0 => {
                        self.error =
                            Some(Error::NoRefs(self.remotes[index].name.clone()).to_string());
                    }
                    Ok(Loaded { refs, hidden, note }) => {
                        let cells = refs
                            .iter()
                            .map(|r| {
                                vec![r.name.clone(), r.oid.get(..7).unwrap_or(&r.oid).to_owned()]
                            })
                            .collect();
                        let keys = refs
                            .iter()
                            .map(|r| SortKeys {
                                name: r.name.clone(),
                                url: r.name.clone(),
                                tags: String::new(),
                                group: 0,
                            })
                            .collect();
                        self.ref_list = List::with_cells(cells, keys)
                            .with_cycle(vec![SortOrder::Default, SortOrder::Label]);
                        self.refs = refs;
                        self.hidden = hidden;
                        if note.is_some() {
                            self.notice = note;
                        }
                        self.chosen = Some(index);
                        self.level = Level::Refs;
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
                Flow::Continue
            }
            Level::Refs => {
                let chosen = &self.remotes[self.chosen.unwrap_or_default()];
                Flow::Done(Selection {
                    remote: chosen.name.clone(),
                    url: chosen.url.clone(),
                    refname: self.refs[index].name.clone(),
                })
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_wraps)] // stub loaders must match `Loader`'s signature
mod tests {
    use super::*;
    use crate::tui::keys::{Command, Mode};

    fn app() -> App {
        App::new(vec![
            Remote {
                name: "origin".into(),
                url: "u1".into(),
                tags: Vec::new(),
            },
            Remote {
                name: "up".into(),
                url: "u2".into(),
                tags: Vec::new(),
            },
        ])
    }

    #[test]
    fn rows_show_tags_only_for_collected_remotes() {
        let a = App::new(vec![
            Remote {
                name: "origin".into(),
                url: "u".into(),
                tags: Vec::new(),
            },
            Remote {
                name: "netdev/net".into(),
                url: "v".into(),
                tags: vec!["merge".into(), "maintainers".into()],
            },
        ]);
        assert_eq!(
            a.list().visible_labels(),
            ["origin  u", "netdev/net  v  merge,maintainers"]
        );
    }

    #[test]
    fn counted_moves_and_goto_work_in_the_picker() {
        let mut a = App::new(
            (0..10)
                .map(|i| Remote {
                    name: format!("r{i}"),
                    url: format!("u{i}"),
                    tags: Vec::new(),
                })
                .collect(),
        );
        a.update_with(
            Command {
                action: Action::Down,
                count: Some(4),
            },
            &mut refs,
        );
        assert_eq!(a.list().cursor(), 4);
        a.update_with(
            Command {
                action: Action::Bottom,
                count: Some(2),
            },
            &mut refs,
        );
        assert_eq!(a.list().cursor(), 1);
        a.set_viewport(6);
        a.update_with(
            Command {
                action: Action::PageDown,
                count: None,
            },
            &mut refs,
        );
        assert_eq!(a.list().cursor(), 6);
    }

    #[test]
    fn sort_mode_moves_across_columns_and_keeps_real_remotes_on_top() {
        let mut a = App::new(vec![
            Remote {
                name: "origin".into(),
                url: "https://z/o".into(),
                tags: Vec::new(),
            },
            Remote {
                name: "zeta".into(),
                url: "https://a/z".into(),
                tags: vec!["merge".into()],
            },
            Remote {
                name: "alpha".into(),
                url: "https://b/a".into(),
                tags: vec!["merge".into()],
            },
        ]);
        a.update(Action::Sort, &mut refs);
        assert!(a.sorting());
        a.update(Action::ColumnNext, &mut refs);
        assert_eq!(a.list().visible(), [0, 2, 1]);
        a.update(Action::ColumnNext, &mut refs);
        assert_eq!(a.list().visible(), [0, 1, 2]);
        assert_eq!(a.list().sort_order().name(), "url");
    }

    #[test]
    fn r_reverses_inside_groups_and_keeps_real_remotes_on_top() {
        let mut a = App::new(vec![
            Remote {
                name: "origin".into(),
                url: "https://z/o".into(),
                tags: Vec::new(),
            },
            Remote {
                name: "alpha".into(),
                url: "https://b/a".into(),
                tags: vec!["merge".into()],
            },
            Remote {
                name: "zeta".into(),
                url: "https://a/z".into(),
                tags: vec!["merge".into()],
            },
        ]);
        a.update(Action::Reverse, &mut refs);
        assert_eq!(a.list().visible(), [0, 2, 1]);
        assert!(a.list().reversed());
    }

    #[test]
    fn the_refs_list_has_default_and_name_columns_only() {
        let mut a = app();
        a.update(Action::Enter, &mut refs);
        assert_eq!(a.level(), Level::Refs);
        assert_eq!(a.list().visible(), [0, 1]);
        a.update(Action::Sort, &mut refs);
        a.update(Action::ColumnNext, &mut refs);
        assert_eq!(a.list().sort_order().name(), "label");
        assert_eq!(a.list().visible(), [1, 0]);
        a.update(Action::ColumnNext, &mut refs);
        assert_eq!(a.list().sort_order().name(), "label");
        a.update(Action::Reverse, &mut refs);
        assert_eq!(a.list().visible(), [0, 1]);
    }

    #[test]
    fn esc_and_enter_leave_sort_mode_without_going_back_or_in() {
        let mut a = app();
        a.update(Action::Sort, &mut refs);
        a.update(Action::Back, &mut refs);
        assert!(!a.sorting());
        assert_eq!(a.level(), Level::Remotes);
        a.update(Action::Sort, &mut refs);
        a.update(Action::Enter, &mut refs);
        assert!(!a.sorting());
        assert_eq!(a.level(), Level::Remotes);
        assert_eq!(a.mode(), Mode::Normal);
        a.update(Action::Sort, &mut refs);
        assert_eq!(a.mode(), Mode::Sort);
    }

    #[test]
    fn s_again_leaves_sort_mode_and_motions_still_work_inside_it() {
        let mut a = app();
        a.update(Action::Sort, &mut refs);
        a.update(Action::Down, &mut refs);
        assert_eq!(a.list().cursor(), 1);
        a.update(Action::Sort, &mut refs);
        assert!(!a.sorting());
    }

    #[test]
    fn the_notice_clears_on_the_first_action() {
        let mut a = app().with_notice(Some("2 files missing".into()));
        assert_eq!(a.notice(), Some("2 files missing"));
        a.update(Action::Down, &mut refs);
        assert_eq!(a.notice(), None);
    }

    fn refs(_: &Remote) -> Result<Loaded> {
        Ok(vec![
            RemoteRef {
                name: "refs/heads/main".into(),
                oid: "abcdef0123".into(),
            },
            RemoteRef {
                name: "refs/heads/dev".into(),
                oid: "1234567890".into(),
            },
        ]
        .into())
    }

    #[test]
    fn esc_at_top_level_does_not_exit() {
        let mut a = app();
        assert_eq!(a.update(Action::Back, &mut refs), Flow::Continue);
        assert_eq!(a.level(), Level::Remotes);
    }

    #[test]
    fn q_quits_from_any_level() {
        let mut a = app();
        assert_eq!(a.update(Action::Quit, &mut refs), Flow::Quit);
        let mut a = app();
        a.update(Action::Enter, &mut refs);
        assert_eq!(a.level(), Level::Refs);
        assert_eq!(a.update(Action::Quit, &mut refs), Flow::Quit);
    }

    #[test]
    fn enter_drops_in_and_esc_drops_out() {
        let mut a = app();
        a.update(Action::Enter, &mut refs);
        assert_eq!(a.level(), Level::Refs);
        a.update(Action::Back, &mut refs);
        assert_eq!(a.level(), Level::Remotes);
    }

    #[test]
    fn selecting_a_ref_finishes() {
        let mut a = app();
        a.update(Action::Down, &mut refs);
        a.update(Action::Enter, &mut refs);
        a.update(Action::Down, &mut refs);
        let flow = a.update(Action::Enter, &mut refs);
        assert_eq!(
            flow,
            Flow::Done(Selection {
                remote: "up".into(),
                url: "u2".into(),
                refname: "refs/heads/dev".into()
            })
        );
    }

    #[test]
    fn search_filters_and_c_clears() {
        let mut a = app();
        a.update(Action::Search, &mut refs);
        a.update(Action::Char('u'), &mut refs);
        a.update(Action::Char('p'), &mut refs);
        a.update(Action::Enter, &mut refs);
        assert!(!a.typing());
        assert_eq!(a.list().visible(), [1]);
        a.update(Action::Clear, &mut refs);
        assert_eq!(a.list().visible().len(), 2);
    }

    #[test]
    fn esc_and_enter_leave_the_search_box_and_keep_the_text() {
        for leave in [Action::Back, Action::Enter] {
            let mut a = app();
            a.update(Action::Enter, &mut refs);
            a.update(Action::Search, &mut refs);
            a.update(Action::Char('v'), &mut refs);
            a.update(leave, &mut refs);
            assert!(!a.typing());
            assert_eq!(a.list().filter(), "v");
            assert_eq!(a.level(), Level::Refs, "stays on the refs list");
            assert_eq!(a.list().visible().len(), 1, "the filter still applies");
        }
    }

    #[test]
    fn editing_again_continues_from_the_kept_text() {
        let mut a = app();
        a.update(Action::Search, &mut refs);
        a.update(Action::Char('u'), &mut refs);
        a.update(Action::Back, &mut refs);
        a.update(Action::Search, &mut refs);
        a.update(Action::Char('p'), &mut refs);
        assert_eq!(a.list().filter(), "up");
        a.update(Action::Back, &mut refs);
        a.update(Action::Clear, &mut refs);
        assert_eq!(a.list().filter(), "");
    }

    #[test]
    fn hidden_refs_are_counted_and_the_load_note_shows() {
        let mut a = app();
        let mut load = |_: &Remote| {
            Ok(Loaded {
                refs: vec![RemoteRef {
                    name: "refs/heads/dev".into(),
                    oid: "1234567890".into(),
                }],
                hidden: 3,
                note: Some("stale".into()),
            })
        };
        a.update(Action::Enter, &mut load);
        assert_eq!(a.level(), Level::Refs);
        assert_eq!(a.hidden(), 3);
        assert_eq!(a.notice(), Some("stale"));
    }

    #[test]
    fn a_remote_whose_refs_are_all_hidden_still_opens() {
        let mut a = app();
        let mut load = |_: &Remote| {
            Ok(Loaded {
                refs: Vec::new(),
                hidden: 5,
                note: None,
            })
        };
        a.update(Action::Enter, &mut load);
        assert_eq!(a.level(), Level::Refs);
        assert!(a.error().is_none());
    }

    #[test]
    fn load_failure_stays_put_and_reports() {
        let mut a = app();
        let mut fail = |_: &Remote| Err(Error::NoRefs("origin".into()));
        a.update(Action::Enter, &mut fail);
        assert_eq!(a.level(), Level::Remotes);
        assert!(a.error().is_some());
    }
}

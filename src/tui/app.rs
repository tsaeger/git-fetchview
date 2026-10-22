// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Picker state machine: remotes, then refs. Pure, so it is easy to test.

use crate::error::{Error, Result};
use crate::git::{Remote, RemoteRef, redact_url};
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
    /// A remote was chosen. List its refs, then call [`App::finish_load`]
    /// with the same id.
    Load(Load),
    /// Stop the load that was running; the remote list stays up.
    CancelLoad,
    /// The question was answered yes: remove this collected remote, then call
    /// [`App::finish_remove`].
    Remove(Remote),
    /// A remote and ref were chosen.
    Done(Selection),
}

/// A request to list the refs of a remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Load {
    /// Identifies this request, so a late answer to a cancelled one is dropped.
    pub id: u64,
    /// The remote to list.
    pub remote: Remote,
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

/// Fetches the refs of a remote (`git ls-remote`), inline. The event loop does
/// this on another thread instead; this is for tests and simple callers.
pub type Loader<'a> = dyn FnMut(&Remote) -> Result<Loaded> + 'a;

fn remote_cells(r: &Remote) -> Vec<String> {
    // The screen can be shared or photographed, like the trail.
    vec![r.name.clone(), redact_url(&r.url), r.tags.join(",")]
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
    /// The load in flight: its id and the remote's index.
    loading: Option<(u64, usize)>,
    /// The collected remote a remove question is open for.
    confirming: Option<usize>,
    next_load: u64,
    spinner: usize,
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
            loading: None,
            confirming: None,
            next_load: 0,
            spinner: 0,
        }
    }

    /// Show a message in the status line until the next key press.
    #[must_use]
    pub fn with_notice(mut self, notice: Option<String>) -> Self {
        self.notice = notice;
        self
    }

    /// Name of the remote whose refs are being listed, if a load is running.
    #[must_use]
    pub fn loading(&self) -> Option<&str> {
        self.loading.map(|(_, i)| self.remotes[i].name.as_str())
    }

    /// Name of the collected remote a remove question is open for, if any.
    #[must_use]
    pub fn confirming(&self) -> Option<&str> {
        self.confirming.map(|i| self.remotes[i].name.as_str())
    }

    /// Which frame of the loading spinner to draw.
    #[must_use]
    pub fn spinner(&self) -> usize {
        self.spinner
    }

    /// Advance the spinner. Call it on a timer while [`App::loading`] is set.
    pub fn tick(&mut self) {
        self.spinner = self.spinner.wrapping_add(1);
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
        if self.confirming.is_some() {
            Mode::Confirm
        } else if self.typing {
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

    /// Apply one action, listing refs inline if it asks for them.
    pub fn update(&mut self, action: Action, load: &mut Loader<'_>) -> Flow {
        self.update_with(Command::from(action), load)
    }

    /// Apply one action with the count typed before it, listing refs inline
    /// if it asks for them.
    pub fn update_with(&mut self, command: Command, load: &mut Loader<'_>) -> Flow {
        match self.step(command) {
            Flow::Load(request) => {
                let result = load(&request.remote);
                self.finish_load(request.id, result);
                Flow::Continue
            }
            flow => flow,
        }
    }

    /// Apply one action with the count typed before it. A remote being chosen
    /// comes back as [`Flow::Load`], and the answer goes to
    /// [`App::finish_load`].
    pub fn step(&mut self, command: Command) -> Flow {
        let Command { action, count } = command;
        self.error = None;
        self.notice = None;
        if action == Action::Quit {
            return Flow::Quit;
        }
        if let Some(index) = self.confirming.take() {
            return match action {
                Action::Confirm => Flow::Remove(self.remotes[index].clone()),
                Action::Interrupt => Flow::Quit,
                _ => Flow::Continue,
            };
        }
        if self.loading.is_some() {
            return self.step_loading(action);
        }
        if action == Action::Interrupt {
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
            Action::Delete => self.ask_remove(),
            Action::Enter => return self.enter(),
            Action::Quit
            | Action::Interrupt
            | Action::Confirm
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

    /// While refs are loading only `Esc`, `h` and Ctrl-C do anything (they
    /// give up). `q` quits as everywhere else, and other keys are ignored.
    fn step_loading(&mut self, action: Action) -> Flow {
        if matches!(action, Action::Back | Action::Interrupt) {
            self.loading = None;
            Flow::CancelLoad
        } else {
            Flow::Continue
        }
    }

    /// `d`: open the question for a collected remote. A real remote belongs to
    /// git, so it gets a note instead.
    fn ask_remove(&mut self) {
        if self.level != Level::Remotes {
            return;
        }
        let Some(index) = self.list().selected() else {
            return;
        };
        let remote = &self.remotes[index];
        if remote.tags.is_empty() {
            self.notice = Some(format!(
                "{0} is a real remote: git remote remove {0}",
                remote.name
            ));
        } else {
            self.confirming = Some(index);
        }
    }

    /// Take the result of a [`Flow::Remove`]. On success the row goes away.
    pub fn finish_remove(&mut self, url: &str, result: Result<()>) {
        match result {
            Ok(()) => {
                if let Some(index) = self.remotes.iter().position(|r| r.url == url) {
                    let gone = self.remotes.remove(index);
                    self.remote_list.remove(index);
                    self.notice = Some(format!("removed {}", gone.name));
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Take the answer to a [`Flow::Load`]. An answer to a load that was
    /// cancelled, or replaced by a newer one, is dropped.
    pub fn finish_load(&mut self, id: u64, result: Result<Loaded>) {
        let Some((current, index)) = self.loading else {
            return;
        };
        if current != id {
            return;
        }
        self.loading = None;
        match result {
            Ok(loaded) if loaded.refs.is_empty() && loaded.hidden == 0 => {
                self.error = Some(Error::NoRefs(self.remotes[index].name.clone()).to_string());
            }
            Ok(Loaded { refs, hidden, note }) => {
                let cells = refs
                    .iter()
                    .map(|r| vec![r.name.clone(), r.oid.get(..7).unwrap_or(&r.oid).to_owned()])
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

    fn enter(&mut self) -> Flow {
        let Some(index) = self.list().selected() else {
            return Flow::Continue;
        };
        match self.level {
            Level::Remotes => {
                let id = self.next_load;
                self.next_load += 1;
                self.loading = Some((id, index));
                self.spinner = 0;
                Flow::Load(Load {
                    id,
                    remote: self.remotes[index].clone(),
                })
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

    fn refs_of(names: &[&str]) -> Loaded {
        Loaded::from(
            names
                .iter()
                .map(|n| RemoteRef {
                    name: (*n).into(),
                    oid: "0123456789abcdef".into(),
                })
                .collect::<Vec<_>>(),
        )
    }

    fn load_request(flow: Flow) -> Load {
        match flow {
            Flow::Load(load) => load,
            other => panic!("expected a load, got {other:?}"),
        }
    }

    #[test]
    fn enter_on_a_remote_asks_for_a_load_and_waits_for_it() {
        let mut a = app();
        let load = load_request(a.step(Command::from(Action::Enter)));
        assert_eq!(load.remote.name, "origin");
        assert_eq!(a.level(), Level::Remotes);
        assert_eq!(a.loading(), Some("origin"));
        a.finish_load(load.id, Ok(refs_of(&["refs/heads/main"])));
        assert_eq!(a.level(), Level::Refs);
        assert_eq!(a.loading(), None);
    }

    #[test]
    fn esc_while_loading_cancels_and_a_late_result_is_ignored() {
        let mut a = app();
        let load = load_request(a.step(Command::from(Action::Enter)));
        assert_eq!(a.step(Command::from(Action::Back)), Flow::CancelLoad);
        assert_eq!(a.loading(), None);
        a.finish_load(load.id, Ok(refs_of(&["refs/heads/main"])));
        assert_eq!(a.level(), Level::Remotes);
    }

    #[test]
    fn ctrl_c_cancels_a_load_and_quits_otherwise() {
        let mut a = app();
        a.step(Command::from(Action::Enter));
        assert_eq!(a.step(Command::from(Action::Interrupt)), Flow::CancelLoad);
        assert_eq!(a.step(Command::from(Action::Interrupt)), Flow::Quit);
    }

    #[test]
    fn q_still_quits_while_loading() {
        let mut a = app();
        a.step(Command::from(Action::Enter));
        assert_eq!(a.step(Command::from(Action::Quit)), Flow::Quit);
    }

    #[test]
    fn keys_do_nothing_while_loading() {
        let mut a = app();
        a.step(Command::from(Action::Enter));
        for action in [Action::Down, Action::Search, Action::Sort, Action::Enter] {
            assert_eq!(a.step(Command::from(action)), Flow::Continue);
        }
        assert_eq!(a.list().cursor(), 0);
        assert!(!a.typing());
        assert_eq!(a.loading(), Some("origin"));
    }

    #[test]
    fn a_failed_load_shows_its_error_and_stops_waiting() {
        let mut a = app();
        let load = load_request(a.step(Command::from(Action::Enter)));
        a.finish_load(load.id, Err(Error::NoRefs("origin".into())));
        assert_eq!(a.loading(), None);
        assert_eq!(a.level(), Level::Remotes);
        assert!(a.error().is_some_and(|e| e.contains("no refs")));
    }

    #[test]
    fn a_result_for_an_older_load_is_ignored() {
        let mut a = app();
        let first = load_request(a.step(Command::from(Action::Enter)));
        a.step(Command::from(Action::Back));
        let second = load_request(a.step(Command::from(Action::Enter)));
        assert_ne!(first.id, second.id);
        a.finish_load(first.id, Ok(refs_of(&["refs/heads/old"])));
        assert_eq!(a.level(), Level::Remotes);
        a.finish_load(second.id, Ok(refs_of(&["refs/heads/new"])));
        assert_eq!(a.level(), Level::Refs);
    }

    #[test]
    fn the_spinner_advances_on_each_tick() {
        let mut a = app();
        let before = a.spinner();
        a.tick();
        assert_ne!(a.spinner(), before);
    }

    fn collected_app() -> App {
        App::new(vec![
            Remote {
                name: "origin".into(),
                url: "https://h/o".into(),
                tags: Vec::new(),
            },
            Remote {
                name: "netdev/net".into(),
                url: "https://h/net".into(),
                tags: vec!["merge".into()],
            },
            Remote {
                name: "usb/usb".into(),
                url: "https://h/usb".into(),
                tags: vec!["merge".into()],
            },
        ])
    }

    fn on_row(a: &mut App, n: usize) {
        for _ in 0..n {
            a.step(Command::from(Action::Down));
        }
    }

    #[test]
    fn d_on_a_collected_row_asks_before_removing() {
        let mut a = collected_app();
        on_row(&mut a, 1);
        assert_eq!(a.step(Command::from(Action::Delete)), Flow::Continue);
        assert_eq!(a.confirming(), Some("netdev/net"));
        assert_eq!(a.mode(), Mode::Confirm);
    }

    #[test]
    fn y_asks_the_loop_to_remove_it_and_anything_else_cancels() {
        let mut a = collected_app();
        on_row(&mut a, 1);
        a.step(Command::from(Action::Delete));
        let Flow::Remove(remote) = a.step(Command::from(Action::Confirm)) else {
            panic!("expected a removal");
        };
        assert_eq!(remote.url, "https://h/net");
        assert_eq!(a.confirming(), None);

        a.step(Command::from(Action::Delete));
        assert_eq!(a.step(Command::from(Action::Back)), Flow::Continue);
        assert_eq!(a.confirming(), None);
        assert_eq!(a.list().visible().len(), 3);
    }

    #[test]
    fn d_on_a_real_remote_only_shows_a_note() {
        let mut a = collected_app();
        assert_eq!(a.step(Command::from(Action::Delete)), Flow::Continue);
        assert_eq!(a.confirming(), None);
        assert!(
            a.notice()
                .is_some_and(|n| n.contains("git remote remove origin"))
        );
    }

    #[test]
    fn d_in_the_refs_list_does_nothing() {
        let mut a = collected_app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        a.update(Action::Enter, &mut load);
        assert_eq!(a.level(), Level::Refs);
        a.step(Command::from(Action::Delete));
        assert_eq!(a.confirming(), None);
    }

    #[test]
    fn a_finished_removal_drops_the_row_and_a_failed_one_keeps_it() {
        let mut a = collected_app();
        on_row(&mut a, 1);
        a.finish_remove("https://h/net", Ok(()));
        assert_eq!(a.list().visible().len(), 2);
        assert!(a.notice().is_some_and(|n| n.contains("netdev/net")));
        let under_cursor = a.list().visible_cells()[a.list().cursor()].1[0].clone();
        assert_eq!(under_cursor, "usb/usb");

        a.finish_remove("https://h/usb", Err(Error::NotFound("usb/usb".into())));
        assert_eq!(a.list().visible().len(), 2);
        assert!(a.error().is_some());
    }

    #[test]
    fn ctrl_c_during_the_question_quits() {
        let mut a = collected_app();
        on_row(&mut a, 1);
        a.step(Command::from(Action::Delete));
        assert_eq!(a.step(Command::from(Action::Interrupt)), Flow::Quit);
    }
}

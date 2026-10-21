// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Picker state machine: remotes, then refs. Pure, so it is easy to test.

use crate::error::{Error, Result};
use crate::git::{Remote, RemoteRef};
use crate::tui::keys::Action;
use crate::tui::list::List;

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

/// Fetches the refs of a remote (`git ls-remote`).
pub type Loader<'a> = dyn FnMut(&Remote) -> Result<Vec<RemoteRef>> + 'a;

/// Picker state.
#[derive(Debug)]
pub struct App {
    remotes: Vec<Remote>,
    remote_list: List,
    refs: Vec<RemoteRef>,
    ref_list: List,
    level: Level,
    typing: bool,
    error: Option<String>,
    chosen: Option<usize>,
}

impl App {
    /// Start at the remote picker.
    #[must_use]
    pub fn new(remotes: Vec<Remote>) -> Self {
        let labels = remotes
            .iter()
            .map(|r| format!("{}  {}", r.name, r.url))
            .collect();
        Self {
            remotes,
            remote_list: List::new(labels),
            refs: Vec::new(),
            ref_list: List::default(),
            level: Level::Remotes,
            typing: false,
            error: None,
            chosen: None,
        }
    }

    /// Current level.
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
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

    /// Apply one action.
    pub fn update(&mut self, action: Action, load: &mut Loader<'_>) -> Flow {
        self.error = None;
        if action == Action::Quit {
            return Flow::Quit;
        }
        if self.typing {
            self.update_typing(action);
            return Flow::Continue;
        }
        match action {
            Action::Down => self.list_mut().down(),
            Action::Up => self.list_mut().up(),
            Action::Top => self.list_mut().top(),
            Action::Bottom => self.list_mut().bottom(),
            Action::Search => self.typing = true,
            Action::Clear => self.list_mut().clear(),
            Action::Back => self.back(),
            Action::Enter => return self.enter(load),
            Action::Quit | Action::Char(_) | Action::Backspace => {}
        }
        Flow::Continue
    }

    fn update_typing(&mut self, action: Action) {
        match action {
            Action::Char(c) => self.list_mut().push(c),
            Action::Backspace => self.list_mut().pop(),
            Action::Enter => self.typing = false,
            Action::Back => {
                self.list_mut().clear();
                self.typing = false;
            }
            _ => {}
        }
    }

    /// Drop out one level. At the top level this does nothing.
    fn back(&mut self) {
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
                    Ok(refs) if refs.is_empty() => {
                        self.error =
                            Some(Error::NoRefs(self.remotes[index].name.clone()).to_string());
                    }
                    Ok(refs) => {
                        let labels = refs
                            .iter()
                            .map(|r| format!("{}  {}", r.name, r.oid.get(..7).unwrap_or(&r.oid)))
                            .collect();
                        self.ref_list = List::new(labels);
                        self.refs = refs;
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

    fn app() -> App {
        App::new(vec![
            Remote {
                name: "origin".into(),
                url: "u1".into(),
            },
            Remote {
                name: "up".into(),
                url: "u2".into(),
            },
        ])
    }

    fn refs(_: &Remote) -> Result<Vec<RemoteRef>> {
        Ok(vec![
            RemoteRef {
                name: "refs/heads/main".into(),
                oid: "abcdef0123".into(),
            },
            RemoteRef {
                name: "refs/heads/dev".into(),
                oid: "1234567890".into(),
            },
        ])
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
    fn esc_while_typing_abandons_the_search_only() {
        let mut a = app();
        a.update(Action::Enter, &mut refs);
        a.update(Action::Search, &mut refs);
        a.update(Action::Char('x'), &mut refs);
        a.update(Action::Back, &mut refs);
        assert!(!a.typing());
        assert_eq!(a.list().filter(), "");
        assert_eq!(a.level(), Level::Refs);
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

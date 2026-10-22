// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Picker state machine: remotes, then refs. Pure, so it is easy to test.

use crate::cli::AddSource;
use crate::error::{Error, Result};
use crate::git::{Remote, RemoteRef, redact_url};
use crate::remotes::entry::{Entry, key as repo_key, validate_url};
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
    /// Add this URL to the default remotes file, then call
    /// [`App::finish_add`].
    Add(String),
    /// Look for remotes in this source, let the user choose, and call
    /// [`App::finish_scan`] with what was added.
    Scan(AddSource),
    /// Run `git remote add <name> <url>`, then call [`App::finish_promote`].
    Promote {
        /// The name for the new remote.
        name: String,
        /// The collected remote's URL.
        url: String,
    },
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
    /// The refs that were left out, so they can be shown again.
    pub hidden: Vec<RemoteRef>,
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

/// The menus there are.
#[derive(Debug, Clone, Copy)]
enum Menu {
    /// Where to collect remotes from.
    Scan,
}

/// What a prompt is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    /// A URL to collect.
    AddUrl,
    /// A name for the collected remote at this index, as a real remote.
    PromoteName(usize),
    /// A URL to browse once, without saving it.
    OpenUrl,
    /// The revision range, and an optional `-n` limit, to scan for merges.
    MergesRange,
    /// A file of URLs, one per line.
    UrlFile,
}

impl PromptKind {
    fn label(self) -> &'static str {
        match self {
            Self::AddUrl => "add url",
            Self::PromoteName(_) => "remote name",
            Self::OpenUrl => "open url",
            Self::MergesRange => "merges range",
            Self::UrlFile => "url file",
        }
    }
}

/// `[-n N] [range]` from the merges prompt. The range defaults to `HEAD` and
/// must not look like an option, since it goes to `git log`.
fn parse_merges(text: &str) -> std::result::Result<(String, Option<usize>), &'static str> {
    let mut range = None;
    let mut max = None;
    let mut words = text.split_whitespace();
    while let Some(word) = words.next() {
        let count = if word == "-n" {
            Some(words.next().ok_or("-n needs a number")?)
        } else {
            word.strip_prefix("-n").filter(|rest| !rest.is_empty())
        };
        if let Some(count) = count {
            max = Some(
                count
                    .parse::<usize>()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or("-n needs a number above 0")?,
            );
        } else if word.starts_with('-') {
            return Err("only -n is understood");
        } else if range.replace(word).is_some() {
            return Err("give one range");
        }
    }
    Ok((range.unwrap_or("HEAD").to_owned(), max))
}

/// One line for the status bar: what git said, not the whole command line.
fn short_reason(e: &Error) -> String {
    if let Error::Failed { stderr, .. } = e
        && let Some(line) = stderr.lines().rev().find(|l| !l.trim().is_empty())
    {
        return line.trim().trim_start_matches("fatal: ").to_owned();
    }
    e.to_string()
}

/// A remote name to suggest for a label: `netdev/net-next` is
/// `netdev-net-next`, and anything odd becomes a dash.
fn suggested_name(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(|c| c == '-' || c == '.')
        .to_owned()
}

#[derive(Debug)]
struct Prompt {
    kind: PromptKind,
    text: String,
}

fn remote_keys(r: &Remote) -> SortKeys {
    SortKeys {
        name: r.name.clone(),
        url: r.url.clone(),
        tags: r.tags.join(","),
        // Real remotes stay above collected ones.
        group: u8::from(!r.tags.is_empty()),
    }
}

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
    /// The refs shown by default, kept while the hidden ones are on show.
    default_refs: Vec<RemoteRef>,
    hidden_refs: Vec<RemoteRef>,
    showing_hidden: bool,
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
    prompt: Option<Prompt>,
    /// Index of a remote typed at the `o` prompt, kept only while it is open.
    /// It is always the last one, and never in the list.
    temp: Option<usize>,
    /// The scan menu is open.
    menu: Option<Menu>,
    /// What a scan in progress is looking at, for the status line.
    scanning: Option<&'static str>,
    next_load: u64,
    spinner: usize,
}

impl App {
    /// Start at the remote picker.
    #[must_use]
    pub fn new(remotes: Vec<Remote>) -> Self {
        let cells = remotes.iter().map(remote_cells).collect();
        let keys = remotes.iter().map(remote_keys).collect();
        Self {
            remotes,
            remote_list: List::with_cells(cells, keys),
            refs: Vec::new(),
            default_refs: Vec::new(),
            hidden_refs: Vec::new(),
            showing_hidden: false,
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
            prompt: None,
            temp: None,
            menu: None,
            scanning: None,
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

    /// The scan menu's items, if it is open.
    #[must_use]
    pub fn menu(&self) -> Option<&'static str> {
        self.menu
            .map(|Menu::Scan| "m merges  t maintainers  f url file")
    }

    /// What a scan in progress is looking at, if one is.
    #[must_use]
    pub fn scanning(&self) -> Option<&'static str> {
        self.scanning
    }

    /// Note that a scan of `what` is starting, so it can be drawn first.
    pub fn begin_scan(&mut self, what: &'static str) {
        self.scanning = Some(what);
    }

    /// The open prompt's label and text, if one is open.
    #[must_use]
    pub fn prompt(&self) -> Option<(&'static str, &str)> {
        self.prompt
            .as_ref()
            .map(|p| (p.kind.label(), p.text.as_str()))
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
        self.hidden_refs.len()
    }

    /// Whether the hidden refs are being shown with the rest.
    #[must_use]
    pub fn showing_hidden(&self) -> bool {
        self.showing_hidden
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
        } else if self.menu.is_some() {
            Mode::Menu
        } else if self.prompt.is_some() {
            Mode::Prompt
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
        if self.menu.is_some() {
            return self.step_menu(action);
        }
        if self.prompt.is_some() {
            return self.step_prompt(action);
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
            Action::New => self.open_prompt(PromptKind::AddUrl),
            Action::Promote => self.ask_promote(),
            Action::Open => self.open_prompt(PromptKind::OpenUrl),
            Action::Scan => self.menu = (self.level == Level::Remotes).then_some(Menu::Scan),
            Action::ShowHidden => self.toggle_hidden(),
            Action::Enter => return self.enter(),
            Action::Quit
            | Action::Interrupt
            | Action::Confirm
            | Action::ToggleRegex
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

    /// `n`: ask for a URL. Only the remote list can take one.
    fn open_prompt(&mut self, kind: PromptKind) {
        if self.level == Level::Remotes {
            self.prompt = Some(Prompt {
                kind,
                text: String::new(),
            });
        }
    }

    /// One letter picks where to collect from, anything else closes the menu.
    fn step_menu(&mut self, action: Action) -> Flow {
        self.menu = None;
        match action {
            Action::Char('m') => {
                self.prompt = Some(Prompt {
                    kind: PromptKind::MergesRange,
                    text: "HEAD -n 500".to_owned(),
                });
                Flow::Continue
            }
            Action::Char('t') => Flow::Scan(AddSource::Maintainers { path: None }),
            Action::Char('f') => {
                self.open_prompt(PromptKind::UrlFile);
                Flow::Continue
            }
            Action::Interrupt => Flow::Quit,
            _ => Flow::Continue,
        }
    }

    /// `p`: ask for a name to make a collected remote a real one.
    fn ask_promote(&mut self) {
        if self.level != Level::Remotes {
            return;
        }
        let Some(index) = self.list().selected() else {
            return;
        };
        let remote = &self.remotes[index];
        if remote.tags.is_empty() {
            self.notice = Some(format!("{} is already a real remote", remote.name));
        } else {
            self.prompt = Some(Prompt {
                kind: PromptKind::PromoteName(index),
                text: suggested_name(&remote.name),
            });
        }
    }

    /// Take the result of a [`Flow::Promote`]. On success the collected row
    /// becomes a real remote, in the real group. On failure the prompt stays,
    /// so the name can be fixed.
    pub fn finish_promote(&mut self, url: &str, name: &str, result: Result<()>) {
        if let Err(e) = result {
            self.error = Some(short_reason(&e));
            return;
        }
        self.prompt = None;
        if let Some(index) = self.remotes.iter().position(|r| r.url == url) {
            self.remotes.remove(index);
            self.remote_list.remove(index);
        }
        let remote = Remote {
            name: name.to_owned(),
            url: url.to_owned(),
            tags: Vec::new(),
        };
        let at = self
            .remotes
            .iter()
            .take_while(|r| r.tags.is_empty())
            .count();
        self.remote_list
            .insert(at, remote_cells(&remote), remote_keys(&remote));
        self.remotes.insert(at, remote);
        self.remote_list.focus(at);
        self.notice = Some(format!("added remote {name}"));
    }

    fn step_prompt(&mut self, action: Action) -> Flow {
        let Some(prompt) = self.prompt.as_mut() else {
            return Flow::Continue;
        };
        match action {
            Action::Char(c) => prompt.text.push(c),
            Action::Backspace => {
                prompt.text.pop();
            }
            Action::Back => self.prompt = None,
            Action::Enter => return self.submit_prompt(),
            Action::Interrupt => return Flow::Quit,
            _ => {}
        }
        Flow::Continue
    }

    /// Enter in the prompt. A bad URL says why and leaves the prompt open. A
    /// repo that is already listed gets the cursor instead of a second row.
    fn submit_prompt(&mut self) -> Flow {
        let Some(prompt) = self.prompt.as_ref() else {
            return Flow::Continue;
        };
        let text = prompt.text.trim().to_owned();
        let kind = prompt.kind;
        match kind {
            PromptKind::PromoteName(index) => return self.submit_name(index, &text),
            PromptKind::MergesRange => return self.submit_merges(&text),
            PromptKind::UrlFile => return self.submit_file(&text),
            PromptKind::AddUrl | PromptKind::OpenUrl => {}
        }
        match validate_url(&text) {
            Err(why) => {
                self.error = Some(format!("not a usable URL: {why}"));
                Flow::Continue
            }
            Ok(url) => {
                self.prompt = None;
                let wanted = repo_key(&url);
                let listed = self.remotes.iter().position(|r| repo_key(&r.url) == wanted);
                match (kind, listed) {
                    (PromptKind::OpenUrl, Some(i)) => {
                        self.remote_list.focus(i);
                        self.begin_load(i)
                    }
                    (PromptKind::OpenUrl, None) => {
                        // Held only while it is open, and never in the list.
                        let at = self.remotes.len();
                        self.remotes.push(Remote {
                            name: url.clone(),
                            url,
                            tags: Vec::new(),
                        });
                        self.temp = Some(at);
                        self.begin_load(at)
                    }
                    (_, Some(i)) => {
                        self.remote_list.focus(i);
                        self.notice = Some(format!("already listed as {}", self.remotes[i].name));
                        Flow::Continue
                    }
                    (_, None) => Flow::Add(url),
                }
            }
        }
    }

    /// Enter in the merges prompt: `[-n N] [range]`, the range defaulting to
    /// `HEAD`.
    fn submit_merges(&mut self, text: &str) -> Flow {
        match parse_merges(text) {
            Ok((range, max)) => {
                self.prompt = None;
                Flow::Scan(AddSource::Merges {
                    max,
                    range: Some(range),
                })
            }
            Err(why) => {
                self.error = Some(why.to_owned());
                Flow::Continue
            }
        }
    }

    /// Enter in the url-file prompt.
    fn submit_file(&mut self, text: &str) -> Flow {
        if text.is_empty() {
            self.error = Some("enter a file name".to_owned());
        } else if text == "-" {
            self.error = Some("stdin is the terminal here; name a file".to_owned());
        } else {
            self.prompt = None;
            return Flow::Scan(AddSource::Url {
                from_file: Some(text.to_owned()),
                urls: Vec::new(),
            });
        }
        Flow::Continue
    }

    /// Enter in the name prompt. Git checks the name for real; this only
    /// refuses what would be misread as an option or clash with a remote.
    fn submit_name(&mut self, index: usize, name: &str) -> Flow {
        let problem = if name.is_empty() {
            Some("enter a name")
        } else if name.starts_with('-') {
            Some("a name cannot start with '-'")
        } else if name.chars().any(char::is_whitespace) {
            Some("a name cannot contain spaces")
        } else if self.remotes.iter().any(|r| r.name == name) {
            Some("a remote with that name exists")
        } else {
            None
        };
        if let Some(problem) = problem {
            self.error = Some(problem.to_owned());
            return Flow::Continue;
        }
        Flow::Promote {
            name: name.to_owned(),
            url: self.remotes[index].url.clone(),
        }
    }

    /// Put a collected remote in its place: real remotes first, then collected
    /// ones by label. Returns its index.
    fn insert_collected(&mut self, entry: Entry) -> usize {
        let remote = Remote {
            name: entry.display_label(),
            url: entry.url,
            tags: entry.tags.into_iter().collect(),
        };
        let first_collected = self
            .remotes
            .iter()
            .take_while(|r| r.tags.is_empty())
            .count();
        let at = self.remotes[first_collected..]
            .iter()
            .position(|r| (&r.name, &r.url) > (&remote.name, &remote.url))
            .map_or(self.remotes.len(), |p| first_collected + p);
        self.remote_list
            .insert(at, remote_cells(&remote), remote_keys(&remote));
        self.remotes.insert(at, remote);
        at
    }

    /// Take the result of a [`Flow::Add`]. On success the row appears in its
    /// place and the cursor goes to it.
    pub fn finish_add(&mut self, result: Result<Entry>) {
        match result {
            Ok(entry) => {
                let name = entry.display_label();
                let at = self.insert_collected(entry);
                self.remote_list.focus(at);
                self.notice = Some(format!("added {name}"));
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Take the result of a [`Flow::Scan`]: the entries that were written.
    /// Their rows appear, except for repos that are already listed.
    pub fn finish_scan(&mut self, result: Result<Vec<Entry>>) {
        self.scanning = None;
        match result {
            Ok(entries) => {
                let mut added = 0;
                for entry in entries {
                    let wanted = repo_key(&entry.url);
                    if !self.remotes.iter().any(|r| repo_key(&r.url) == wanted) {
                        self.insert_collected(entry);
                        added += 1;
                    }
                }
                self.notice = Some(format!("added {added}"));
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// While refs are loading only `Esc`, `h` and Ctrl-C do anything (they
    /// give up). `q` quits as everywhere else, and other keys are ignored.
    fn step_loading(&mut self, action: Action) -> Flow {
        if matches!(action, Action::Back | Action::Interrupt) {
            self.loading = None;
            self.forget_temp();
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
            Ok(loaded) if loaded.refs.is_empty() && loaded.hidden.is_empty() => {
                self.error = Some(Error::NoRefs(self.remotes[index].name.clone()).to_string());
                self.forget_temp();
            }
            Ok(Loaded { refs, hidden, note }) => {
                self.default_refs.clone_from(&refs);
                self.hidden_refs = hidden;
                self.showing_hidden = false;
                self.set_refs(refs);
                if note.is_some() {
                    self.notice = note;
                }
                self.chosen = Some(index);
                self.level = Level::Refs;
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.forget_temp();
            }
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
            Action::ToggleRegex => self.list_mut().toggle_mode(),
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
            self.default_refs.clear();
            self.hidden_refs.clear();
            self.showing_hidden = false;
            self.ref_list = List::default();
            self.forget_temp();
        }
    }

    /// Show these refs in the refs list.
    fn set_refs(&mut self, refs: Vec<RemoteRef>) {
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
        self.ref_list =
            List::with_cells(cells, keys).with_cycle(vec![SortOrder::Default, SortOrder::Label]);
        self.refs = refs;
    }

    /// `H`: show the hidden refs among the others, or hide them again.
    fn toggle_hidden(&mut self) {
        if self.level != Level::Refs || self.hidden_refs.is_empty() {
            return;
        }
        self.showing_hidden = !self.showing_hidden;
        let mut refs = self.default_refs.clone();
        if self.showing_hidden {
            // Git lists refs in byte order, so this puts them where they were.
            refs.extend(self.hidden_refs.iter().cloned());
            refs.sort_by(|a, b| a.name.cmp(&b.name));
        }
        self.set_refs(refs);
    }

    /// Drop the remote typed at the `o` prompt, if one is held.
    fn forget_temp(&mut self) {
        if let Some(index) = self.temp.take() {
            self.remotes.truncate(index);
        }
    }

    /// Start listing the refs of the remote at `index`.
    fn begin_load(&mut self, index: usize) -> Flow {
        let id = self.next_load;
        self.next_load += 1;
        self.loading = Some((id, index));
        self.spinner = 0;
        Flow::Load(Load {
            id,
            remote: self.remotes[index].clone(),
        })
    }

    fn enter(&mut self) -> Flow {
        let Some(index) = self.list().selected() else {
            return Flow::Continue;
        };
        match self.level {
            Level::Remotes => self.begin_load(index),
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

    fn hidden_ref() -> RemoteRef {
        RemoteRef {
            name: "refs/pull/1/merge".into(),
            oid: "abcdef0123".into(),
        }
    }

    fn with_hidden() -> (App, Load) {
        let mut a = app();
        let load = load_request(a.step(Command::from(Action::Enter)));
        let named = |n: &str| RemoteRef {
            name: n.into(),
            oid: "0123456789".into(),
        };
        a.finish_load(
            load.id,
            Ok(Loaded {
                refs: vec![
                    named("HEAD"),
                    named("refs/heads/main"),
                    named("refs/pull/1/head"),
                ],
                hidden: vec![named("refs/pull/1/merge"), named("refs/pull/2/merge")],
                note: None,
            }),
        );
        (a, load)
    }

    fn shown_names(a: &App) -> Vec<String> {
        a.list()
            .visible_cells()
            .iter()
            .map(|(_, c)| c[0].clone())
            .collect()
    }

    #[test]
    fn the_search_takes_not_and_or_and_ctrl_r_switches_to_regex() {
        let mut a = collected_app();
        a.step(Command::from(Action::Search));
        typed(&mut a, "!origin");
        assert_eq!(a.list().visible().len(), 2);
        for _ in 0.."!origin".len() {
            a.step(Command::from(Action::Backspace));
        }
        typed(&mut a, "origin | usb");
        assert_eq!(a.list().visible().len(), 2);
        a.step(Command::from(Action::ToggleRegex));
        for _ in 0.."origin | usb".len() {
            a.step(Command::from(Action::Backspace));
        }
        typed(&mut a, "^(origin|usb/)");
        assert_eq!(a.list().visible().len(), 2);
        // Leaving the box keeps the filter and its mode.
        a.step(Command::from(Action::Back));
        assert_eq!(a.list().visible().len(), 2);
        assert_eq!(a.list().mode(), crate::search::Mode::Regex);
    }

    #[test]
    fn capital_h_shows_the_hidden_refs_in_order_and_hides_them_again() {
        let (mut a, _) = with_hidden();
        assert_eq!(a.hidden(), 2);
        assert!(!a.showing_hidden());
        assert_eq!(
            shown_names(&a),
            ["HEAD", "refs/heads/main", "refs/pull/1/head"]
        );

        a.step(Command::from(Action::ShowHidden));
        assert!(a.showing_hidden());
        assert_eq!(
            shown_names(&a),
            [
                "HEAD",
                "refs/heads/main",
                "refs/pull/1/head",
                "refs/pull/1/merge",
                "refs/pull/2/merge"
            ]
        );
        // A hidden ref can be picked while shown.
        a.step(Command::from(Action::Bottom));
        let Flow::Done(selection) = a.step(Command::from(Action::Enter)) else {
            panic!("expected a selection");
        };
        assert_eq!(selection.refname, "refs/pull/2/merge");

        a.step(Command::from(Action::ShowHidden));
        assert!(!a.showing_hidden());
        assert_eq!(shown_names(&a).len(), 3);
    }

    #[test]
    fn capital_h_does_nothing_in_the_remote_list_or_with_nothing_hidden() {
        let mut a = app();
        a.step(Command::from(Action::ShowHidden));
        assert!(!a.showing_hidden());
        let mut b = app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        b.update(Action::Enter, &mut load);
        b.step(Command::from(Action::ShowHidden));
        assert!(!b.showing_hidden());
    }

    #[test]
    fn going_back_forgets_the_hidden_refs_and_the_toggle() {
        let (mut a, _) = with_hidden();
        a.step(Command::from(Action::ShowHidden));
        a.step(Command::from(Action::Back));
        let load = load_request(a.step(Command::from(Action::Enter)));
        a.finish_load(load.id, Ok(refs_of(&["refs/heads/main"])));
        assert_eq!(a.hidden(), 0);
        assert!(!a.showing_hidden());
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
                hidden: vec![hidden_ref(); 3],
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
                hidden: vec![hidden_ref(); 5],
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

    fn typed(a: &mut App, text: &str) {
        for c in text.chars() {
            a.step(Command::from(Action::Char(c)));
        }
    }

    fn entry_for(url: &str) -> crate::remotes::entry::Entry {
        crate::remotes::entry::Entry::new(url, "manual")
    }

    #[test]
    fn n_opens_a_prompt_that_takes_text_and_esc_closes_it() {
        let mut a = collected_app();
        a.step(Command::from(Action::New));
        assert_eq!(a.mode(), Mode::Prompt);
        assert_eq!(a.prompt(), Some(("add url", "")));
        typed(&mut a, "ab");
        a.step(Command::from(Action::Backspace));
        assert_eq!(a.prompt(), Some(("add url", "a")));
        assert_eq!(a.step(Command::from(Action::Back)), Flow::Continue);
        assert_eq!(a.prompt(), None);
        assert_eq!(a.mode(), Mode::Normal);
    }

    #[test]
    fn enter_on_a_new_valid_url_asks_the_loop_to_add_it() {
        let mut a = collected_app();
        a.step(Command::from(Action::New));
        typed(&mut a, "  https://u:secret@h/team/proj.git ");
        let Flow::Add(url) = a.step(Command::from(Action::Enter)) else {
            panic!("expected an add");
        };
        // Trimmed, and the password is not kept.
        assert_eq!(url, "https://u@h/team/proj.git");
        assert_eq!(a.prompt(), None);
    }

    #[test]
    fn an_invalid_url_shows_why_and_keeps_the_prompt_open() {
        let mut a = collected_app();
        a.step(Command::from(Action::New));
        typed(&mut a, "ext::sh -c id");
        assert_eq!(a.step(Command::from(Action::Enter)), Flow::Continue);
        assert!(a.error().is_some());
        assert_eq!(a.prompt(), Some(("add url", "ext::sh -c id")));
        // An empty prompt is refused too.
        let mut b = collected_app();
        b.step(Command::from(Action::New));
        assert_eq!(b.step(Command::from(Action::Enter)), Flow::Continue);
        assert!(b.error().is_some());
        assert!(b.prompt().is_some());
    }

    #[test]
    fn a_url_already_listed_moves_the_cursor_instead_of_adding() {
        let mut a = collected_app();
        a.step(Command::from(Action::New));
        // Same repo as netdev/net, spelled differently.
        typed(&mut a, "git@h:net.git");
        assert_eq!(a.step(Command::from(Action::Enter)), Flow::Continue);
        assert_eq!(a.prompt(), None);
        let under = a.list().visible_cells()[a.list().cursor()].1[0].clone();
        assert_eq!(under, "netdev/net");
        assert!(a.notice().is_some_and(|n| n.contains("already")));
        assert_eq!(a.list().total(), 3);
    }

    #[test]
    fn a_finished_add_inserts_the_row_in_order_and_moves_the_cursor_to_it() {
        let mut a = collected_app();
        a.finish_add(Ok(entry_for("https://h/team/proj")));
        // Real remotes first, then collected ones by label.
        let names: Vec<String> = a
            .list()
            .visible_cells()
            .iter()
            .map(|(_, c)| c[0].clone())
            .collect();
        assert_eq!(names, ["origin", "netdev/net", "team/proj", "usb/usb"]);
        let under = a.list().visible_cells()[a.list().cursor()].1[0].clone();
        assert_eq!(under, "team/proj");
        assert_eq!(a.level(), Level::Remotes);
        assert!(a.notice().is_some_and(|n| n.contains("added team/proj")));
    }

    #[test]
    fn a_failed_add_shows_the_error_and_adds_nothing() {
        let mut a = collected_app();
        a.finish_add(Err(Error::NoRefs("x".into())));
        assert!(a.error().is_some());
        assert_eq!(a.list().total(), 3);
    }

    #[test]
    fn n_does_nothing_in_the_refs_list() {
        let mut a = collected_app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        a.update(Action::Enter, &mut load);
        a.step(Command::from(Action::New));
        assert_eq!(a.prompt(), None);
    }

    fn promoting(a: &mut App) {
        on_row(a, 1);
        a.step(Command::from(Action::Promote));
    }

    #[test]
    fn p_on_a_collected_row_opens_a_name_prompt_filled_from_the_label() {
        let mut a = collected_app();
        promoting(&mut a);
        assert_eq!(a.prompt(), Some(("remote name", "netdev-net")));
        assert_eq!(a.mode(), Mode::Prompt);
    }

    #[test]
    fn p_on_a_real_remote_or_in_the_refs_list_does_nothing_but_note() {
        let mut a = collected_app();
        a.step(Command::from(Action::Promote));
        assert_eq!(a.prompt(), None);
        assert!(
            a.notice()
                .is_some_and(|n| n.contains("already a real remote"))
        );

        let mut b = collected_app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        b.update(Action::Enter, &mut load);
        b.step(Command::from(Action::Promote));
        assert_eq!(b.prompt(), None);
    }

    #[test]
    fn enter_asks_the_loop_to_run_git_remote_add_and_keeps_the_prompt_open() {
        let mut a = collected_app();
        promoting(&mut a);
        let flow = a.step(Command::from(Action::Enter));
        assert_eq!(
            flow,
            Flow::Promote {
                name: "netdev-net".into(),
                url: "https://h/net".into()
            }
        );
        assert!(a.prompt().is_some());
    }

    #[test]
    fn bad_names_are_refused_before_git_sees_them() {
        for bad in ["", "-x", "a b", "origin"] {
            let mut a = collected_app();
            promoting(&mut a);
            for _ in 0.."netdev-net".len() {
                a.step(Command::from(Action::Backspace));
            }
            typed(&mut a, bad);
            assert_eq!(
                a.step(Command::from(Action::Enter)),
                Flow::Continue,
                "{bad:?}"
            );
            assert!(a.error().is_some(), "{bad:?}");
            assert!(a.prompt().is_some(), "{bad:?}");
        }
    }

    #[test]
    fn a_finished_promotion_turns_the_row_into_a_real_remote() {
        let mut a = collected_app();
        promoting(&mut a);
        a.finish_promote("https://h/net", "netdev-net", Ok(()));
        assert_eq!(a.prompt(), None);
        let names: Vec<String> = a
            .list()
            .visible_cells()
            .iter()
            .map(|(_, c)| c[0].clone())
            .collect();
        assert_eq!(names, ["origin", "netdev-net", "usb/usb"]);
        let under = a.list().visible_cells()[a.list().cursor()].1[0].clone();
        assert_eq!(under, "netdev-net");
        // Real now, so d refuses it and p says so.
        a.step(Command::from(Action::Delete));
        assert_eq!(a.confirming(), None);
        assert!(a.notice().is_some_and(|n| n.contains("real remote")));
    }

    #[test]
    fn a_failed_command_is_shown_as_the_last_line_git_printed() {
        let status = std::process::Command::new("false").status().unwrap();
        let e = Error::Failed {
            command: "git remote add a..b u".into(),
            status,
            stderr: "fatal: 'a..b' is not a valid remote name\n".into(),
        };
        assert_eq!(short_reason(&e), "'a..b' is not a valid remote name");
    }

    #[test]
    fn a_failed_promotion_shows_git_s_reason_and_keeps_the_prompt() {
        let mut a = collected_app();
        promoting(&mut a);
        a.finish_promote(
            "https://h/net",
            "netdev-net",
            Err(Error::NoRefs("x".into())),
        );
        assert!(a.error().is_some());
        assert_eq!(a.prompt(), Some(("remote name", "netdev-net")));
        assert_eq!(a.list().total(), 3);
    }

    fn open_prompt_with(a: &mut App, url: &str) -> Flow {
        a.step(Command::from(Action::Open));
        typed(a, url);
        a.step(Command::from(Action::Enter))
    }

    #[test]
    fn o_opens_a_prompt_and_only_in_the_remote_list() {
        let mut a = collected_app();
        a.step(Command::from(Action::Open));
        assert_eq!(a.prompt(), Some(("open url", "")));
        let mut b = collected_app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        b.update(Action::Enter, &mut load);
        b.step(Command::from(Action::Open));
        assert_eq!(b.prompt(), None);
    }

    #[test]
    fn a_new_url_is_loaded_without_being_added_to_the_list() {
        let mut a = collected_app();
        let flow = open_prompt_with(&mut a, "https://u:secret@h/team/proj.git");
        let load = load_request(flow);
        assert_eq!(load.remote.url, "https://u@h/team/proj.git");
        assert_eq!(load.remote.name, "https://u@h/team/proj.git");
        assert_eq!(load.remote.tags, Vec::<String>::new());
        assert_eq!(a.prompt(), None);
        assert_eq!(a.list().total(), 3);
        a.finish_load(load.id, Ok(refs_of(&["refs/heads/main"])));
        assert_eq!(a.level(), Level::Refs);
        assert_eq!(a.chosen_remote(), Some("https://u@h/team/proj.git"));
        let Flow::Done(selection) = a.step(Command::from(Action::Enter)) else {
            panic!("expected a selection");
        };
        assert_eq!(selection.url, "https://u@h/team/proj.git");
        assert_eq!(selection.refname, "refs/heads/main");
    }

    #[test]
    fn going_back_forgets_the_one_off_remote() {
        let mut a = collected_app();
        let load = load_request(open_prompt_with(&mut a, "https://h/team/proj"));
        a.finish_load(load.id, Ok(refs_of(&["refs/heads/main"])));
        a.step(Command::from(Action::Back));
        assert_eq!(a.level(), Level::Remotes);
        assert_eq!(a.remotes.len(), 3);
        assert_eq!(a.list().total(), 3);
    }

    #[test]
    fn cancelling_or_failing_the_load_forgets_it_too() {
        let mut a = collected_app();
        load_request(open_prompt_with(&mut a, "https://h/team/proj"));
        assert_eq!(a.step(Command::from(Action::Back)), Flow::CancelLoad);
        assert_eq!(a.remotes.len(), 3);

        let load = load_request(open_prompt_with(&mut a, "https://h/team/proj"));
        a.finish_load(load.id, Err(Error::NoRefs("x".into())));
        assert_eq!(a.remotes.len(), 3);
        assert!(a.error().is_some());
    }

    #[test]
    fn a_bad_url_says_why_and_keeps_the_prompt() {
        let mut a = collected_app();
        assert_eq!(open_prompt_with(&mut a, "ext::sh"), Flow::Continue);
        assert!(a.error().is_some());
        assert!(a.prompt().is_some());
    }

    #[test]
    fn a_url_already_listed_loads_that_remote_instead() {
        let mut a = collected_app();
        let load = load_request(open_prompt_with(&mut a, "git@h:net.git"));
        assert_eq!(load.remote.name, "netdev/net");
        assert_eq!(a.remotes.len(), 3);
    }

    fn scan_menu(a: &mut App, item: char) -> Flow {
        a.step(Command::from(Action::Scan));
        a.step(Command::from(Action::Char(item)))
    }

    #[test]
    fn capital_a_opens_a_menu_and_anything_else_closes_it() {
        let mut a = collected_app();
        a.step(Command::from(Action::Scan));
        assert_eq!(a.mode(), Mode::Menu);
        assert!(a.menu().is_some());
        assert_eq!(a.step(Command::from(Action::Back)), Flow::Continue);
        assert_eq!(a.mode(), Mode::Normal);
        let mut b = collected_app();
        b.step(Command::from(Action::Scan));
        assert_eq!(b.step(Command::from(Action::Char('z'))), Flow::Continue);
        assert_eq!(b.mode(), Mode::Normal);
    }

    #[test]
    fn the_menu_only_opens_in_the_remote_list() {
        let mut a = collected_app();
        let mut load = |_: &Remote| Ok(refs_of(&["refs/heads/main"]));
        a.update(Action::Enter, &mut load);
        a.step(Command::from(Action::Scan));
        assert_eq!(a.mode(), Mode::Normal);
    }

    #[test]
    fn merges_asks_for_a_range_that_defaults_to_head_with_500() {
        let mut a = collected_app();
        assert_eq!(scan_menu(&mut a, 'm'), Flow::Continue);
        assert_eq!(a.prompt(), Some(("merges range", "HEAD -n 500")));
        let Flow::Scan(AddSource::Merges { range, max }) = a.step(Command::from(Action::Enter))
        else {
            panic!("expected a merges scan");
        };
        assert_eq!((range.as_deref(), max), (Some("HEAD"), Some(500)));
    }

    #[test]
    fn the_merges_range_can_be_edited_and_the_limit_dropped() {
        let mut a = collected_app();
        scan_menu(&mut a, 'm');
        for _ in 0.."HEAD -n 500".len() {
            a.step(Command::from(Action::Backspace));
        }
        typed(&mut a, "v6.11..v6.12");
        let Flow::Scan(AddSource::Merges { range, max }) = a.step(Command::from(Action::Enter))
        else {
            panic!("expected a merges scan");
        };
        assert_eq!((range.as_deref(), max), (Some("v6.11..v6.12"), None));

        let mut b = collected_app();
        scan_menu(&mut b, 'm');
        for _ in 0.."HEAD -n 500".len() {
            b.step(Command::from(Action::Backspace));
        }
        typed(&mut b, "-n 20 v6.1..v6.2");
        let Flow::Scan(AddSource::Merges { range, max }) = b.step(Command::from(Action::Enter))
        else {
            panic!("expected a merges scan");
        };
        assert_eq!((range.as_deref(), max), (Some("v6.1..v6.2"), Some(20)));
    }

    #[test]
    fn a_bad_merges_range_says_why_and_keeps_the_prompt() {
        for bad in ["-n", "-n x", "-n 5 a b", "--output=x", "-x"] {
            let mut a = collected_app();
            scan_menu(&mut a, 'm');
            for _ in 0.."HEAD -n 500".len() {
                a.step(Command::from(Action::Backspace));
            }
            typed(&mut a, bad);
            assert_eq!(
                a.step(Command::from(Action::Enter)),
                Flow::Continue,
                "{bad:?}"
            );
            assert!(a.error().is_some(), "{bad:?}");
            assert!(a.prompt().is_some(), "{bad:?}");
        }
    }

    #[test]
    fn t_scans_maintainers_at_once_and_f_asks_for_a_file() {
        let mut a = collected_app();
        assert_eq!(
            scan_menu(&mut a, 't'),
            Flow::Scan(AddSource::Maintainers { path: None })
        );
        let mut b = collected_app();
        scan_menu(&mut b, 'f');
        assert_eq!(b.prompt(), Some(("url file", "")));
        typed(&mut b, "urls.txt");
        let Flow::Scan(AddSource::Url { from_file, urls }) = b.step(Command::from(Action::Enter))
        else {
            panic!("expected a url scan");
        };
        assert_eq!(from_file.as_deref(), Some("urls.txt"));
        assert_eq!(urls, Vec::<String>::new());
        // Stdin is the terminal here, so `-` makes no sense.
        let mut c = collected_app();
        scan_menu(&mut c, 'f');
        typed(&mut c, "-");
        assert_eq!(c.step(Command::from(Action::Enter)), Flow::Continue);
        assert!(c.error().is_some());
    }

    #[test]
    fn a_finished_scan_adds_the_rows_and_says_how_many() {
        let mut a = collected_app();
        a.finish_scan(Ok(vec![
            entry_for("https://h/aa/first"),
            entry_for("https://h/net.git"),
            entry_for("https://h/zz/last"),
        ]));
        let names: Vec<String> = a
            .list()
            .visible_cells()
            .iter()
            .map(|(_, c)| c[0].clone())
            .collect();
        // The repo that was already listed is not added again.
        assert_eq!(
            names,
            ["origin", "aa/first", "netdev/net", "usb/usb", "zz/last"]
        );
        assert!(a.notice().is_some_and(|n| n.contains("added 2")));
    }

    #[test]
    fn a_failed_scan_shows_the_error_and_a_scan_in_progress_is_named() {
        let mut a = collected_app();
        a.begin_scan("merges");
        assert_eq!(a.scanning(), Some("merges"));
        a.finish_scan(Err(Error::NoRefs("x".into())));
        assert_eq!(a.scanning(), None);
        assert!(a.error().is_some());
        assert_eq!(a.list().total(), 3);
    }
}

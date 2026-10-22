// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The search box: what a typed query matches.
//!
//! Simple mode follows fzf's extended search. Terms are separated by spaces
//! and all must match (AND). A term starting with `!` must not match. A lone
//! `|` between terms is OR, and AND binds tighter than OR, so
//! `v6.1 !rc | v6.2` means (`v6.1` and not `rc`) or `v6.2`. There are no
//! parentheses. A backslash makes `!` or `|` literal: `\!x` searches for `!x`.
//! Matching ignores case.
//!
//! Regex mode treats the whole query as one regular expression, also ignoring
//! case. The engine matches in linear time, so no pattern can hang the picker.
//!
//! This module does no I/O, so it can be fuzzed.

use regex::{Regex, RegexBuilder};

/// How a query is read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    /// Terms, `!` and `|`, as above.
    #[default]
    Simple,
    /// One regular expression.
    Regex,
}

impl Mode {
    /// The other mode.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Simple => Self::Regex,
            Self::Regex => Self::Simple,
        }
    }

    /// The label the search box shows.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Simple => "search",
            Self::Regex => "regex",
        }
    }
}

/// One word of a simple query.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Term {
    /// Lowercased text to look for.
    text: String,
    /// The text must not be there.
    negated: bool,
}

#[derive(Debug, Clone)]
enum Kind {
    /// Every row matches.
    All,
    /// OR of groups, each an AND of terms.
    Simple(Vec<Vec<Term>>),
    /// A regular expression.
    Regex(Regex),
}

/// A compiled query.
#[derive(Debug, Clone)]
pub struct Matcher {
    kind: Kind,
}

impl Default for Matcher {
    fn default() -> Self {
        Self { kind: Kind::All }
    }
}

/// Turn a simple query into groups of terms.
fn parse_simple(query: &str) -> Vec<Vec<Term>> {
    let mut groups: Vec<Vec<Term>> = vec![Vec::new()];
    for word in query.split_whitespace() {
        if word == "|" {
            groups.push(Vec::new());
            continue;
        }
        // A backslash in front of `!` or `|` makes it an ordinary character.
        let (negated, text) = if word.starts_with("\\!") || word.starts_with("\\|") {
            (false, &word[1..])
        } else if let Some(rest) = word.strip_prefix('!') {
            (true, rest)
        } else {
            (false, word)
        };
        if text.is_empty() {
            // A lone `!` means nothing.
            continue;
        }
        if let Some(group) = groups.last_mut() {
            group.push(Term {
                text: text.to_lowercase(),
                negated,
            });
        }
    }
    groups.retain(|group| !group.is_empty());
    groups
}

impl Matcher {
    /// Read `query` in simple mode. An empty query matches everything.
    #[must_use]
    pub fn simple(query: &str) -> Self {
        let groups = parse_simple(query);
        if groups.is_empty() {
            Self::default()
        } else {
            Self {
                kind: Kind::Simple(groups),
            }
        }
    }

    /// Read `query` as a regular expression. An empty query matches
    /// everything.
    ///
    /// # Errors
    /// The pattern's own message, when it does not compile.
    pub fn regex(query: &str) -> Result<Self, String> {
        if query.is_empty() {
            return Ok(Self::default());
        }
        RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
            .map(|re| Self {
                kind: Kind::Regex(re),
            })
            .map_err(|e| first_line(&e.to_string()))
    }

    /// Read `query` in `mode`.
    ///
    /// # Errors
    /// In regex mode, the pattern's own message.
    pub fn new(mode: Mode, query: &str) -> Result<Self, String> {
        match mode {
            Mode::Simple => Ok(Self::simple(query)),
            Mode::Regex => Self::regex(query),
        }
    }

    /// Whether a row matches. `label` is the row's text and `cells` its
    /// columns. A regex is tried on each column, so `^` and `$` mean the start
    /// and end of a name, not of the whole line. Simple terms see the label.
    #[must_use]
    pub fn is_row_match(&self, label: &str, cells: &[String]) -> bool {
        match &self.kind {
            Kind::Regex(re) if !cells.is_empty() => cells.iter().any(|cell| re.is_match(cell)),
            _ => self.is_match(label),
        }
    }

    /// Whether `label` matches.
    #[must_use]
    pub fn is_match(&self, label: &str) -> bool {
        match &self.kind {
            Kind::All => true,
            Kind::Regex(re) => re.is_match(label),
            Kind::Simple(groups) => {
                let label = label.to_lowercase();
                groups.iter().any(|group| {
                    group
                        .iter()
                        .all(|term| label.contains(&term.text) != term.negated)
                })
            }
        }
    }
}

/// The part of a regex error worth showing in one line.
fn first_line(message: &str) -> String {
    message
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("error: "))
        .or_else(|| message.lines().next())
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple(query: &str, label: &str) -> bool {
        Matcher::simple(query).is_match(label)
    }

    #[test]
    fn an_empty_query_matches_everything() {
        for query in ["", "   ", "!", " ! ", "|", " | | "] {
            assert!(simple(query, "anything"), "{query:?}");
        }
    }

    #[test]
    fn terms_are_anded_and_ignore_case() {
        assert!(simple("net bpf", "netdev/BPF-next"));
        assert!(!simple("net bpf", "netdev/net"));
        assert!(simple("NET", "netdev"));
    }

    #[test]
    fn a_bang_excludes() {
        assert!(simple("v6.1 !rc", "refs/tags/v6.1"));
        assert!(!simple("v6.1 !rc", "refs/tags/v6.1-rc3"));
        assert!(!simple("!rc", "v6.1-rc3"));
        assert!(simple("!rc", "v6.1"));
    }

    #[test]
    fn a_lone_pipe_is_or_and_and_binds_tighter() {
        assert!(simple("net | bpf", "bpf-next"));
        assert!(simple("net | bpf", "netdev"));
        assert!(!simple("net | bpf", "mm"));
        // (v6.1 and not rc) or v6.2
        assert!(simple("v6.1 !rc | v6.2", "v6.1"));
        assert!(!simple("v6.1 !rc | v6.2", "v6.1-rc1"));
        assert!(simple("v6.1 !rc | v6.2", "v6.2-rc1"));
    }

    #[test]
    fn empty_groups_around_a_pipe_are_ignored() {
        assert!(simple("net |", "netdev"));
        assert!(!simple("net |", "mm"));
        assert!(simple("| net", "netdev"));
        assert!(simple("net | | bpf", "bpf"));
    }

    #[test]
    fn a_backslash_makes_bang_and_pipe_literal() {
        assert!(simple("\\!x", "a !x b"));
        assert!(!simple("\\!x", "a x b"));
        assert!(simple("\\|", "a|b"));
        assert!(!simple("\\|", "ab"));
        // Only in front of those two: other backslashes stay as typed.
        assert!(simple("a\\b", "a\\b"));
    }

    #[test]
    fn a_pipe_inside_a_word_is_literal() {
        assert!(simple("a|b", "xa|by"));
        assert!(!simple("a|b", "a"));
    }

    #[test]
    fn regex_mode_matches_the_whole_pattern_ignoring_case() {
        let m = Matcher::regex("^refs/tags/v6\\.(1|2)$").unwrap();
        assert!(m.is_match("refs/tags/v6.1"));
        assert!(m.is_match("REFS/TAGS/V6.2"));
        assert!(!m.is_match("refs/tags/v6.10"));
        assert!(Matcher::regex("").unwrap().is_match("x"));
    }

    #[test]
    fn a_regex_anchors_to_each_column_of_a_row() {
        let cells = ["refs/tags/v1.0".to_owned(), "45eb8ae".to_owned()];
        let label = "refs/tags/v1.0  45eb8ae";
        let ends = Matcher::regex("v1\\.0$").unwrap();
        assert!(ends.is_row_match(label, &cells));
        assert!(!ends.is_match(label));
        assert!(
            Matcher::regex("^refs/")
                .unwrap()
                .is_row_match(label, &cells)
        );
        assert!(Matcher::regex("^45eb").unwrap().is_row_match(label, &cells));
        assert!(!Matcher::regex("^tags").unwrap().is_row_match(label, &cells));
        // Simple terms still see the whole row.
        assert!(Matcher::simple("v1.0 45eb").is_row_match(label, &cells));
    }

    #[test]
    fn a_bad_regex_says_why() {
        for bad in ["(", "[a-", "a{2,1}", "*"] {
            let why = Matcher::regex(bad).unwrap_err();
            assert!(!why.is_empty(), "{bad:?}");
            assert!(!why.contains('\n'), "{bad:?}: {why:?}");
        }
    }

    #[test]
    fn a_hostile_pattern_cannot_hang() {
        // Catastrophic for a backtracking engine.
        let m = Matcher::regex("(a+)+$").unwrap();
        assert!(!m.is_match(&format!("{}b", "a".repeat(10_000))));
    }

    #[test]
    fn the_mode_toggles_and_names_itself() {
        assert_eq!(Mode::Simple.toggled(), Mode::Regex);
        assert_eq!(Mode::Regex.toggled(), Mode::Simple);
        assert_eq!(Mode::Simple.label(), "search");
        assert_eq!(Mode::Regex.label(), "regex");
    }
}

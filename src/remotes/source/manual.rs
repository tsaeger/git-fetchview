// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! URLs given by hand, in arguments or a file.

use super::{Scan, Source, collect};
use crate::error::Result;

/// One URL per line; blank lines and `#` comments are skipped.
#[must_use]
pub fn parse_url_list(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// URLs given directly.
#[derive(Debug, Clone, Default)]
pub struct Manual {
    /// The URLs, unvalidated.
    pub urls: Vec<String>,
}

impl Source for Manual {
    fn scan(&self) -> Result<Scan> {
        Ok(collect(self.urls.iter().map(String::as_str), "manual"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_and_comment_lines() {
        let text = "# list\n\n https://h/a \nssh://h/b\n#git://h/c\n";
        assert_eq!(parse_url_list(text), ["https://h/a", "ssh://h/b"]);
    }
}

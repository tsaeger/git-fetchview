// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

#![no_main]

use git_fetchview::search::{Matcher, Mode};
use libfuzzer_sys::fuzz_target;

// Any query, in either mode, against any label: it must not panic, and an
// empty query must match everything.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (query, label) = text.split_once('\n').unwrap_or((text, ""));
    for mode in [Mode::Simple, Mode::Regex] {
        if let Ok(matcher) = Matcher::new(mode, query) {
            let _ = matcher.is_match(label);
            if query.is_empty() {
                assert!(matcher.is_match(label));
            }
        }
    }
});

#![no_main]
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>


use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    if let Ok(url) = git_fetchview::remotes::entry::validate_url(text) {
        // Whatever is accepted must never look like an option or a helper.
        assert!(!url.starts_with('-'));
        let is_helper = url.split_once("::").is_some_and(|(helper, _)| {
            !helper.is_empty()
                && helper
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
        });
        assert!(!is_helper);
        let _ = git_fetchview::remotes::entry::key(&url);
        let _ = git_fetchview::remotes::entry::derive_label(&url);
    }
});

#![no_main]
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>


use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = git_fetchview::remotes::source::maintainers::parse_maintainers(text);
});

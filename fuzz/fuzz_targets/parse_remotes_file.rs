#![no_main]
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>


use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let parsed = git_fetchview::remotes::file::parse(text);
    // Everything that parses must survive a render and merge round trip.
    let merged = git_fetchview::remotes::file::merge("", &parsed.entries);
    let _ = git_fetchview::remotes::file::parse(&merged);
});

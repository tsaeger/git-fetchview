// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The merges source against a real repository. It changes the process
//! working directory, so it lives alone in its own test binary.

use std::process::Command;

use git_fetchview::remotes::source::Source;
use git_fetchview::remotes::source::merges::Merges;

#[test]
fn scans_merge_commits_from_a_real_repo() {
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.org")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.org")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["commit", "-q", "--allow-empty", "-m", "root"]);
    git(&["checkout", "-q", "-b", "side"]);
    git(&["commit", "-q", "--allow-empty", "-m", "side"]);
    git(&["checkout", "-q", "main"]);
    git(&["commit", "-q", "--allow-empty", "-m", "main"]);
    git(&[
        "merge",
        "-q",
        "--no-ff",
        "side",
        "-m",
        "Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net",
    ]);

    std::env::set_current_dir(dir.path()).unwrap();
    let scan = Merges {
        range: "HEAD".into(),
        max: None,
    }
    .scan()
    .unwrap();

    assert_eq!(scan.entries.len(), 1);
    assert_eq!(scan.rejected, 0);
    assert!(scan.entries[0].tags.contains("merge"));
}

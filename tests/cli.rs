// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! End-to-end tests: run the built binary in throwaway repos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const NET: &str = "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git";

struct Repo {
    dir: TempDir,
    home: TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["commit", "-q", "--allow-empty", "-m", "root"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn global_config(&self) -> PathBuf {
        self.home.path().join("gitconfig")
    }

    fn isolate(&self, cmd: &mut Command, cwd: &Path) {
        cmd.current_dir(cwd)
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_GLOBAL", self.global_config())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.org")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.org")
            .stdin(Stdio::null());
    }

    fn git_in(&self, cwd: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.args(args);
        self.isolate(&mut cmd, cwd);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(self.path(), args)
    }

    fn fv_in(&self, cwd: &Path, args: &[&str]) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_git-fetchview"));
        cmd.args(args);
        self.isolate(&mut cmd, cwd);
        cmd.output().unwrap()
    }

    fn fv(&self, args: &[&str]) -> Output {
        self.fv_in(self.path(), args)
    }

    fn remotes_file(&self) -> String {
        fs::read_to_string(self.path().join(".git/fetchview.remotes")).unwrap_or_default()
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn add_url_writes_the_default_file_and_registers_it_once() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(repo.remotes_file().contains("netdev/net") || repo.remotes_file().contains(NET));
    assert_eq!(
        repo.git(&["config", "--get-all", "fetchview.remotes"])
            .trim(),
        "fetchview.remotes"
    );

    let again = repo.fv(&["add", "--all", "url", NET]);
    assert!(again.status.success());
    assert_eq!(
        repo.git(&["config", "--get-all", "fetchview.remotes"])
            .lines()
            .count(),
        1
    );
    assert_eq!(repo.remotes_file().lines().count(), 1);
}

#[test]
fn list_into_a_closed_pipe_exits_quietly() {
    // `git fetchview list | head` closes the pipe early. That must not panic.
    let repo = Repo::new();
    repo.fv(&["add", "--all", "url", NET]);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_git-fetchview"));
    cmd.arg("list");
    repo.isolate(&mut cmd, repo.path());
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("panicked"), "{err}");
    assert!(out.status.success(), "{err}");
}

#[test]
fn list_shows_label_url_tags_and_file() {
    let repo = Repo::new();
    repo.fv(&["add", "--all", "url", NET]);
    let out = repo.fv(&["list"]);
    let text = stdout(&out);
    assert!(
        text.starts_with("netdev/net\thttps://git.kernel.org/"),
        "{text}"
    );
    assert!(text.contains("\tmanual\t"));
    assert!(text.contains("fetchview.remotes"));
}

#[test]
fn the_summary_uses_would_on_a_dry_run_and_added_otherwise() {
    let repo = Repo::new();
    let dry = stderr(&repo.fv(&["add", "--all", "--dry-run", "url", NET]));
    assert!(dry.contains("would add 1"), "{dry}");
    assert!(!dry.contains("added"), "{dry}");
    let real = stderr(&repo.fv(&["add", "--all", "url", NET]));
    assert!(real.contains("added 1"), "{real}");
    assert!(!real.contains("would"), "{real}");
}

#[test]
fn dry_run_changes_nothing() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "--dry-run", "url", NET]);
    assert!(out.status.success());
    assert!(stdout(&out).contains(NET));
    assert_eq!(repo.remotes_file(), "");
    // `git config --get-all` exits 1 and prints nothing when the key is unset.
    let get = Command::new("git")
        .args(["config", "--get-all", "fetchview.remotes"])
        .current_dir(repo.path())
        .env("HOME", repo.home.path())
        .env("GIT_CONFIG_GLOBAL", repo.global_config())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert_eq!(get.stdout, Vec::<u8>::new());
}

#[test]
fn urls_named_by_hand_need_neither_a_terminal_nor_all() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(repo.remotes_file().contains(NET));
}

#[test]
fn a_search_without_a_terminal_needs_all_but_only_when_something_is_new() {
    let repo = Repo::new();
    fs::write(
        repo.path().join("MAINTAINERS"),
        format!("NETWORKING\nT:\tgit {NET}\n"),
    )
    .unwrap();
    let out = repo.fv(&["add", "maintainers"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("terminal"), "{}", stderr(&out));
    assert_eq!(repo.remotes_file(), "");

    assert!(repo.fv(&["add", "--all", "maintainers"]).status.success());
    // Nothing new now, so there is nothing to choose and no terminal is needed.
    let again = repo.fv(&["add", "maintainers"]);
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(
        stderr(&again).contains("tags updated 1"),
        "{}",
        stderr(&again)
    );
}

#[test]
fn hostile_urls_are_rejected_and_counted() {
    let repo = Repo::new();
    let out = repo.fv(&[
        "add",
        "--all",
        "url",
        "--",
        "ext::sh -c id",
        "-oProxyCommand=x",
        "file:///etc",
        NET,
    ]);
    assert!(out.status.success());
    assert!(stderr(&out).contains("rejected 3"), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1);
    assert!(!file.contains("ext::"));
}

#[test]
fn add_merges_finds_pull_urls() {
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "side"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "side"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "main"]);
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "side",
        "-m",
        "Merge tag 'net-6.12' of git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net",
    ]);
    repo.git(&["checkout", "-q", "-b", "other", "main~1"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "other"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "other",
        "-m",
        "Merge remote-tracking branch 'a/b'",
    ]);

    let out = repo.fv(&["add", "--all", "merges"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1, "{file}");
    assert!(file.contains("git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net"));
    assert!(file.contains("\tmerge"));
    assert!(stderr(&out).contains("rejected 0"));
}

#[test]
fn add_maintainers_reads_t_lines() {
    let repo = Repo::new();
    fs::write(
        repo.path().join("MAINTAINERS"),
        "NET\nT:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\nT:\tquilt https://x/y\n",
    )
    .unwrap();
    let out = repo.fv(&["add", "--all", "maintainers"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(repo.remotes_file().contains("\tmaintainers"));
    assert_eq!(repo.remotes_file().lines().count(), 1);
}

#[test]
fn the_same_repo_from_two_sources_is_one_line_with_both_tags() {
    let repo = Repo::new();
    fs::write(
        repo.path().join("MAINTAINERS"),
        "T:\tgit git://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git\n",
    )
    .unwrap();
    repo.fv(&["add", "--all", "maintainers"]);
    repo.fv(&["add", "--all", "url", NET]);
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1, "{file}");
    assert!(file.contains("maintainers,manual"), "{file}");
}

#[test]
fn remove_by_label_edits_the_file() {
    let repo = Repo::new();
    repo.fv(&[
        "add",
        "--all",
        "url",
        NET,
        "https://git.kernel.org/pub/scm/linux/kernel/git/davem/net-next.git",
    ]);
    let out = repo.fv(&["remove", "netdev/net"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1);
    assert!(file.contains("net-next"));
    assert!(!repo.fv(&["remove", "nothing/here"]).status.success());
}

#[test]
fn a_relative_pointer_in_the_global_config_resolves_against_that_config() {
    let repo = Repo::new();
    let cfg_dir = repo.home.path().join("cfg");
    fs::create_dir_all(cfg_dir.join("lists")).unwrap();
    fs::write(
        cfg_dir.join("lists/k.remotes"),
        format!("{NET}\tmine\tshared\n"),
    )
    .unwrap();
    fs::write(
        repo.global_config(),
        format!(
            "[include]\n\tpath = {}\n",
            cfg_dir.join("gitconfig").display()
        ),
    )
    .unwrap();
    fs::write(
        cfg_dir.join("gitconfig"),
        "[fetchview]\n\tremotes = lists/k.remotes\n",
    )
    .unwrap();

    let out = repo.fv(&["list"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).starts_with("mine\t"), "{}", stdout(&out));
}

#[test]
fn a_linked_worktree_shares_the_default_file() {
    let repo = Repo::new();
    let wt = repo.home.path().join("wt");
    repo.git(&["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "wt"]);
    let out = repo.fv_in(&wt, &["add", "--all", "url", NET]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(repo.remotes_file().lines().count(), 1);
    assert!(!wt.join(".git").is_dir());
    let listed = repo.fv(&["list"]);
    assert!(stdout(&listed).contains("netdev/net"));
}

#[test]
fn file_and_global_register_an_explicit_file_in_the_global_config() {
    let repo = Repo::new();
    let file = repo.home.path().join("shared.remotes");
    let out = repo.fv(&[
        "add",
        "--all",
        "--file",
        file.to_str().unwrap(),
        "--global",
        "url",
        NET,
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        fs::read_to_string(&file).unwrap().contains("netdev/net")
            || fs::read_to_string(&file).unwrap().contains(NET)
    );
    let cfg = fs::read_to_string(repo.global_config()).unwrap();
    assert!(cfg.contains("shared.remotes"), "{cfg}");
    assert_eq!(repo.remotes_file(), "");
}

#[test]
fn global_without_file_is_refused() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "--all", "--global", "url", NET]);
    assert!(!out.status.success());
}

#[test]
fn commands_work_from_a_subdirectory() {
    let repo = Repo::new();
    let sub = repo.path().join("drivers/net");
    fs::create_dir_all(&sub).unwrap();
    assert!(repo.fv(&["add", "--all", "url", NET]).status.success());

    let listed = repo.fv_in(&sub, &["list"]);
    assert!(
        stdout(&listed).contains("netdev/net"),
        "{}",
        stderr(&listed)
    );
    assert!(!stderr(&listed).contains("warning"), "{}", stderr(&listed));

    let again = repo.fv_in(&sub, &["add", "--all", "url", NET]);
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(stderr(&again).contains("added 0"), "{}", stderr(&again));
    assert_eq!(
        repo.git(&["config", "--get-all", "fetchview.remotes"])
            .lines()
            .count(),
        1
    );

    let removed = repo.fv_in(&sub, &["remove", "netdev/net"]);
    assert!(removed.status.success(), "{}", stderr(&removed));
    assert_eq!(repo.remotes_file().lines().count(), 0);
}

#[test]
fn a_bad_pointer_is_a_warning_not_a_failure() {
    let repo = Repo::new();
    repo.git(&["config", "--add", "fetchview.remotes", "~nosuchuser/list"]);
    let out = repo.fv(&["list"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("warning"), "{}", stderr(&out));
}

#[test]
fn a_blank_viewer_flag_is_reported_as_a_flag_problem() {
    let repo = Repo::new();
    let out = repo.fv(&["--viewer", " "]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("--viewer"), "{err}");
    assert!(!err.contains("git config"), "{err}");
}

#[test]
fn a_bad_viewer_in_git_config_is_reported_as_a_config_problem() {
    let repo = Repo::new();
    repo.git(&["config", "fetchview.theme", "neon"]);
    let out = repo.fv(&[]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("fetchview.theme"), "{}", stderr(&out));
}

#[test]
fn the_same_kernel_tree_with_an_empty_port_is_listed_once() {
    let repo = Repo::new();
    let out = repo.fv(&[
        "add",
        "--all",
        "url",
        "git://git.kernel.org:/pub/scm/fs/xfs/xfs-linux",
        "git://git.kernel.org/pub/scm/fs/xfs/xfs-linux",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let file = repo.remotes_file();
    assert_eq!(file.lines().count(), 1, "{file}");
    assert!(file.starts_with("git://git.kernel.org/pub/scm/fs/xfs/xfs-linux"));
}

/// A bare repository for `origin`, holding `main`, a tag, a branch and a tag of
/// one name, and the refs a host makes for a pull request and a merge request.
fn origin(repo: &Repo) -> TempDir {
    let bare = tempfile::tempdir().unwrap();
    let path = bare.path().to_str().unwrap().to_owned();
    repo.git_in(bare.path(), &["init", "-q", "--bare", "-b", "main"]);
    repo.git(&["remote", "add", "origin", &path]);
    repo.git(&["branch", "dup"]);
    repo.git(&["tag", "v1.0"]);
    repo.git(&["tag", "dup"]);
    repo.git(&[
        "push",
        "-q",
        "origin",
        "refs/heads/main",
        "refs/heads/dup",
        "refs/tags/dup",
        "refs/tags/v1.0",
    ]);
    for name in ["refs/pull/7/head", "refs/merge-requests/9/head"] {
        repo.git_in(bare.path(), &["update-ref", name, "refs/heads/main"]);
    }
    repo.git(&["config", "--global", "fetchview.viewer", "echo viewing {}"]);
    repo.git(&["config", "--global", "--add", "fetchview.hideRefsFrom", ""]);
    bare
}

#[test]
fn a_remote_and_a_ref_fetch_and_view_without_the_picker() {
    let repo = Repo::new();
    let bare = origin(&repo);
    let out = repo.fv(&["origin", "main"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let trail = stderr(&out);
    assert!(
        trail.contains(&format!(
            "git fetch {} refs/heads/main",
            bare.path().display()
        )),
        "{trail}"
    );
    assert!(trail.contains("echo viewing FETCH_HEAD"), "{trail}");
    assert!(
        stdout(&out).contains("viewing FETCH_HEAD"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn short_names_and_pull_request_numbers_resolve_against_the_remote() {
    let repo = Repo::new();
    let _bare = origin(&repo);
    for (asked, wanted) in [
        ("v1.0", "refs/tags/v1.0"),
        ("refs/tags/v1.0", "refs/tags/v1.0"),
        ("7", "refs/pull/7/head"),
        ("#7", "refs/pull/7/head"),
        ("9", "refs/merge-requests/9/head"),
    ] {
        let out = repo.fv(&["origin", asked]);
        assert!(out.status.success(), "{asked}: {}", stderr(&out));
        assert!(stderr(&out).contains(wanted), "{asked}: {}", stderr(&out));
    }
}

#[test]
fn an_unknown_or_ambiguous_ref_is_an_error_that_says_so() {
    let repo = Repo::new();
    let _bare = origin(&repo);
    let none = repo.fv(&["origin", "nope"]);
    assert!(!none.status.success());
    assert!(
        stderr(&none).contains("no ref matching `nope`"),
        "{}",
        stderr(&none)
    );
    let dup = repo.fv(&["origin", "dup"]);
    assert!(!dup.status.success());
    let message = stderr(&dup);
    assert!(message.contains("more than one ref"), "{message}");
    assert!(
        message.contains("refs/heads/dup") && message.contains("refs/tags/dup"),
        "{message}"
    );
    assert!(!stdout(&dup).contains("viewing"));
}

#[test]
fn a_url_that_is_not_allowed_is_refused_before_anything_runs() {
    let repo = Repo::new();
    let _bare = origin(&repo);
    let out = repo.fv(&["ext::sh -c id", "main"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("<URL>"), "{}", stderr(&out));
    assert!(!stdout(&out).contains("viewing"));
}

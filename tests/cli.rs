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
fn without_a_terminal_add_needs_all() {
    let repo = Repo::new();
    let out = repo.fv(&["add", "url", NET]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("terminal"), "{}", stderr(&out));
    assert_eq!(repo.remotes_file(), "");
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

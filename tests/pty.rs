// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Drive the real binary in a pseudo-terminal and read what a person would
//! see. These catch what the `TestBackend` tests cannot: raw mode, the
//! alternate screen, the status line while a load runs, and the trail left on
//! screen after the picker exits.
//!
//! No network: the remote is a bare repository on disk, and the stalled one
//! is a local socket that never answers.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::num::NonZeroU16;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use tempfile::TempDir;

const ROWS: u16 = 24;
const COLS: u16 = 100;
const GRID: (NonZeroU16, NonZeroU16) = (
    NonZeroU16::new(ROWS).unwrap(),
    NonZeroU16::new(COLS).unwrap(),
);
const WAIT: Duration = Duration::from_secs(15);

/// A scratch repository with one real remote, `origin`, that is a bare repo
/// holding `main` and a tag.
struct World {
    work: TempDir,
    home: TempDir,
    bare: TempDir,
}

fn git(dir: &Path, home: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", home.join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.org")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.org")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

impl World {
    fn new() -> Self {
        let world = Self {
            work: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
            bare: tempfile::tempdir().unwrap(),
        };
        // Nothing here may reach the network or a keychain.
        std::fs::write(
            world.home.path().join("gitconfig"),
            "[fetchview]\n\thideRefsFrom =\n\tviewer = echo viewing {}\n[credential]\n\thelper =\n",
        )
        .unwrap();
        let (work, home, bare) = (world.work.path(), world.home.path(), world.bare.path());
        git(bare, home, &["init", "-q", "--bare", "-b", "main"]);
        git(work, home, &["init", "-q", "-b", "main"]);
        git(work, home, &["commit", "-q", "--allow-empty", "-m", "root"]);
        git(work, home, &["tag", "v1.0"]);
        git(
            work,
            home,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );
        git(work, home, &["push", "-q", "origin", "main", "v1.0"]);
        world
    }

    fn add_remote(&self, name: &str, url: &str) {
        git(
            self.work.path(),
            self.home.path(),
            &["remote", "add", name, url],
        );
    }

    /// Collect a URL the way `git fetchview add url` does.
    fn collect(&self, url: &str) {
        let out = Command::new(env!("CARGO_BIN_EXE_git-fetchview"))
            .args(["add", "--all", "url", url])
            .current_dir(self.work.path())
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_GLOBAL", self.home.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }

    fn git_output(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(self.work.path())
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_GLOBAL", self.home.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn collected_file(&self) -> String {
        std::fs::read_to_string(self.work.path().join(".git/fetchview.remotes")).unwrap_or_default()
    }

    fn start(&self) -> Option<Session> {
        Session::start(self)
    }
}

/// The program in a pty, with a terminal emulator behind it.
struct Session {
    screen: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Session {
    /// `None` where there is no pty to be had, so the test can skip.
    fn start(world: &World) -> Option<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .ok()?;
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_git-fetchview"));
        cmd.env_clear();
        cmd.cwd(world.work.path());
        for key in ["PATH", "TMPDIR"] {
            if let Some(value) = std::env::var_os(key) {
                cmd.env(key, value);
            }
        }
        cmd.env("HOME", world.home.path());
        cmd.env("GIT_CONFIG_GLOBAL", world.home.path().join("gitconfig"));
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd).ok()?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().ok()?;
        let writer = pair.master.take_writer().ok()?;
        let screen = Arc::new(Mutex::new(vt100::Parser::new(GRID.0, GRID.1, 0)));
        let feed = Arc::clone(&screen);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                feed.lock().unwrap().process(&buf[..n]);
            }
            // Keep the master alive until the child is gone.
            drop(pair.master);
        });
        Some(Self {
            screen,
            writer,
            child,
        })
    }

    fn text(&self) -> String {
        self.screen.lock().unwrap().screen().contents()
    }

    fn send(&mut self, keys: &str) {
        self.writer.write_all(keys.as_bytes()).unwrap();
        self.writer.flush().unwrap();
    }

    /// Wait until the screen satisfies `ok`, or fail showing the screen.
    fn wait_until(&self, what: &str, ok: impl Fn(&str) -> bool) {
        let start = Instant::now();
        loop {
            let text = self.text();
            if ok(&text) {
                return;
            }
            assert!(
                start.elapsed() < WAIT,
                "timed out waiting for {what}. Screen:\n{text}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_for(&self, needle: &str) {
        self.wait_until(&format!("{needle:?}"), |t| t.contains(needle));
    }

    fn wait_exit(&mut self) -> portable_pty::ExitStatus {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                start.elapsed() < WAIT,
                "the program did not exit. Screen:\n{}",
                self.text()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Skip a test where no pty can be opened (some sandboxes).
macro_rules! session {
    ($world:expr) => {
        match $world.start() {
            Some(session) => session,
            None => {
                eprintln!("no pty available, skipping");
                return;
            }
        }
    };
}

#[test]
fn pick_a_remote_then_quit() {
    let world = World::new();
    let mut s = session!(world);
    s.wait_for("remotes");
    s.wait_for("origin");
    s.send("\r");
    s.wait_for("refs of origin");
    s.wait_for("refs/tags/v1.0");
    s.send("q");
    assert!(s.wait_exit().success());
}

#[test]
fn picking_a_ref_prints_the_trail_and_runs_the_viewer() {
    let world = World::new();
    let mut s = session!(world);
    s.wait_for("origin");
    s.send("\r");
    s.wait_for("refs/heads/main");
    // Narrow to the one ref, leave the search box, and pick it.
    s.send("i");
    s.send("heads/main");
    s.send("\x1b");
    s.wait_until("one match", |t| t.contains("1 of "));
    s.send("\r");
    assert!(s.wait_exit().success());
    let screen = s.text();
    let bare = world.bare.path().to_str().unwrap();
    assert!(screen.contains("git ls-remote"), "{screen}");
    assert!(
        screen.contains(&format!("git fetch {bare} refs/heads/main")),
        "{screen}"
    );
    assert!(screen.contains("echo viewing FETCH_HEAD"), "{screen}");
    assert!(screen.contains("viewing FETCH_HEAD"), "{screen}");
}

#[test]
fn the_picker_leaves_the_alternate_screen_when_it_quits() {
    let world = World::new();
    let mut s = session!(world);
    s.wait_for("origin");
    s.send("q");
    assert!(s.wait_exit().success());
    let screen = s.text();
    // The picker's frame is gone, and the terminal is usable again.
    assert!(!screen.contains("remotes"), "{screen}");
}

#[test]
fn esc_gives_up_on_a_remote_that_never_answers() {
    // A socket that accepts and says nothing, so ls-remote waits forever.
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        eprintln!("no local sockets, skipping");
        return;
    };
    let port = listener.local_addr().unwrap().port();
    let held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            keep.lock().unwrap().push(stream);
        }
    });

    let world = World::new();
    world.add_remote("stall", &format!("git://127.0.0.1:{port}/x"));
    let mut s = session!(world);
    s.wait_for("stall");
    s.send("j");
    s.send("\r");
    s.wait_for("loading stall");
    s.wait_for("esc cancel");
    s.send("\x1b");
    s.wait_until("the remote list again", |t| {
        !t.contains("loading") && t.contains("j/k move")
    });
    // Still alive and responsive: Ctrl-C now quits.
    s.send("\x03");
    assert!(s.wait_exit().success());
}

#[test]
fn ctrl_c_cancels_a_load_and_then_quits() {
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        eprintln!("no local sockets, skipping");
        return;
    };
    let port = listener.local_addr().unwrap().port();
    let held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            keep.lock().unwrap().push(stream);
        }
    });

    let world = World::new();
    world.add_remote("stall", &format!("git://127.0.0.1:{port}/x"));
    let mut s = session!(world);
    s.wait_for("stall");
    s.send("j");
    s.send("\r");
    s.wait_for("loading stall");
    s.send("\x03");
    s.wait_until("the remote list again", |t| !t.contains("loading"));
    s.send("\x03");
    assert!(s.wait_exit().success());
}

#[test]
fn d_removes_a_collected_remote_after_y_and_refuses_a_real_one() {
    let world = World::new();
    world.collect("https://example.org/team/proj.git");
    assert!(world.collected_file().contains("team/proj"));
    let mut s = session!(world);
    s.wait_for("team/proj");

    // A real remote is git's to remove.
    s.send("d");
    s.wait_for("git remote remove origin");

    // On the collected row, the question comes first.
    s.send("j");
    s.send("d");
    s.wait_for("remove team/proj? y/n");
    s.send("n");
    s.wait_until("the question gone", |t| !t.contains("y/n"));
    assert!(world.collected_file().contains("team/proj"));

    s.send("d");
    s.wait_for("y/n");
    s.send("y");
    s.wait_for("removed team/proj");
    s.wait_until("the row gone", |t| !t.contains("example.org"));
    assert!(!world.collected_file().contains("team/proj"));
    s.send("q");
    assert!(s.wait_exit().success());
}

#[test]
fn n_adds_a_url_and_puts_the_cursor_on_the_new_row() {
    let world = World::new();
    let mut s = session!(world);
    s.wait_for("origin");

    // A bad URL says why and keeps the prompt.
    s.send("n");
    s.wait_for("add url:");
    s.send("ext::sh");
    s.send("\r");
    s.wait_for("transport helper");
    assert_eq!(world.collected_file(), "");

    // Start over with a good one.
    for _ in 0.."ext::sh".len() {
        s.send("\x7f");
    }
    s.send("https://example.org/team/proj.git");
    s.send("\r");
    s.wait_for("added team/proj");
    s.wait_until("the cursor on the new row", |t| {
        t.lines().any(|l| l.contains("> team/proj"))
    });
    assert!(world.collected_file().contains("example.org/team/proj"));

    // The same repo again, spelled as ssh, only moves the cursor.
    s.send("k");
    s.send("n");
    s.send("git@example.org:team/proj");
    s.send("\r");
    s.wait_for("already listed as team/proj");
    assert_eq!(world.collected_file().matches("team/proj").count(), 1);
    s.send("q");
    assert!(s.wait_exit().success());
}

#[test]
fn esc_closes_the_prompt_without_adding() {
    let world = World::new();
    let mut s = session!(world);
    s.wait_for("origin");
    s.send("n");
    s.send("https://example.org/a/b");
    s.wait_for("add url: https://example.org/a/b");
    s.send("\x1b");
    s.wait_until("the prompt gone", |t| !t.contains("add url:"));
    assert_eq!(world.collected_file(), "");
    s.send("q");
    assert!(s.wait_exit().success());
}

#[test]
fn p_makes_a_collected_remote_a_real_one() {
    let world = World::new();
    world.collect("https://example.org/team/proj.git");
    let mut s = session!(world);
    s.wait_for("team/proj");
    s.send("j");
    s.send("p");
    s.wait_for("remote name: team-proj");

    // Git refuses a bad name and the prompt stays, so it can be fixed.
    for _ in 0.."team-proj".len() {
        s.send("\x7f");
    }
    s.send("a..b");
    s.send("\r");
    s.wait_for("not a valid remote name");
    s.wait_for("remote name: a..b");
    assert!(!world.git_output(&["remote"]).contains("a..b"));

    for _ in 0.."a..b".len() {
        s.send("\x7f");
    }
    s.send("team");
    s.send("\r");
    s.wait_for("added remote team");
    s.wait_until("the cursor on the real remote", |t| {
        t.lines()
            .any(|l| l.contains("> team") && !l.contains("manual"))
    });
    assert_eq!(
        world.git_output(&["remote"]).lines().collect::<Vec<_>>(),
        ["origin", "team"]
    );
    assert_eq!(
        world.git_output(&["remote", "get-url", "team"]).trim(),
        "https://example.org/team/proj.git"
    );
    // The collected entry stays in its file.
    assert!(world.collected_file().contains("team/proj"));
    s.send("q");
    assert!(s.wait_exit().success());
}

// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! `cargo xtask install`: binaries, man pages and shell completions.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

/// Where everything goes.
#[derive(Debug)]
pub struct Layout {
    /// `cargo install --root`; `None` means cargo's own default (`~/.cargo/bin`).
    pub bin_root: Option<PathBuf>,
    /// Man pages.
    pub man1: PathBuf,
    /// Bash completions.
    pub bash: PathBuf,
    /// Zsh completions.
    pub zsh: PathBuf,
    /// Fish completions.
    pub fish: PathBuf,
}

/// What goes into an installed file.
#[derive(Debug, Clone, Copy)]
pub enum Content {
    /// The generated man page.
    Man,
    /// The same page under the `git-fv` name.
    FvMan,
    /// A generated completion script for this shell.
    Completion(&'static str),
}

/// A file to install.
#[derive(Debug)]
pub struct File {
    /// What to write.
    pub content: Content,
    /// Where to write it.
    pub dest: PathBuf,
}

impl Layout {
    /// With a prefix, a conventional `<prefix>/share/...` tree. Without one,
    /// per-user directories that man, bash, zsh and fish already search.
    pub fn new(
        prefix: Option<&Path>,
        home: &Path,
        xdg_data: Option<&Path>,
        xdg_config: Option<&Path>,
    ) -> Self {
        if let Some(prefix) = prefix {
            let share = prefix.join("share");
            return Self {
                bin_root: Some(prefix.to_owned()),
                man1: share.join("man/man1"),
                bash: share.join("bash-completion/completions"),
                zsh: share.join("zsh/site-functions"),
                fish: share.join("fish/vendor_completions.d"),
            };
        }
        let data = xdg_data.map_or_else(|| home.join(".local/share"), Path::to_owned);
        let config = xdg_config.map_or_else(|| home.join(".config"), Path::to_owned);
        Self {
            bin_root: None,
            man1: data.join("man/man1"),
            bash: data.join("bash-completion/completions"),
            zsh: data.join("zsh/site-functions"),
            fish: config.join("fish/completions"),
        }
    }

    /// Read the layout from the environment.
    fn detect(prefix: Option<&Path>) -> Result<Self> {
        let var = |name| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let home = var("HOME").context("HOME is not set; pass --prefix")?;
        Ok(Self::new(
            prefix,
            &home,
            var("XDG_DATA_HOME").as_deref(),
            var("XDG_CONFIG_HOME").as_deref(),
        ))
    }

    /// Every file that will be written, in order.
    pub fn files(&self) -> Vec<File> {
        let file = |content, dir: &Path, name: &str| File {
            content,
            dest: dir.join(name),
        };
        vec![
            file(Content::Man, &self.man1, "git-fetchview.1"),
            file(Content::FvMan, &self.man1, "git-fv.1"),
            file(Content::Completion("bash"), &self.bash, "git-fetchview"),
            file(Content::Completion("zsh"), &self.zsh, "_git-fetchview"),
            file(
                Content::Completion("fish"),
                &self.fish,
                "git-fetchview.fish",
            ),
        ]
    }
}

/// The man page again under the `git-fv` name, so `git fv --help` finds one.
pub fn git_fv_man(page: &str) -> String {
    page.replace("git\\-fetchview", "git\\-fv")
        .replace("git-fetchview", "git-fv")
}

fn contents(content: Content) -> Result<String> {
    let read = |path: &str| fs::read_to_string(path).with_context(|| format!("reading {path}"));
    Ok(match content {
        Content::Man => read("target/man/git-fetchview.1")?,
        Content::FvMan => git_fv_man(&read("target/man/git-fetchview.1")?),
        Content::Completion(shell) => read(&format!("target/completions/git-fetchview.{shell}"))?,
    })
}

/// Build and install the binaries, then the man pages and completions.
pub fn run(prefix: Option<&Path>, dry_run: bool) -> Result<()> {
    let layout = Layout::detect(prefix)?;
    let mut install = Command::new("cargo");
    install.args(["install", "--path", ".", "--locked", "--force"]);
    if let Some(root) = &layout.bin_root {
        install.arg("--root").arg(root);
    }
    if dry_run {
        eprintln!("would run: {install:?}");
        for file in layout.files() {
            eprintln!("would write: {}", file.dest.display());
        }
        return Ok(());
    }
    super::exec(install)?;
    super::dist()?;
    for file in layout.files() {
        if let Some(dir) = file.dest.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        fs::write(&file.dest, contents(file.content)?)
            .with_context(|| format!("writing {}", file.dest.display()))?;
        eprintln!("installed {}", file.dest.display());
    }
    match Command::new("man").args(["-w", "git-fetchview"]).output() {
        Ok(out) if out.status.success() => {
            eprintln!(
                "man finds it: {}",
                String::from_utf8_lossy(&out.stdout).trim()
            );
        }
        _ => eprintln!(
            "man does not find it yet; add {} to MANPATH",
            layout
                .man1
                .parent()
                .map_or_else(String::new, |p| p.display().to_string())
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_gets_a_conventional_tree() {
        let l = Layout::new(Some(Path::new("/opt/fv")), Path::new("/home/u"), None, None);
        assert_eq!(l.bin_root.as_deref(), Some(Path::new("/opt/fv")));
        assert_eq!(l.man1, Path::new("/opt/fv/share/man/man1"));
        assert_eq!(
            l.bash,
            Path::new("/opt/fv/share/bash-completion/completions")
        );
        assert_eq!(l.zsh, Path::new("/opt/fv/share/zsh/site-functions"));
        assert_eq!(l.fish, Path::new("/opt/fv/share/fish/vendor_completions.d"));
    }

    #[test]
    fn without_a_prefix_it_installs_for_the_user() {
        let l = Layout::new(None, Path::new("/home/u"), None, None);
        assert_eq!(l.bin_root, None);
        assert_eq!(l.man1, Path::new("/home/u/.local/share/man/man1"));
        assert_eq!(
            l.bash,
            Path::new("/home/u/.local/share/bash-completion/completions")
        );
        assert_eq!(l.zsh, Path::new("/home/u/.local/share/zsh/site-functions"));
        assert_eq!(l.fish, Path::new("/home/u/.config/fish/completions"));
    }

    #[test]
    fn xdg_directories_win_over_the_home_defaults() {
        let l = Layout::new(
            None,
            Path::new("/home/u"),
            Some(Path::new("/data")),
            Some(Path::new("/conf")),
        );
        assert_eq!(l.man1, Path::new("/data/man/man1"));
        assert_eq!(l.fish, Path::new("/conf/fish/completions"));
    }

    #[test]
    fn the_git_fv_man_page_is_the_same_page_under_its_own_name() {
        let page = ".TH git-fetchview 1\n.SH NAME\ngit\\-fetchview \\- pick\n.B git\\-fetchview\n";
        let fv = git_fv_man(page);
        assert!(fv.contains(".TH git-fv 1"));
        assert!(fv.contains("git\\-fv \\- pick"));
        assert!(!fv.contains("fetchview"));
    }

    #[test]
    fn it_lists_every_file_it_will_write() {
        let l = Layout::new(Some(Path::new("/p")), Path::new("/h"), None, None);
        let files: Vec<_> = l
            .files()
            .into_iter()
            .map(|f| f.dest.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            files,
            [
                "/p/share/man/man1/git-fetchview.1",
                "/p/share/man/man1/git-fv.1",
                "/p/share/bash-completion/completions/git-fetchview",
                "/p/share/zsh/site-functions/_git-fetchview",
                "/p/share/fish/vendor_completions.d/git-fetchview.fish",
            ]
        );
    }
}

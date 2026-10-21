// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! SPDX header checking and insertion: `cargo xtask headers [--fix]`.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const LICENSE: &str = "MIT";
const COPYRIGHT: &str = "2026 Tom Saeger <tom.saeger@gmail.com>";

/// How far into a file the header must appear.
const HEADER_WINDOW: usize = 4;

#[derive(Clone, Copy)]
enum Style {
    /// `// text`
    Slashes,
    /// `# text`
    Hash,
    /// `<!-- text -->`
    Html,
}

impl Style {
    fn of(path: &str) -> Option<Self> {
        // backlog.md task files start with YAML front matter.
        if path.starts_with("backlog/") {
            return None;
        }
        let name = Path::new(path).file_name()?.to_str()?;
        if name == "Cargo.lock" || name == "LICENSE" {
            return None;
        }
        match Path::new(name).extension().and_then(|e| e.to_str()) {
            Some("rs") => Some(Self::Slashes),
            Some("toml" | "nix") => Some(Self::Hash),
            Some("md") => Some(Self::Html),
            _ if name == ".gitignore" => Some(Self::Hash),
            _ => None,
        }
    }

    fn line(self, text: &str) -> String {
        match self {
            Self::Slashes => format!("// {text}"),
            Self::Hash => format!("# {text}"),
            Self::Html => format!("<!-- {text} -->"),
        }
    }

    fn header(self) -> String {
        format!(
            "{}\n{}\n\n",
            self.line(&format!("SPDX-License-Identifier: {LICENSE}")),
            self.line(&format!("SPDX-FileCopyrightText: {COPYRIGHT}")),
        )
    }
}

fn has_header(style: Style, text: &str) -> bool {
    let head: Vec<&str> = text.lines().take(HEADER_WINDOW).collect();
    let want = [
        style.line(&format!("SPDX-License-Identifier: {LICENSE}")),
        style.line(&format!("SPDX-FileCopyrightText: {COPYRIGHT}")),
    ];
    want.iter().all(|w| head.contains(&w.as_str()))
}

/// Insert the header, keeping a leading shebang first.
fn with_header(style: Style, text: &str) -> String {
    let header = style.header();
    if text.starts_with("#!") {
        let (shebang, rest) = text.split_once('\n').unwrap_or((text, ""));
        format!("{shebang}\n{header}{rest}")
    } else {
        format!("{header}{text}")
    }
}

/// Tracked and untracked-but-not-ignored files.
fn project_files() -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .context("failed to run git ls-files")?;
    if !out.status.success() {
        bail!(
            "git ls-files failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8(out.stdout)?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Check every file, or add missing headers with `fix`.
pub fn run(fix: bool) -> Result<()> {
    let mut missing = Vec::new();
    for path in project_files()? {
        let Some(style) = Style::of(&path) else {
            continue;
        };
        if !Path::new(&path).is_file() {
            continue; // listed by git but deleted in the working tree
        }
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        if has_header(style, &text) {
            continue;
        }
        if fix {
            std::fs::write(&path, with_header(style, &text))
                .with_context(|| format!("writing {path}"))?;
            eprintln!("added header: {path}");
        } else {
            missing.push(path);
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        bail!(
            "{} file(s) missing the SPDX header (run `cargo xtask headers --fix`):\n  {}",
            missing.len(),
            missing.join("\n  ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_by_file() {
        assert!(matches!(Style::of("src/lib.rs"), Some(Style::Slashes)));
        assert!(matches!(Style::of("a/Cargo.toml"), Some(Style::Hash)));
        assert!(matches!(Style::of(".gitignore"), Some(Style::Hash)));
        assert!(matches!(Style::of("README.md"), Some(Style::Html)));
        assert!(matches!(Style::of("flake.nix"), Some(Style::Hash)));
        assert!(matches!(
            Style::of("rust-toolchain.toml"),
            Some(Style::Hash)
        ));
        assert!(Style::of("flake.lock").is_none());
        assert!(Style::of("LICENSE").is_none());
        // backlog.md task files start with YAML front matter, so no header.
        assert!(Style::of("backlog/tasks/task-1 - Something.md").is_none());
        assert!(Style::of("backlog/docs/notes.md").is_none());
        assert!(Style::of("docs/backlog/notes.md").is_some());
        assert!(Style::of("Cargo.lock").is_none());
    }

    #[test]
    fn fix_then_check_round_trips_and_is_idempotent() {
        for style in [Style::Slashes, Style::Hash, Style::Html] {
            let fixed = with_header(style, "body\n");
            assert!(!has_header(style, "body\n"));
            assert!(has_header(style, &fixed));
            assert!(fixed.ends_with("body\n"));
        }
    }

    #[test]
    fn shebang_stays_first() {
        let fixed = with_header(Style::Hash, "#!/bin/sh\necho\n");
        assert!(fixed.starts_with("#!/bin/sh\n# SPDX-License-Identifier"));
        assert!(fixed.ends_with("\n\necho\n"));
        assert!(has_header(Style::Hash, &fixed));
    }

    #[test]
    fn wrong_license_is_not_a_header() {
        let text = "// SPDX-License-Identifier: GPL-3.0\n// SPDX-FileCopyrightText: x\n";
        assert!(!has_header(Style::Slashes, text));
    }
}

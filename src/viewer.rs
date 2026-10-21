// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! The program that browses `FETCH_HEAD`: a preset name or a command line.

/// What runs when nothing else is configured.
pub const DEFAULT: &str = "tig";

/// Named viewers. Each is a shell command line that ends up showing
/// `FETCH_HEAD`, so a preset and a custom command are the same thing.
pub const PRESETS: [(&str, &str); 5] = [
    ("tig", "tig FETCH_HEAD"),
    ("gitk", "gitk FETCH_HEAD"),
    ("git-log", "git log --graph --decorate --stat FETCH_HEAD"),
    (
        "fzf",
        "git log --oneline --color=always FETCH_HEAD | fzf --ansi --no-sort --preview 'git show --color=always {1}'",
    ),
    ("fugitive", "nvim -c 'Git log --graph --oneline FETCH_HEAD'"),
];

/// The preset names, for messages and help.
#[must_use]
pub fn preset_names() -> String {
    PRESETS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Turn a preset name or a command into the shell command line to run.
///
/// A preset gives its command. Anything else is a command where `{}` stands
/// for `FETCH_HEAD`; without a `{}`, `FETCH_HEAD` is appended. Returns `None`
/// for a blank spec.
#[must_use]
pub fn resolve(spec: &str) -> Option<String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }
    if let Some((_, line)) = PRESETS.iter().find(|(name, _)| *name == spec) {
        return Some((*line).to_owned());
    }
    Some(if spec.contains("{}") {
        spec.replace("{}", "FETCH_HEAD")
    } else {
        format!("{spec} FETCH_HEAD")
    })
}

/// Pick the viewer: the flag, else the configured value, else the default.
///
/// # Errors
/// The reason, when the chosen spec is blank.
pub fn choose(flag: Option<&str>, configured: Option<&str>) -> Result<String, String> {
    resolve(flag.or(configured).unwrap_or(DEFAULT))
        .ok_or_else(|| "the viewer command is empty".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_resolve_to_their_command() {
        assert_eq!(resolve("tig").as_deref(), Some("tig FETCH_HEAD"));
        assert_eq!(resolve("gitk").as_deref(), Some("gitk FETCH_HEAD"));
        assert_eq!(
            resolve("git-log").as_deref(),
            Some("git log --graph --decorate --stat FETCH_HEAD")
        );
        assert!(resolve("fzf").unwrap().contains("| fzf --ansi"));
        assert!(resolve("fugitive").unwrap().starts_with("nvim -c 'Git log"));
        assert!(PRESETS.iter().all(|(_, line)| line.contains("FETCH_HEAD")));
    }

    #[test]
    fn braces_stand_for_fetch_head_everywhere_they_appear() {
        assert_eq!(
            resolve("show {} && log {}").as_deref(),
            Some("show FETCH_HEAD && log FETCH_HEAD")
        );
    }

    #[test]
    fn fetch_head_is_appended_when_there_are_no_braces() {
        assert_eq!(
            resolve("gitk --all").as_deref(),
            Some("gitk --all FETCH_HEAD")
        );
    }

    #[test]
    fn whitespace_is_trimmed_and_blank_is_nothing() {
        assert_eq!(resolve("  tig  ").as_deref(), Some("tig FETCH_HEAD"));
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   "), None);
    }

    #[test]
    fn an_unknown_word_is_run_as_a_command() {
        assert_eq!(resolve("tgi").as_deref(), Some("tgi FETCH_HEAD"));
    }

    #[test]
    fn the_flag_beats_config_which_beats_the_default() {
        assert_eq!(
            choose(Some("gitk"), Some("fzf")).as_deref(),
            Ok("gitk FETCH_HEAD")
        );
        assert!(choose(None, Some("fzf")).unwrap().contains("fzf"));
        assert_eq!(choose(None, None).as_deref(), Ok("tig FETCH_HEAD"));
    }

    #[test]
    fn a_blank_flag_is_rejected_with_a_reason() {
        let reason = choose(Some("  "), None).unwrap_err();
        assert!(reason.contains("empty"), "{reason}");
    }

    #[test]
    fn the_names_list_every_preset() {
        let names = preset_names();
        for (name, _) in PRESETS {
            assert!(names.contains(name), "{names}");
        }
    }
}

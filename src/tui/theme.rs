// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

//! Colour themes.

use std::str::FromStr;

use clap::ValueEnum;
use ratatui::style::Color;

/// Selectable theme names (`fetchview.theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ThemeName {
    /// Kanagawa wave.
    #[default]
    Kanagawa,
    /// Gruvbox dark.
    Gruvbox,
    /// Nord.
    Nord,
    /// Catppuccin mocha.
    Catppuccin,
}

impl FromStr for ThemeName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as ValueEnum>::from_str(s.trim(), true).map_err(|_| {
            let names: Vec<_> = Self::value_variants()
                .iter()
                .filter_map(ValueEnum::to_possible_value)
                .map(|v| v.get_name().to_owned())
                .collect();
            format!("expected one of: {}", names.join(", "))
        })
    }
}

/// Resolved palette.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Background.
    pub bg: Color,
    /// Normal text.
    pub fg: Color,
    /// Titles and the search prompt.
    pub accent: Color,
    /// Secondary text.
    pub dim: Color,
    /// Background of the selected row.
    pub selection: Color,
    /// Error messages.
    pub error: Color,
}

impl From<ThemeName> for Theme {
    #[allow(clippy::unreadable_literal)] // hex colours read fine unseparated
    fn from(name: ThemeName) -> Self {
        let rgb = Color::from_u32;
        let (bg, fg, accent, dim, selection, error) = match name {
            ThemeName::Kanagawa => (0x1F1F28, 0xDCD7BA, 0x7E9CD8, 0x727169, 0x2D4F67, 0xE82424),
            ThemeName::Gruvbox => (0x282828, 0xEBDBB2, 0xFABD2F, 0x928374, 0x504945, 0xFB4934),
            ThemeName::Nord => (0x2E3440, 0xD8DEE9, 0x88C0D0, 0x4C566A, 0x434C5E, 0xBF616A),
            ThemeName::Catppuccin => (0x1E1E2E, 0xCDD6F4, 0x89B4FA, 0x6C7086, 0x45475A, 0xF38BA8),
        };
        Self {
            bg: rgb(bg),
            fg: rgb(fg),
            accent: rgb(accent),
            dim: rgb(dim),
            selection: rgb(selection),
            error: rgb(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitively() {
        assert_eq!("Kanagawa".parse(), Ok(ThemeName::Kanagawa));
        assert_eq!(" nord ".parse(), Ok(ThemeName::Nord));
    }

    #[test]
    fn rejects_unknown_with_choices() {
        let err = "neon".parse::<ThemeName>().unwrap_err();
        assert!(err.contains("kanagawa"));
    }
}

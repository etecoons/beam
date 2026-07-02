//! Super Metroid theme management.
//!
//! Provides a strongly-typed [`Theme`] enum that replaces the string
//! literals previously scattered across each companion app. The theme
//! name (used in CSS `data-theme` selectors and `localStorage`) is the
//! lowercased variant name.
//!
//! CSS-name mapping (e.g. `"light"` → `Brinstar`) lives in [`mapping`].

use serde::{Deserialize, Serialize};

pub mod icons;
pub mod mapping;

/// The five Super Metroid-themed visual styles plus a default fallback.
///
/// The order in [`Theme::ALL`] is the order shown in the theme picker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Theme {
    /// Dark purple (default). Matches `crateria` in some apps.
    #[default]
    Crateria,
    /// Green forest — light mode.
    Brinstar,
    /// Red/orange — sepia mode.
    Norfair,
    /// Cyan ghost — dark mode.
    WreckedShip,
    /// Blue waves — nord mode.
    Maridia,
    /// Red target — accent/highlight theme.
    Tourian,
}

impl Theme {
    /// All themes in picker order.
    pub const ALL: &'static [Self] = &[
        Self::Brinstar,
        Self::Norfair,
        Self::WreckedShip,
        Self::Maridia,
        Self::Tourian,
        Self::Crateria,
    ];

    /// Lowercase name used in CSS selectors and `localStorage`.
    /// Must match a `[data-theme="..."]` block in `themes.css`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Crateria => "crateria",
            Self::Brinstar => "brinstar",
            Self::Norfair => "norfair",
            Self::WreckedShip => "wrecked_ship",
            Self::Maridia => "maridia",
            Self::Tourian => "tourian",
        }
    }

    /// Parse from a CSS name. Unknown names return [`None`] so the caller
    /// can decide on a fallback (typically the app's last-saved theme).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "crateria" => Self::Crateria,
            "brinstar" => Self::Brinstar,
            "norfair" => Self::Norfair,
            "wrecked_ship" => Self::WreckedShip,
            "maridia" => Self::Maridia,
            "tourian" => Self::Tourian,
            _ => return None,
        })
    }

    /// SVG icon HTML for the theme toggle button.
    pub fn icon_html(self) -> yew::Html {
        icons::icon(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        let names: Vec<&str> = Theme::ALL.iter().map(|t| t.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "duplicate theme names");
    }

    #[test]
    fn name_roundtrip_for_all() {
        for theme in Theme::ALL {
            assert_eq!(
                Theme::from_name(theme.name()),
                Some(*theme),
                "roundtrip failed for {theme:?}"
            );
        }
    }

    #[test]
    fn unknown_name_returns_none() {
        assert_eq!(Theme::from_name("nope"), None);
        assert_eq!(Theme::from_name(""), None);
        assert_eq!(Theme::from_name("BRINSTAR"), None, "case-sensitive");
    }

    #[test]
    fn default_is_crateria() {
        assert_eq!(Theme::default(), Theme::Crateria);
    }
}

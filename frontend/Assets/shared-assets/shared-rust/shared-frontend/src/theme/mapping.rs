//! Maps between the user-facing "color scheme" selector names used in each
//! app (e.g. `"light"`, `"dracula"`) and the shared [`Theme`] enum.
//!
//! Each app historically had its own hardcoded mapping (and they diverged).
//! This module consolidates them so every app picks from a single source of
//! truth. Apps that need a different mapping can still wrap these helpers.

use super::Theme;

/// User-facing color scheme name (the key in the theme picker `<select>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Sepia,
    Dracula,
    Nord,
    Crateria,
}

impl Scheme {
    /// All schemes in picker order.
    pub const ALL: &'static [Self] = &[
        Self::Light,
        Self::Sepia,
        Self::Dracula,
        Self::Nord,
        Self::Crateria,
    ];

    /// Selector value used in `<option value="...">`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Sepia => "sepia",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Crateria => "crateria",
        }
    }

    /// Friendly label shown next to the option.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Sepia => "Sepia",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::Crateria => "Crateria",
        }
    }

    /// Parse a scheme id. Unknown values return `None`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "light" => Self::Light,
            "sepia" => Self::Sepia,
            "dracula" => Self::Dracula,
            "nord" => Self::Nord,
            "crateria" => Self::Crateria,
            _ => return None,
        })
    }

    /// Convert a user-selected scheme to the corresponding Super Metroid
    /// theme. This is the single source of truth for the mapping.
    #[must_use]
    pub const fn to_theme(self) -> Theme {
        match self {
            Self::Light => Theme::Brinstar,
            Self::Sepia => Theme::Norfair,
            Self::Dracula => Theme::WreckedShip,
            Self::Nord => Theme::Maridia,
            Self::Crateria => Theme::Crateria,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_ids_are_unique() {
        let ids: Vec<&str> = Scheme::ALL.iter().map(|s| s.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate scheme ids");
    }

    #[test]
    fn scheme_labels_are_unique() {
        let labels: Vec<&str> = Scheme::ALL.iter().map(|s| s.label()).collect();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(labels.len(), sorted.len(), "duplicate scheme labels");
    }

    #[test]
    fn scheme_id_roundtrip() {
        for scheme in Scheme::ALL {
            assert_eq!(Scheme::from_id(scheme.id()), Some(*scheme));
        }
    }

    #[test]
    fn unknown_scheme_returns_none() {
        assert_eq!(Scheme::from_id("nope"), None);
        assert_eq!(Scheme::from_id(""), None);
        assert_eq!(Scheme::from_id("LIGHT"), None);
    }

    #[test]
    fn scheme_to_theme_matches_documented_mapping() {
        assert_eq!(Scheme::Light.to_theme(), Theme::Brinstar);
        assert_eq!(Scheme::Sepia.to_theme(), Theme::Norfair);
        assert_eq!(Scheme::Dracula.to_theme(), Theme::WreckedShip);
        assert_eq!(Scheme::Nord.to_theme(), Theme::Maridia);
        assert_eq!(Scheme::Crateria.to_theme(), Theme::Crateria);
    }
}

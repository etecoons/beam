//! Internationalization support: the [`Language`] enum and code ↔ label helpers.
//!
//! Translations of UI strings live in [`super::strings`].

use serde::{Deserialize, Serialize};

pub mod strings;

/// Languages supported by the UberMetroid companion apps.
///
/// The order in [`Language::all`] is the order shown in the language picker.
/// Adding a language means: add a variant here, add entries to [`Language::label`],
/// [`Language::from_code`], and translations to [`super::strings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    Chinese,
    Spanish,
    German,
    Japanese,
    French,
    Portuguese,
    Russian,
}

impl Language {
    /// ISO 639-1 code used in `<html lang="...">` and the `<select>` value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "zh",
            Self::Spanish => "es",
            Self::German => "de",
            Self::Japanese => "ja",
            Self::French => "fr",
            Self::Portuguese => "pt",
            Self::Russian => "ru",
        }
    }

    /// Human-readable label shown in the language picker.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
            Self::Spanish => "Español",
            Self::German => "Deutsch",
            Self::Japanese => "日本語",
            Self::French => "Français",
            Self::Portuguese => "Português",
            Self::Russian => "Русский",
        }
    }

    /// Parse from a language code. Unknown codes fall back to English.
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "zh" => Self::Chinese,
            "es" => Self::Spanish,
            "de" => Self::German,
            "ja" => Self::Japanese,
            "fr" => Self::French,
            "pt" => Self::Portuguese,
            "ru" => Self::Russian,
            _ => Self::English,
        }
    }

    /// All supported languages in picker order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::English,
            Self::Chinese,
            Self::Spanish,
            Self::German,
            Self::Japanese,
            Self::French,
            Self::Portuguese,
            Self::Russian,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique() {
        let codes: Vec<&str> = Language::all().iter().map(|l| l.code()).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "duplicate language codes");
    }

    #[test]
    fn labels_are_unique() {
        let labels: Vec<&str> = Language::all().iter().map(|l| l.label()).collect();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(labels.len(), sorted.len(), "duplicate language labels");
    }

    #[test]
    fn from_code_roundtrip() {
        for lang in Language::all() {
            assert_eq!(Language::from_code(lang.code()), *lang);
        }
    }

    #[test]
    fn unknown_code_falls_back_to_english() {
        assert_eq!(Language::from_code("xx"), Language::English);
        assert_eq!(Language::from_code(""), Language::English);
        assert_eq!(
            Language::from_code("EN"),
            Language::English,
            "case-sensitive"
        );
    }

    #[test]
    fn all_codes_match_expected() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Chinese.code(), "zh");
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::German.code(), "de");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(Language::French.code(), "fr");
        assert_eq!(Language::Portuguese.code(), "pt");
        assert_eq!(Language::Russian.code(), "ru");
    }
}

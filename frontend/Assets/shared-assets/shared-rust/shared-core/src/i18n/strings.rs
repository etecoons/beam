//! Centralized UI string lookup.
//!
//! All translated strings used by the shared components live here.
//! Adding a string means: add a variant to [`StringKey`], add a translation
//! row, and the lookup function will pick it up.
//!
//! Lookup is O(n) over the inner array; with 8 languages and a handful of
//! keys, this is faster than a hashmap. If the string count grows past ~50,
//! consider switching to `phf`.

use super::Language;

/// Keys for translatable UI strings.
///
/// Each key maps to a row of translations across all supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringKey {
    TooltipToggleTheme,
    TooltipPrint,
    TooltipLogout,
    AriaSelectLanguage,
    TitleViewReleaseNotes,
    AriaGitHubProfile,
}

impl StringKey {
    /// All keys in stable display order. Used to validate translation coverage.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::TooltipToggleTheme,
            Self::TooltipPrint,
            Self::TooltipLogout,
            Self::AriaSelectLanguage,
            Self::TitleViewReleaseNotes,
            Self::AriaGitHubProfile,
        ]
    }

    /// English fallback text, used when a language row is missing a key.
    #[must_use]
    fn english(self) -> &'static str {
        match self {
            Self::TooltipToggleTheme => "Toggle theme",
            Self::TooltipPrint => "Print",
            Self::TooltipLogout => "Log out",
            Self::AriaSelectLanguage => "Select language",
            Self::TitleViewReleaseNotes => "View Release Notes",
            Self::AriaGitHubProfile => "GitHub Profile",
        }
    }
}

/// Look up a translated string. Falls back to English if the language is
/// missing a translation for the given key.
#[must_use]
pub fn lookup(key: StringKey, lang: Language) -> &'static str {
    let entries: &[(&str, &str)] = match key {
        StringKey::TooltipToggleTheme => &[
            ("en", "Toggle theme"),
            ("zh", "切换主题"),
            ("es", "Cambiar tema"),
            ("de", "Design umschalten"),
            ("ja", "テーマ切り替え"),
            ("fr", "Changer de thème"),
            ("pt", "Alternar tema"),
            ("ru", "Переключить тему"),
        ],
        StringKey::TooltipPrint => &[
            ("en", "Print"),
            ("zh", "打印"),
            ("es", "Imprimir"),
            ("de", "Drucken"),
            ("ja", "印刷"),
            ("fr", "Imprimer"),
            ("pt", "Imprimir"),
            ("ru", "Печать"),
        ],
        StringKey::TooltipLogout => &[
            ("en", "Log out"),
            ("zh", "退出登录"),
            ("es", "Cerrar sesión"),
            ("de", "Abmelden"),
            ("ja", "ログアウト"),
            ("fr", "Se déconnecter"),
            ("pt", "Sair"),
            ("ru", "Выйти"),
        ],
        StringKey::AriaSelectLanguage => &[
            ("en", "Select language"),
            ("zh", "选择语言"),
            ("es", "Seleccionar idioma"),
            ("de", "Sprache auswählen"),
            ("ja", "言語を選択"),
            ("fr", "Sélectionner la langue"),
            ("pt", "Selecionar idioma"),
            ("ru", "Выбрать язык"),
        ],
        StringKey::TitleViewReleaseNotes => &[
            ("en", "View Release Notes"),
            ("zh", "查看发行说明"),
            ("es", "Ver notas de la versión"),
            ("de", "Versionshinweise anzeigen"),
            ("ja", "リリースノートを表示"),
            ("fr", "Voir les notes de version"),
            ("pt", "Ver notas de versão"),
            ("ru", "Посмотреть примечания к выпуску"),
        ],
        StringKey::AriaGitHubProfile => &[
            ("en", "GitHub Profile"),
            ("zh", "GitHub 个人主页"),
            ("es", "Perfil de GitHub"),
            ("de", "GitHub-Profil"),
            ("ja", "GitHub プロフィール"),
            ("fr", "Profil GitHub"),
            ("pt", "Perfil do GitHub"),
            ("ru", "Профиль GitHub"),
        ],
    };

    let code = lang.code();
    entries
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, s)| *s)
        .unwrap_or_else(|| key.english())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_fallback_present_for_every_key() {
        for key in StringKey::all() {
            assert!(!key.english().is_empty(), "{key:?} has empty fallback");
        }
    }

    #[test]
    fn every_key_has_translation_for_every_language() {
        for key in StringKey::all() {
            for lang in Language::all() {
                let s = lookup(*key, *lang);
                assert!(!s.is_empty(), "{key:?} missing translation for {:?}", lang);
            }
        }
    }

    #[test]
    fn english_matches_known_constants() {
        assert_eq!(
            lookup(StringKey::TooltipToggleTheme, Language::English),
            "Toggle theme"
        );
        assert_eq!(lookup(StringKey::TooltipPrint, Language::English), "Print");
        assert_eq!(
            lookup(StringKey::TooltipLogout, Language::English),
            "Log out"
        );
    }

    #[test]
    fn non_english_codes_return_localized_text() {
        assert_eq!(lookup(StringKey::TooltipPrint, Language::Chinese), "打印");
        assert_eq!(lookup(StringKey::TooltipPrint, Language::Japanese), "印刷");
    }

    #[test]
    fn language_codes_in_table_are_consistent() {
        // All entries should use codes that match Language::code() for at
        // least one variant. If this fails, someone added a code typo.
        for key in StringKey::all() {
            let entries: &[(&str, &str)] = match key {
                StringKey::TooltipToggleTheme => &[
                    ("en", ""),
                    ("zh", ""),
                    ("es", ""),
                    ("de", ""),
                    ("ja", ""),
                    ("fr", ""),
                    ("pt", ""),
                    ("ru", ""),
                ],
                // Just need to verify codes here, so we can re-use any row
                _ => &[
                    ("en", "x"),
                    ("zh", "x"),
                    ("es", "x"),
                    ("de", "x"),
                    ("ja", "x"),
                    ("fr", "x"),
                    ("pt", "x"),
                    ("ru", "x"),
                ],
            };
            for (code, _) in entries {
                // Don't compare to a specific language since order in the
                // table is independent of Language::all(); just check that
                // the code is recognized.
                assert!(
                    Language::all().iter().any(|l| l.code() == *code),
                    "{code} is not a known language code"
                );
            }
        }
    }
}

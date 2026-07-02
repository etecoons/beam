//! Shared header component — title bar with theme/language/print/logout controls.

use crate::i18n::Language;
use crate::i18n::strings::{StringKey, lookup};
use crate::theme::Theme;
use yew::prelude::*;

/// Props for [`Header`].
#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub site_title: String,
    /// Theme name (e.g. `"crateria"`, `"brinstar"`). Parsed into the
    /// [`Theme`] enum inside the component; unrecognised names fall
    /// back to [`Theme::default`].
    #[prop_or_default]
    pub theme: String,
    pub language: Language,
    pub toggle_theme: Callback<MouseEvent>,
    pub on_language_change: Callback<Language>,
    pub is_authenticated: bool,
    pub pin_required: bool,
    pub on_logout: Callback<MouseEvent>,

    #[prop_or_default]
    pub logout_tooltip: String,
    #[prop_or_default]
    pub theme_toggle_tooltip: String,
    #[prop_or_default]
    pub print_tooltip: String,
    pub on_print: Option<Callback<MouseEvent>>,

    #[prop_or(true)]
    pub enable_translation: bool,
    #[prop_or(true)]
    pub enable_themes: bool,
    #[prop_or(true)]
    pub enable_print: bool,
    pub print_disabled: bool,

    #[prop_or_default]
    pub site_url: Option<String>,
    #[prop_or_default]
    pub version: Option<String>,
    #[prop_or_default]
    pub version_url: Option<String>,
}

/// Top-of-page navigation bar shared by all companion apps.
#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let on_change_lang = {
        let on_lang_change = props.on_language_change.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            on_lang_change.emit(Language::from_code(&select.value()));
        })
    };

    let disabled = !props.is_authenticated || !props.pin_required;
    let onclick_logout = if disabled {
        Callback::from(|_| ())
    } else {
        props.on_logout.clone()
    };

    let theme_tooltip = tooltip_or_override(
        &props.theme_toggle_tooltip,
        StringKey::TooltipToggleTheme,
        props.language,
    );
    let print_tooltip = tooltip_or_override(
        &props.print_tooltip,
        StringKey::TooltipPrint,
        props.language,
    );
    let logout_tooltip = tooltip_or_override(
        &props.logout_tooltip,
        StringKey::TooltipLogout,
        props.language,
    );

    let on_print = props.on_print.clone().unwrap_or_else(|| {
        Callback::from(|_| {
            if let Some(window) = web_sys::window() {
                let _ = window.print();
            }
        })
    });

    // Parse the theme name into the `Theme` enum. Accept either the
    // kebab-case CSS names ("wrecked_ship") or any other value the
    // user stored in localStorage; unknown values fall back to default.
    let theme = Theme::from_name(&props.theme).unwrap_or_default();

    let site_url = props.site_url.clone().unwrap_or_else(|| {
        format!("https://github.com/UberMetroid/{}", props.site_title.to_lowercase())
    });

    let title_html = html! {
        <a class="header-title-link" href={site_url} target="_blank" rel="noopener noreferrer">
            <h1>{&props.site_title}</h1>
        </a>
    };

    let version_html = match &props.version {
        Some(ver) => {
            let version_url = props.version_url.clone().unwrap_or_else(|| {
                format!("https://github.com/UberMetroid/{}/releases/tag/v{}", props.site_title.to_lowercase(), ver)
            });
            html! {
                <a class="header-version-link" href={version_url} target="_blank" rel="noopener noreferrer">
                    <span class="header-version">{format!("v{}", ver)}</span>
                </a>
            }
        }
        None => html! {},
    };

    html! {
        <header>
            <div id="header-title">
                {title_html}
                {version_html}
            </div>

            <div class="header-right">
                {language_picker(props.enable_translation, props.language, on_change_lang)}
                {theme_toggle(props.enable_themes, props.toggle_theme.clone(), theme, theme_tooltip)}
                {print_button(props.enable_print, props.print_disabled, on_print, print_tooltip)}
                {logout_button(props.pin_required, disabled, onclick_logout, logout_tooltip)}
            </div>
        </header>
    }
}

/// Returns the override tooltip if non-empty, otherwise the localized default.
fn tooltip_or_override(override_text: &str, key: StringKey, lang: Language) -> String {
    if !override_text.is_empty() {
        return override_text.to_string();
    }
    lookup(key, lang).to_string()
}

#[allow(clippy::too_many_arguments)]
fn language_picker(enabled: bool, current: Language, on_change: Callback<Event>) -> Html {
    if !enabled {
        return html! {};
    }
    let aria = lookup(StringKey::AriaSelectLanguage, current);
    html! {
        <div class="language-select-container">
            <select
                class="language-select"
                id="language-select"
                value={current.code()}
                onchange={on_change}
                aria-label={aria}
            >
                {for Language::all().iter().map(|lang| {
                    html! {
                        <option value={lang.code()} selected={current == *lang}>
                            {lang.label()}
                        </option>
                    }
                })}
            </select>
        </div>
    }
}

fn theme_toggle(
    enabled: bool,
    on_click: Callback<MouseEvent>,
    theme: Theme,
    tooltip: String,
) -> Html {
    if !enabled {
        return html! {};
    }
    html! {
        <button id="theme-toggle" class="icon-button"
                onclick={on_click}
                aria-label="Toggle theme"
                title={tooltip}>
            {theme.icon_html()}
        </button>
    }
}

fn print_button(
    enabled: bool,
    disabled: bool,
    on_click: Callback<MouseEvent>,
    tooltip: String,
) -> Html {
    if !enabled {
        return html! {};
    }
    html! {
        <button id="print-button" class="icon-button"
                onclick={on_click}
                disabled={disabled}
                title={tooltip}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2"
                 stroke-linecap="round" stroke-linejoin="round">
                <polyline points="6 9 6 2 18 2 18 9" />
                <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
                <rect x="6" y="14" width="12" height="8" />
            </svg>
        </button>
    }
}

fn logout_button(
    pin_required: bool,
    disabled: bool,
    on_click: Callback<MouseEvent>,
    tooltip: String,
) -> Html {
    if !pin_required {
        return html! {};
    }
    html! {
        <button id="logout-button" class="icon-button"
                onclick={on_click}
                disabled={disabled}
                title={if disabled { String::new() } else { tooltip }}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2"
                 stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
            </svg>
        </button>
    }
}

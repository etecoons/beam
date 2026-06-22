use crate::types::Language;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub site_title: String,
    pub theme: String,
    pub is_authenticated: bool,
    pub pin_required: bool,
    pub language: Language,
    pub toggle_theme: Callback<MouseEvent>,
    pub on_logout: Callback<MouseEvent>,
    pub on_language_change: Callback<Language>,
    pub logout_tooltip: String,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let theme = &props.theme;
    let on_toggle = props.toggle_theme.clone();
    let site_title = &props.site_title;
    let language = props.language;
    let on_logout = props.on_logout.clone();
    let logout_tooltip = &props.logout_tooltip;
    let is_authenticated = props.is_authenticated;
    let pin_required = props.pin_required;

    let on_change_lang = {
        let on_lang_change = props.on_language_change.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let lang = match select.value().as_str() {
                "zh" => Language::Chinese,
                "es" => Language::Spanish,
                "de" => Language::German,
                "ja" => Language::Japanese,
                "fr" => Language::French,
                "pt" => Language::Portuguese,
                "ru" => Language::Russian,
                _ => Language::English,
            };
            on_lang_change.emit(lang);
        })
    };

    let disabled = !is_authenticated || !pin_required;
    let onclick_handler = if disabled {
        Callback::from(|_| ())
    } else {
        on_logout
    };

    let theme_toggle_icon = match theme.as_str() {
        "dark" => html! {
            <svg id="moon-icon" class="moon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" /></svg>
        },
        "nord" => html! {
            <svg id="droplet-icon" class="droplet" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22a7 7 0 0 0 7-7c0-4.3-7-13-7-13S5 10.7 5 15a7 7 0 0 0 7 7z"/></svg>
        },
        "dracula" => html! {
            <svg id="sparkles-icon" class="sparkles" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275Z"/><path d="m5 3 1 2.5L8.5 6 6 7 5 9.5 4 7 1.5 6 4 5Z"/><path d="m19 17 1 2.5 2.5.5-2.5 1-1 2.5-1-2.5-2.5-1 2.5-1Z"/></svg>
        },
        "sepia" => html! {
            <svg id="coffee-icon" class="coffee" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 8h1a4 4 0 1 1 0 8h-1"/><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"/><line x1="6" y1="2" x2="6" y2="4"/><line x1="10" y1="2" x2="10" y2="4"/><line x1="14" y1="2" x2="14" y2="4"/></svg>
        },
        _ => html! {
            <svg id="sun-icon" class="sun" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2" /><path d="M12 20v2" /><path d="M4.93 4.93l1.41 1.41" /><path d="M17.66 17.66l1.41 1.41" /><path d="M2 12h2" /><path d="M20 12h2" /><path d="M6.34 17.66l-1.41 1.41" /><path d="M19.07 4.93l-1.41 1.41" /></svg>
        },
    };

    html! {
        <header>
            <div id="header-title">
                <h1>{site_title}</h1>
            </div>
            <div class="header-right">
                <div class="language-select-container">
                    <select
                        class="language-select"
                        id="language-select"
                        value={language.code()}
                        onchange={on_change_lang}
                        aria-label="Select language"
                    >
                        {for Language::all().iter().map(|lang| {
                            html! {
                                <option value={lang.code()} selected={language == *lang}>
                                    {lang.label()}
                                </option>
                            }
                        })}
                    </select>
                </div>
                <button id="theme-toggle" class="icon-button" onclick={on_toggle} aria-label="Toggle theme">
                    {theme_toggle_icon}
                </button>
                <button
                    id="logout-button"
                    class="icon-button"
                    onclick={onclick_handler}
                    disabled={disabled}
                    data-tooltip={if disabled { "".to_string() } else { logout_tooltip.clone() }}
                >
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                        <polyline points="16 17 21 12 16 7" />
                        <line x1="21" y1="12" x2="9" y2="12" />
                    </svg>
                </button>
            </div>
        </header>
    }
}

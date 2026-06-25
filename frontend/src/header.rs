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
    pub disable_print: bool,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
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
        "brinstar" => html! {
            <svg id="leaf-icon" class="leaf" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 3.5 1 9.8a7 7 0 0 1-9 8.2Z" /><path d="M19 2 9.8 11.5" /></svg>
        },
        "norfair" => html! {
            <svg id="flame-icon" class="flame" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" /></svg>
        },
        "wrecked_ship" => html! {
            <svg id="ghost-icon" class="ghost" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 10h.01"/><path d="M15 10h.01"/><path d="M12 2a8 8 0 0 0-8 8v12l3-3 2.5 2.5L12 19l2.5 2.5L17 19l3 3V10a8 8 0 0 0-8-8z"/></svg>
        },
        "maridia" => html! {
            <svg id="waves-icon" class="waves" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 6c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /><path d="M2 12c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /><path d="M2 18c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /></svg>
        },
        "tourian" => html! {
            <svg id="target-icon" class="target" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" /><circle cx="12" cy="12" r="6" /><circle cx="12" cy="12" r="2" /></svg>
        },
        _ => html! {
            <svg id="cloud-rain-icon" class="cloud-rain" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 17.58A5 5 0 0 0 18 8h-1.26A8 8 0 1 0 4 16.25" /><path d="M8 20v2" /><path d="M12 20v2" /><path d="M16 20v2" /></svg>
        },
    };

    let theme_toggle_tooltip = match language {
        Language::Chinese => "切换主题",
        Language::Spanish => "Cambiar tema",
        Language::German => "Design umschalten",
        Language::Japanese => "テーマ切り替え",
        Language::French => "Changer de thème",
        Language::Portuguese => "Alternar tema",
        Language::Russian => "Переключить тему",
        _ => "Toggle theme",
    };

    let print_tooltip = match language {
        Language::Chinese => "打印",
        Language::Spanish => "Imprimir",
        Language::German => "Drucken",
        Language::Japanese => "印刷",
        Language::French => "Imprimer",
        Language::Portuguese => "Imprimir",
        Language::Russian => "Печать",
        _ => "Print",
    };

    let on_print = Callback::from(|_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    });

    html! {
        <header>
            <div id="header-title">
                <h1>{site_title}</h1>
            </div>
            <div class="header-right">
                {if props.enable_translation {
                    html! {
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
                    }
                } else {
                    html! {}
                }}
                {if props.enable_themes {
                    html! {
                        <button id="theme-toggle" class="icon-button" onclick={on_toggle} aria-label="Toggle theme" title={theme_toggle_tooltip}>
                            {theme_toggle_icon}
                        </button>
                    }
                } else {
                    html! {}
                }}
                {if props.enable_print {
                    html! {
                        <button
                            id="print-button"
                            class="icon-button"
                            onclick={on_print}
                            disabled={props.disable_print}
                            title={print_tooltip}
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <polyline points="6 9 6 2 18 2 18 9" />
                                <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
                                <rect x="6" y="14" width="12" height="8" />
                            </svg>
                        </button>
                    }
                } else {
                    html! {}
                }}
                {if props.pin_required {
                    html! {
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
                    }
                } else {
                    html! {}
                }}
            </div>
        </header>
    }
}

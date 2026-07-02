//! Shared footer component — version, optional children, optional GitHub link.

use crate::i18n::Language;
use crate::i18n::strings::{StringKey, lookup};
use yew::prelude::*;

/// Props for [`Footer`].
#[derive(Properties, PartialEq, Clone)]
pub struct FooterProps {
    #[prop_or_default]
    pub show_version: bool,
    #[prop_or_default]
    pub version: String,
    #[prop_or(true)]
    pub show_github: bool,
    #[prop_or_default]
    pub github_url: Option<String>,

    #[prop_or_default]
    pub version_url: Option<String>,

    #[prop_or(true)]
    pub show_coffee: bool,
    #[prop_or_default]
    pub coffee_url: Option<String>,

    #[prop_or_default]
    pub children: Html,
}

/// Bottom-of-page footer shared by all companion apps.
#[function_component(Footer)]
pub fn footer(props: &FooterProps) -> Html {
    let github_link = props
        .github_url
        .clone()
        .unwrap_or_else(|| "https://github.com/UberMetroid".to_string());

    let coffee_link = props
        .coffee_url
        .clone()
        .unwrap_or_else(|| "https://buymeacoffee.com/ubermetroid".to_string());

    let aria_github = lookup(StringKey::AriaGitHubProfile, Language::English);

    html! {
        <footer class="layout-footer">
            <div class="footer-left">
                {if props.show_github {
                    html! {
                        <a class="footer-github-link"
                           href={github_link}
                           target="_blank"
                           rel="noopener noreferrer"
                           aria-label={aria_github}>
                            <svg class="github-icon"
                                 width="16" height="16"
                                 viewBox="0 0 24 24" fill="none"
                                 stroke="currentColor" stroke-width="2"
                                 stroke-linecap="round" stroke-linejoin="round">
                                <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
                                <path d="M9 18c-4.51 2-5-2-7-2" />
                            </svg>
                            <span>{"GitHub"}</span>
                        </a>
                    }
                } else {
                    html! {}
                }}
                {version_block(props.show_version, &props.version, props.version_url.as_deref())}
            </div>

            <div class="footer-center">
                {props.children.clone()}
            </div>

            <div class="footer-right">
                {if props.show_coffee {
                    html! {
                        <a class="footer-coffee-link"
                           href={coffee_link}
                           target="_blank"
                           rel="noopener noreferrer"
                           title="Buy Me a Coffee">
                            <svg class="coffee-icon"
                                 width="16" height="16"
                                 viewBox="0 0 24 24" fill="none"
                                 stroke="currentColor" stroke-width="2"
                                 stroke-linecap="round" stroke-linejoin="round">
                                <path d="M17 8h1a4 4 0 1 1 0 8h-1" />
                                <path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z" />
                                <line x1="6" y1="2" x2="6" y2="4" />
                                <line x1="10" y1="2" x2="10" y2="4" />
                                <line x1="14" y1="2" x2="14" y2="4" />
                            </svg>
                            <span>{"Coffee"}</span>
                        </a>
                    }
                } else {
                    html! {}
                }}
            </div>
        </footer>
    }
}

/// Renders the version link or static version text, depending on whether a URL is set.
fn version_block(show: bool, version: &str, url: Option<&str>) -> Html {
    if !show {
        return html! {};
    }
    let display = format!("v{version}");
    match url {
        Some(u) => {
            let title = lookup(StringKey::TitleViewReleaseNotes, Language::English);
            html! {
                <a class="footer-version-link"
                   href={u.to_string()}
                   target="_blank"
                   rel="noopener noreferrer"
                   title={title}>
                    {display}
                </a>
            }
        }
        None => html! {
            <span class="footer-version">{display}</span>
        },
    }
}

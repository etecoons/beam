//! SVG icons rendered for each theme in the header toggle button.
//!
//! Icons are sourced from Lucide (ISC license) and rendered via Yew's
//! `html!` macro. Each function returns the `<svg>` element for one theme.

use super::Theme;
use yew::prelude::*;

const COMMON_PROPS: (&str, &str, &str, &str, &str, &str) =
    ("24", "24", "0 0 24 24", "none", "currentColor", "2");

/// Dispatch to the correct icon for a theme.
pub fn icon(theme: Theme) -> Html {
    match theme {
        Theme::Brinstar => brinstar(),
        Theme::Norfair => norfair(),
        Theme::WreckedShip => wrecked_ship(),
        Theme::Maridia => maridia(),
        Theme::Tourian => tourian(),
        Theme::Crateria => crateria(),
    }
}

fn brinstar() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="leaf-icon" class="leaf"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 3.5 1 9.8a7 7 0 0 1-9 8.2Z" />
            <path d="M19 2 9.8 11.5" />
        </svg>
    }
}

fn norfair() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="flame-icon" class="flame"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" />
        </svg>
    }
}

fn wrecked_ship() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="ghost-icon" class="ghost"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 10h.01" />
            <path d="M15 10h.01" />
            <path d="M12 2a8 8 0 0 0-8 8v12l3-3 2.5 2.5L12 19l2.5 2.5L17 19l3 3V10a8 8 0 0 0-8-8z" />
        </svg>
    }
}

fn maridia() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="waves-icon" class="waves"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 6c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" />
            <path d="M2 12c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" />
            <path d="M2 18c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" />
        </svg>
    }
}

fn tourian() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="target-icon" class="target"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10" />
            <circle cx="12" cy="12" r="6" />
            <circle cx="12" cy="12" r="2" />
        </svg>
    }
}

fn crateria() -> Html {
    let (w, h, vb, fill, sc, sw) = COMMON_PROPS;
    html! {
        <svg id="cloud-rain-icon" class="cloud-rain"
             width={w} height={h} viewBox={vb} fill={fill}
             stroke={sc} stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 17.58A5 5 0 0 0 18 8h-1.26A8 8 0 1 0 4 16.25" />
            <path d="M8 20v2" />
            <path d="M12 20v2" />
            <path d="M16 20v2" />
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_renders_for_every_theme() {
        // Smoke test: each theme produces a VNode without panicking.
        // Full snapshot testing would require Yew's `yew::tests` machinery
        // and a renderer — overkill for icon shapes that rarely change.
        for theme in Theme::ALL {
            let html = icon(*theme);
            let _ = html; // Touch the value so the compiler keeps the call.
        }
    }
}

//! Cookie issuance helpers for session management.
//!
//! Apps call [`issue_cookie`] when a user successfully authenticates to
//! set a `pin` cookie that subsequent requests can present.

use crate::server::ServerConfig;
use axum::http::header::{COOKIE, HeaderValue, SET_COOKIE};

/// Build a `Set-Cookie` header value for a successful PIN session.
///
/// Cookie name: `pin`. Value: a random token (caller-provided).
/// Lifetime: [`ServerConfig::cookie_max_age_hours`].
pub fn build_set_cookie_header(config: &ServerConfig, value: &str) -> String {
    let max_age = config.cookie_max_age_hours * 3600;
    format!("pin={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}")
}

/// Attach the session cookie to an outgoing response.
pub fn issue_cookie(config: &ServerConfig, value: &str, response: &mut axum::response::Response) {
    let header = build_set_cookie_header(config, value);
    if let Ok(v) = HeaderValue::from_str(&header) {
        response.headers_mut().append(SET_COOKIE, v);
    }
}

/// Read the `pin` cookie value from a request, if present.
#[must_use]
pub fn read_cookie(request: &axum::extract::Request) -> Option<String> {
    let header = request.headers().get(COOKIE)?.to_str().ok()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(rest) = pair.strip_prefix("pin=") {
            return Some(rest.to_string());
        }
    }
    None
}

/// Extract a PIN value from the request cookie (if any).
pub fn read_pin_cookie(request: &axum::extract::Request) -> Option<String> {
    read_cookie(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ServerConfig {
        ServerConfig {
            port: 4401,
            site_title: "X".into(),
            base_url: "http://localhost".into(),
            allowed_origins: "*".into(),
            pin: Some("12345678".into()),
            enable_translation: false,
            enable_themes: false,
            enable_print: false,
            show_version: true,
            show_github: true,
            trust_proxy: false,
            trusted_proxies: vec![],
            max_attempts: 5,
            lockout_time_minutes: 15,
            cookie_max_age_hours: 24,
            shutdown_drain_seconds: 5,
        }
    }

    #[test]
    fn cookie_header_contains_value_and_max_age() {
        let h = build_set_cookie_header(&cfg(), "abc123");
        assert!(h.contains("pin=abc123"));
        assert!(h.contains("Max-Age=86400")); // 24h
        assert!(h.contains("HttpOnly"));
        assert!(h.contains("SameSite=Strict"));
    }

    #[test]
    fn cookie_max_age_scales_with_config() {
        let mut c = cfg();
        c.cookie_max_age_hours = 1;
        assert!(build_set_cookie_header(&c, "x").contains("Max-Age=3600"));
    }
}

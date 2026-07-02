//! CORS layer factory.
//!
//! Builds a `tower-http` CORS layer from [`ServerConfig::allowed_origins`].
//! A value of `"*"` produces a permissive layer; comma-separated origins
//! are added individually with credentials allowed.

use crate::server::ServerConfig;
use axum::http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::CorsLayer;

/// Build a CORS layer configured per `ALLOWED_ORIGINS`.
pub fn cors_layer(config: &ServerConfig) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
    ];
    let headers = [
        header::CONTENT_TYPE,
        header::COOKIE,
        header::AUTHORIZATION,
        HeaderName::from_static("x-pin"),
    ];

    let trimmed = config.allowed_origins.trim();
    if trimmed == "*" || trimmed.is_empty() {
        return CorsLayer::permissive();
    }

    let mut layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers)
        .allow_credentials(true);

    for origin in config.allowed_origins.split(',') {
        if let Ok(parsed) = HeaderValue::from_str(origin.trim()) {
            layer = layer.allow_origin(parsed);
        }
    }
    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with(origins: &str) -> ServerConfig {
        ServerConfig {
            port: 4401,
            site_title: "X".into(),
            base_url: "http://localhost".into(),
            allowed_origins: origins.into(),
            pin: None,
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
    fn wildcard_builds_without_panic() {
        let _ = cors_layer(&cfg_with("*"));
    }

    #[test]
    fn multiple_origins_build_without_panic() {
        let _ = cors_layer(&cfg_with(
            "https://app.example.com,https://other.example.com",
        ));
    }

    #[test]
    fn empty_origins_falls_back_to_wildcard() {
        // Empty string should not panic; falls back to permissive.
        let _ = cors_layer(&cfg_with(""));
    }
}

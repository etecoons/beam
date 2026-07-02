//! HSTS middleware.
//!
//! Adds `Strict-Transport-Security` when the request indicates HTTPS via
//! `X-Forwarded-Proto` (when behind a trusted proxy) or `config.base_url`.

use axum::extract::{Request, State};
use axum::http::header::{HeaderValue, STRICT_TRANSPORT_SECURITY};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;

use crate::server::ServerConfig;

/// State wrapper so apps can install the layer with `.layer(...)`.
#[derive(Clone)]
pub struct HstsState(pub Arc<ServerConfig>);

/// Add HSTS header when the connection is HTTPS.
pub async fn hsts_layer(State(state): State<HstsState>, request: Request, next: Next) -> Response {
    let is_secure = request
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or_else(|| state.0.base_url.starts_with("https"));

    let mut response = next.run(request).await;
    if is_secure {
        response.headers_mut().insert(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles() {
        fn _exists() {
            let _: fn(State<HstsState>, Request, Next) -> _ = hsts_layer;
        }
    }
}

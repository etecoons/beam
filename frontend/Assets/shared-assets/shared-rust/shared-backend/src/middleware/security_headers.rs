//! Security headers middleware.
//!
//! Adds a standard set of security headers to every response:
//!
//! - `X-Frame-Options: DENY`
//! - `X-Content-Type-Options: nosniff`
//! - `Referrer-Policy: strict-origin-when-cross-origin`
//! - `Content-Security-Policy: <default-src 'self' …>` (Yew-compatible)

use axum::extract::Request;
use axum::http::header::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

/// Add security headers to every response.
///
/// The CSP includes `'unsafe-eval'` because Yew's CSR runtime requires it
/// in some build configurations.
///
/// `connect-src` is intentionally restricted to `'self' ws: wss:` so that
/// client-side fetches are limited to the same origin or WebSocket-only.
pub async fn security_headers_layer(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
             img-src 'self' data: blob: https:; \
             connect-src 'self' ws: wss:; \
             font-src 'self'; \
             manifest-src 'self';",
        ),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pure function composition; full integration testing happens via the
    // axum test client in each app's test suite. Smoke test: it compiles.
    #[test]
    fn compiles() {
        fn _exists() {
            let _: fn(Request, Next) -> _ = security_headers_layer;
        }
    }
}

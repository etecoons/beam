//! PIN authentication middleware.
//!
//! Apps wrap protected routes with [`pin_auth_layer`]. The middleware:
//!
//! - Allows unauthenticated requests through (apps add their own auth routes)
//! - Checks a session cookie OR `X-PIN` header against the configured PIN
//! - Locks out brute-force attempts using [`attempts`]
//! - Issues a session cookie on successful auth

use super::attempts;
use crate::server::{ServerConfig, get_client_ip};
use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use constant_time_eq::constant_time_eq;
use std::net::SocketAddr;
use std::sync::Arc;

/// Wrapper type so apps can store `PinState` in their `AppState`.
#[derive(Clone)]
pub struct PinState(pub Arc<ServerConfig>);

impl From<Arc<ServerConfig>> for PinState {
    fn from(c: Arc<ServerConfig>) -> Self {
        Self(c)
    }
}

/// Axum middleware that gates routes behind the configured PIN.
///
/// On a request with a valid PIN (via cookie or `X-PIN` header), the
/// request proceeds. Failed attempts are recorded; once `max_attempts` is
/// exceeded, requests from that IP are rejected with 429 until the lockout
/// expires.
pub async fn pin_auth_layer(
    State(state): State<PinState>,
    ConnectInfo(socket): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let config = &state.0;
    let Some(expected_pin) = config.pin.as_deref() else {
        // No PIN configured — public mode, no auth needed.
        return Ok(next.run(request).await);
    };

    let ip = get_client_ip(
        request.headers(),
        socket,
        config.trust_proxy,
        &config.trusted_proxies,
    );
    let lockout = config.lockout_duration();

    if attempts::is_locked_out(&ip, config.max_attempts, lockout) {
        let remaining = attempts::lockout_remaining_secs(&ip, lockout);
        tracing::warn!(
            target: "auth",
            "IP {ip} locked out for {remaining}s more; rejecting request with empty body"
        );
        return Err(axum::http::Response::builder()
            .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "text/plain")
            .body(axum::body::Body::empty())
            .unwrap());
    }

    match extract_pin(&request) {
        Some(p) if constant_time_eq(expected_pin.as_bytes(), p.as_bytes()) => {
            attempts::reset_attempts(&ip);
            Ok(next.run(request).await)
        }
        Some(_) => {
            let attempt = attempts::record_attempt(&ip);
            tracing::warn!(
                target: "auth",
                "failed PIN attempt #{count} from {ip}",
                count = attempt.count
            );
            if attempt.count >= config.max_attempts {
                tracing::warn!(target: "auth", "IP {ip} locked out");
            }
            Err(unauthorized_response())
        }
        None => Err(unauthorized_response()),
    }
}

/// Extract a PIN from either the `X-PIN` header or a `pin=...` cookie.
fn extract_pin(request: &Request) -> Option<String> {
    if let Some(p) = request.headers().get("x-pin").and_then(|h| h.to_str().ok())
        && !p.is_empty()
    {
        return Some(p.to_string());
    }
    crate::auth::read_pin_cookie(request)
}

fn unauthorized_response() -> Response {
    axum::http::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .header("content-type", "text/plain")
        .body(axum::body::Body::from("unauthorized"))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as AxumRequest;

    #[test]
    fn extract_pin_from_header() {
        let mut req = AxumRequest::default();
        req.headers_mut().insert("x-pin", "1234".parse().unwrap());
        assert_eq!(extract_pin(&req), Some("1234".to_string()));
    }

    #[test]
    fn extract_pin_from_cookie() {
        let mut req = AxumRequest::default();
        req.headers_mut().insert(
            axum::http::header::COOKIE,
            "pin=abcd; other=foo".parse().unwrap(),
        );
        assert_eq!(extract_pin(&req), Some("abcd".to_string()));
    }

    #[test]
    fn extract_pin_none_when_missing() {
        let req = AxumRequest::<Body>::default();
        assert_eq!(extract_pin(&req), None);
    }
}

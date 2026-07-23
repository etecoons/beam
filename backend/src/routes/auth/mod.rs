use crate::security::{get_client_ip, safe_compare};
use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRef, FromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;

pub mod logout;
pub mod pin_required;
pub mod verify_pin;

// Extractor to require a valid PIN if one is configured
pub struct RequirePin;

impl<S> FromRequestParts<S> for RequirePin
where
    crate::AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = crate::AppState::from_ref(state);
        let config = &app_state.config;

        if let Some(ref pin) = config.pin {
            let jar = CookieJar::from_headers(&parts.headers);
            let cookie_pin = jar.get("BEAM_PIN").map(|c| c.value());
            let header_pin = parts.headers.get("x-pin").and_then(|h| h.to_str().ok());

            let authenticated = match (cookie_pin, header_pin) {
                (Some(cookie), _) => app_state.active_sessions.read().await.contains(cookie),
                (None, Some(hdr)) => safe_compare(hdr, pin),
                (None, None) => false,
            };

            if authenticated {
                return Ok(RequirePin);
            }

            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "success": false, "error": "Unauthorized: Invalid or missing PIN" })),
            )
                .into_response());
        }

        Ok(RequirePin)
    }
}

// Request payload for PIN verification
#[derive(Deserialize)]
pub struct VerifyPinPayload {
    pub pin: Option<String>,
}

// Response payload for PIN verification
#[derive(Serialize)]
pub struct VerifyPinResponse {
    pub success: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/pin-required", get(pin_required::pin_required))
        .route("/verify-pin", post(verify_pin::verify_pin))
        .route("/logout", post(logout::logout))
        .route("/config", get(pin_required::get_config))
}

pub async fn rate_limit_middleware(
    State(state): State<crate::AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);

    let ip = get_client_ip(
        req.headers(),
        addr.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0))),
        state.config.trust_proxy,
        state.config.trusted_proxy_ips.as_deref(),
    );

    let ip_addr = ip
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    if !state.check_rate_limit(ip_addr).await {
        let body = serde_json::json!({
            "error": "Too many requests. Please slow down."
        });
        let mut response = axum::response::Json(body).into_response();
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        return Ok(response);
    }

    Ok(next.run(req).await)
}

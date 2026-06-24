use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{HeaderMap, StatusCode, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::security::{
    get_client_ip, get_lockout_time_remaining, get_max_attempts, hash_pin, is_locked_out, record_attempt,
    reset_attempts, safe_compare,
};

// Extractor to require a valid PIN if one is configured
pub struct RequirePin;

impl<S> FromRequestParts<S> for RequirePin
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let config = parts.extensions.get::<Arc<AppConfig>>().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal config missing" })),
            )
                .into_response()
        })?;

        if let Some(ref pin) = config.pin {
            let jar = CookieJar::from_headers(&parts.headers);
            let cookie_pin = jar.get("RUSTDROP_PIN").map(|c| c.value());
            let header_pin = parts.headers.get("x-pin").and_then(|h| h.to_str().ok());

            let authenticated = match (cookie_pin, header_pin) {
                (Some(cookie), _) => safe_compare(cookie, &hash_pin(pin)),
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
struct VerifyPinPayload {
    pin: Option<String>,
}

// Response payload for PIN verification
#[derive(Serialize)]
struct VerifyPinResponse {
    success: bool,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Serialize)]
struct FrontendConfig {
    site_title: String,
    auto_upload: bool,
    show_file_list: bool,
    pin_required: bool,
    pin_length: usize,
    max_file_size: u64,
    client_max_retries: u32,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/pin-required", get(pin_required))
        .route("/verify-pin", post(verify_pin))
        .route("/logout", post(logout))
        .route("/config", get(get_config))
}

async fn get_config(State(config): State<Arc<AppConfig>>) -> Json<FrontendConfig> {
    Json(FrontendConfig {
        site_title: config.site_title.clone(),
        auto_upload: config.auto_upload,
        show_file_list: config.show_file_list,
        pin_required: config.pin.is_some(),
        pin_length: config.pin.as_ref().map(|p| p.len()).unwrap_or(0),
        max_file_size: config.max_file_size,
        client_max_retries: config.client_max_retries,
    })
}

async fn pin_required(State(config): State<Arc<AppConfig>>) -> Json<serde_json::Value> {
    let length = config.pin.as_ref().map(|p| p.len()).unwrap_or(0);
    Json(json!({
        "required": config.pin.is_some(),
        "length": length,
        "enable_translation": config.enable_translation
    }))
}

async fn verify_pin(
    State(config): State<Arc<AppConfig>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(payload): Json<VerifyPinPayload>,
) -> impl IntoResponse {
    let ip = get_client_ip(
        &headers,
        addr,
        config.trust_proxy,
        config.trusted_proxy_ips.as_deref(),
    );

    // 1. If PIN is not set in config, clear cookie and return success
    let Some(ref config_pin) = config.pin else {
        let new_jar = jar.add(Cookie::build(("RUSTDROP_PIN", "")).path("/").build());
        let res = (
            StatusCode::OK,
            Json(VerifyPinResponse {
                success: true,
                error: None,
                path: Some("/".to_string()),
            }),
        )
            .into_response();
        return (new_jar, res).into_response();
    };

    // 2. Validate empty/missing PIN (returns 400)
    let pin_str = payload.pin.as_deref().unwrap_or("").trim();
    if pin_str.is_empty() {
        let res = (
            StatusCode::BAD_REQUEST,
            Json(VerifyPinResponse {
                success: false,
                error: Some("PIN is required.".to_string()),
                path: None,
            }),
        )
            .into_response();
        return (jar, res).into_response();
    }

    // 3. Check for lockout
    if is_locked_out(&ip) {
        let _ = record_attempt(&ip);
        let time_left = get_lockout_time_remaining(&ip);
        let minutes_left = (time_left as f64 / 60.0).ceil() as u64;

        tracing::warn!("Login attempt from locked out IP: {}", ip);
        let res = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(VerifyPinResponse {
                success: false,
                error: Some(format!(
                    "Too many PIN verification attempts. Please try again in {} minutes.",
                    minutes_left
                )),
                path: None,
            }),
        )
            .into_response();
        return (jar, res).into_response();
    }

    // 4. Verify PIN
    if safe_compare(pin_str, config_pin) {
        reset_attempts(&ip);

        let is_secure = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("https"))
            .unwrap_or_else(|| config.base_url.starts_with("https"));

        // Build secure cookie
        let secure_cookie = Cookie::build(("RUSTDROP_PIN", hash_pin(pin_str)))
            .http_only(true)
            .secure(is_secure)
            .same_site(SameSite::Lax)
            .path("/")
            .build();

        let new_jar = jar.add(secure_cookie);

        tracing::info!("Successful PIN verification from IP: {}", ip);
        let res = (
            StatusCode::OK,
            Json(VerifyPinResponse {
                success: true,
                error: None,
                path: None,
            }),
        )
            .into_response();
        (new_jar, res).into_response()
    } else {
        // Record failed attempt
        let attempt = record_attempt(&ip);
        let attempts_left = get_max_attempts().saturating_sub(attempt.count);

        let error_msg = if attempts_left > 0 {
            format!("Invalid PIN. {} attempts remaining.", attempts_left)
        } else {
            "Too many PIN verification attempts. Account locked for 15 minutes.".to_string()
        };

        tracing::warn!(
            "Failed PIN verification from IP: {} ({} attempts remaining)",
            ip,
            attempts_left
        );
        let res = (
            StatusCode::UNAUTHORIZED,
            Json(VerifyPinResponse {
                success: false,
                error: Some(error_msg),
                path: None,
            }),
        )
            .into_response();
        (jar, res).into_response()
    }
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let new_jar = jar.add(Cookie::build(("RUSTDROP_PIN", "")).path("/").build());
    let res = (StatusCode::OK, Json(json!({ "success": true }))).into_response();
    (new_jar, res).into_response()
}

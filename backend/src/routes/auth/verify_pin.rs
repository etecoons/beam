//! PIN verification handler.

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::net::SocketAddr;

use crate::config::AppConfig;
use crate::routes::auth::{VerifyPinPayload, VerifyPinResponse};
use crate::security::{
    get_client_ip, get_lockout_time_remaining, get_max_attempts, is_locked_out, record_attempt,
    reset_attempts, safe_compare,
};

pub async fn verify_pin(
    State(state): State<crate::AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(payload): Json<VerifyPinPayload>,
) -> Response {
    let config: &AppConfig = &state.config;
    let ip = get_client_ip(
        &headers,
        addr,
        config.trust_proxy,
        config.trusted_proxy_ips.as_deref(),
    );

    // 1. No PIN configured: clear cookie, success.
    let Some(config_pin) = config.pin.as_deref() else {
        let new_jar = jar.add(Cookie::build(("BEAM_PIN", "")).path("/").build());
        return (
            new_jar,
            (
                StatusCode::OK,
                Json(VerifyPinResponse {
                    success: true,
                    error: None,
                    path: Some("/".to_string()),
                }),
            ),
        )
            .into_response();
    };

    // 2. Empty PIN: 400.
    let pin_str = payload.pin.as_deref().unwrap_or("").trim();
    if pin_str.is_empty() {
        return (
            jar,
            (
                StatusCode::BAD_REQUEST,
                Json(VerifyPinResponse {
                    success: false,
                    error: Some("PIN is required.".to_string()),
                    path: None,
                }),
            ),
        )
            .into_response();
    }

    // 3. Lockout.
    if is_locked_out(&ip) {
        let time_left = get_lockout_time_remaining(&ip);
        let minutes_left = (time_left as f64 / 60.0).ceil() as u64;
        tracing::warn!("Login attempt from locked out IP: {}", ip);
        return (
            jar,
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(VerifyPinResponse {
                    success: false,
                    error: Some(format!(
                        "Too many PIN verification attempts. Please try again in {} minutes.",
                        minutes_left
                    )),
                    path: None,
                }),
            ),
        )
            .into_response();
    }

    // 4. Compare PIN.
    if safe_compare(pin_str, config_pin) {
        reset_attempts(&ip);
        let is_secure = crate::cookie_auth::cookie_should_be_secure(
            &headers,
            &config.base_url,
        );

        let session_id = crate::session_id::generate_session_id();
        state
            .active_sessions
            .write()
            .await
            .insert(session_id.clone());
        let cookie = crate::cookie_auth::build_cookie(&session_id, config.cookie_max_age_hours, is_secure);
        let new_jar = jar.add(cookie);
        tracing::info!("Successful PIN verification from IP: {}", ip);
        (
            new_jar,
            (
                StatusCode::OK,
                Json(VerifyPinResponse {
                    success: true,
                    error: None,
                    path: None,
                }),
            ),
        )
            .into_response()
    } else {
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
        (
            jar,
            (
                StatusCode::UNAUTHORIZED,
                Json(VerifyPinResponse {
                    success: false,
                    error: Some(error_msg),
                    path: None,
                }),
            ),
        )
            .into_response()
    }
}

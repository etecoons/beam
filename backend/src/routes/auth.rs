use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRef, FromRequestParts, State},
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
    get_client_ip, get_lockout_time_remaining, get_max_attempts, is_locked_out, record_attempt,
    reset_attempts, safe_compare,
};

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

        if let Some(ref pin) = config.server.pin {
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

#[derive(Serialize)]
struct FrontendConfig {
    site_title: String,
    auto_upload: bool,
    show_file_list: bool,
    pin_required: bool,
    pin_length: usize,
    max_file_size: u64,
    client_max_retries: u32,
    enable_translation: bool,
    enable_themes: bool,
    enable_print: bool,
    show_version: bool,
    show_github: bool,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/pin-required", get(pin_required))
        .route("/verify-pin", post(crate::routes::verify_pin::verify_pin))
        .route("/logout", post(logout))
        .route("/config", get(get_config))
}

async fn get_config(State(config): State<Arc<AppConfig>>) -> Json<FrontendConfig> {
    Json(FrontendConfig {
        site_title: config.server.site_title.clone(),
        auto_upload: config.auto_upload,
        show_file_list: config.show_file_list,
        pin_required: config.server.pin.is_some(),
        pin_length: config.server.pin.as_ref().map(|p| p.len()).unwrap_or(0),
        max_file_size: config.max_file_size,
        client_max_retries: config.client_max_retries,
        enable_translation: config.server.enable_translation,
        enable_themes: config.server.enable_themes,
        enable_print: config.server.enable_print,
        show_version: config.server.show_version,
        show_github: config.server.show_github,
    })
}

async fn pin_required(State(config): State<Arc<AppConfig>>) -> Json<serde_json::Value> {
    let length = config.server.pin.as_ref().map(|p| p.len()).unwrap_or(0);
    Json(json!({
        "required": config.server.pin.is_some(),
        "length": length,
        "enable_translation": config.server.enable_translation,
        "enable_themes": config.server.enable_themes,
        "enable_print": config.server.enable_print,
    }))
}

async fn logout(State(state): State<crate::AppState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("BEAM_PIN") {
        state.active_sessions.write().await.remove(cookie.value());
    }
    let new_jar = jar.add(Cookie::build(("BEAM_PIN", "")).path("/").build());
    let res = (StatusCode::OK, Json(json!({ "success": true }))).into_response();
    (new_jar, res).into_response()
}

pub fn generate_session_id() -> String {
    use std::fs::File;
    use std::io::Read;
    let file = File::open("/dev/urandom").ok();
    let mut bytes = [0u8; 16];
    if let Some(mut f) = file
        && f.read_exact(&mut bytes).is_ok()
    {
        return bytes.iter().map(|b| format!("{:02x}", b)).collect();
    }
    let random_val = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(random_val.to_string().as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
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

//! Static file serving and HTML template rendering.

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use std::path::Path;
use std::sync::Arc;

use crate::config::AppConfig;

pub async fn serve_index(State(config): State<Arc<AppConfig>>) -> impl IntoResponse {
    serve_html(config, "index.html").await.into_response()
}

pub async fn serve_login() -> impl IntoResponse {
    Redirect::temporary("/")
}

/// Liveness probe. Returns uptime timestamp + status: ok.
pub async fn serve_health() -> impl IntoResponse {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": secs
    }))
}

pub async fn serve_html(config: Arc<AppConfig>, req_path: &str) -> Response {
    let file_path = Path::new("frontend/dist/index.html");
    let content = match tokio::fs::read_to_string(&file_path).await {
        Ok(c) => c,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let base_url_with_slash = if config.base_url.ends_with('/') {
        config.base_url.clone()
    } else {
        format!("{}/", config.base_url)
    };

    let mut rendered = content
        .replace("{{SITE_TITLE}}", &config.site_title)
        .replace("{{BASE_URL}}", &base_url_with_slash);

    if req_path == "index.html" {
        rendered = rendered
            .replace("{{AUTO_UPLOAD}}", &config.auto_upload.to_string())
            .replace("{{MAX_RETRIES}}", &config.client_max_retries.to_string())
            .replace("{{SHOW_FILE_LIST}}", &config.show_file_list.to_string());
    }

    let mut response = axum::response::Html(rendered).into_response();
    if req_path == "login.html" {
        let h = response.headers_mut();
        h.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
        h.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
        h.insert(header::EXPIRES, HeaderValue::from_static("0"));
    }
    response
}

mod config;
mod routes;
mod security;
mod services;
#[cfg(test)]
mod tests;
mod utils;

use axum::http::StatusCode;
use axum::{
    Extension, Router,
    extract::{FromRef, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;
use crate::routes::upload::{UploadState, start_batch_cleanup};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub upload: Arc<UploadState>,
}

impl FromRef<AppState> for Arc<AppConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for Arc<UploadState> {
    fn from_ref(state: &AppState) -> Self {
        state.upload.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(AppConfig::load());
    let upload_state = Arc::new(UploadState::new());
    let app_state = AppState {
        config: config.clone(),
        upload: upload_state.clone(),
    };

    let _ = fs::create_dir_all(&config.upload_dir);
    let _ = fs::create_dir_all(config.upload_dir.join(".metadata"));
    start_batch_cleanup(config.clone(), upload_state.clone());

    generate_pwa_manifest(&config);

    let api_routes = Router::new()
        .nest("/auth", crate::routes::auth::router())
        .nest("/upload", crate::routes::upload::router())
        .nest("/files", crate::routes::files::router());

    let app = Router::new()
        .nest("/api", api_routes)
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/login.html", get(serve_login))
        .fallback_service(tower_http::services::ServeDir::new("frontend/dist"))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            hsts_middleware,
        ))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(Extension(config.clone()))
        .with_state(app_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("RustDrop server running on {}", config.base_url);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn serve_index(State(config): State<Arc<AppConfig>>) -> impl IntoResponse {
    serve_html(config, "index.html").await.into_response()
}

async fn serve_login() -> impl IntoResponse {
    Redirect::temporary("/")
}

async fn serve_html(config: Arc<AppConfig>, req_path: &str) -> Response {
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
        let headers = response.headers_mut();
        headers.insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
        headers.insert(
            axum::http::header::PRAGMA,
            axum::http::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            axum::http::header::EXPIRES,
            axum::http::HeaderValue::from_static("0"),
        );
    }
    response
}

fn generate_pwa_manifest(config: &AppConfig) {
    let mut manifest_assets = Vec::new();
    fn walk_dir(dir: &Path, base: &str, assets: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let full = entry.path();
                let rel = if base.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", base, name)
                };
                if full.is_dir() {
                    walk_dir(&full, &rel, assets);
                } else {
                    assets.push(format!("/{}", rel));
                }
            }
        }
    }
    walk_dir(Path::new("frontend/dist"), "", &mut manifest_assets);

    let asset_path = Path::new("frontend/dist/asset-manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&manifest_assets) {
        let _ = fs::write(asset_path, json);
    }

    let pwa_manifest = serde_json::json!({
        "name": &config.site_title,
        "short_name": &config.site_title,
        "description": "A simple file upload application",
        "start_url": "/",
        "display": "standalone",
        "background_color": "#ffffff",
        "theme_color": "#000000",
        "icons": [
            {
                "src": "/assets/icon.png",
                "type": "image/png",
                "sizes": "192x192"
            },
            {
                "src": "/assets/icon.png",
                "type": "image/png",
                "sizes": "512x512"
            }
        ],
        "orientation": "any"
    });

    let pwa_path = Path::new("frontend/dist/manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&pwa_manifest) {
        let _ = fs::write(pwa_path, json);
    }
}

async fn hsts_middleware(
    State(config): State<Arc<AppConfig>>,
    headers: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let is_secure = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or_else(|| config.base_url.starts_with("https"));

    let mut response = next.run(request).await;

    if is_secure {
        response.headers_mut().insert(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

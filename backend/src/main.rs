mod config;
mod routes;
mod security;
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
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;
use crate::routes::upload::{UploadState, start_batch_cleanup};
use crate::security::security_headers_middleware;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub upload: Arc<UploadState>,
    pub active_sessions: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    pub rate_limiter: Arc<
        tokio::sync::RwLock<std::collections::HashMap<std::net::IpAddr, Vec<std::time::Instant>>>,
    >,
}

impl AppState {
    pub async fn check_rate_limit(&self, ip: std::net::IpAddr) -> bool {
        let max_requests = 100;
        let window = std::time::Duration::from_secs(60);
        let now = std::time::Instant::now();

        let mut map = self.rate_limiter.write().await;
        let timestamps = map.entry(ip).or_insert_with(Vec::new);

        timestamps.retain(|&t| now.duration_since(t) < window);

        if timestamps.len() >= max_requests {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    pub async fn clean_old_rate_limits(&self) {
        let window = std::time::Duration::from_secs(60);
        let now = std::time::Instant::now();
        let mut map = self.rate_limiter.write().await;
        map.retain(|_, timestamps| {
            timestamps.retain(|&t| now.duration_since(t) < window);
            !timestamps.is_empty()
        });
    }
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
    let log_dir = std::env::var("LOG_DIR").ok().or_else(|| {
        let data_dir = std::path::Path::new("/app/data");
        if data_dir.is_dir() {
            Some("/app/data/log".to_string())
        } else {
            Some("/app/log".to_string())
        }
    });

    let (file_layer_error, file_layer_app) = if let Some(ref dir) = log_dir {
        if dir == "off" || dir == "none" || dir == "false" {
            (None, None)
        } else {
            let _ = std::fs::create_dir_all(dir);
            let error_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(std::path::Path::new(dir).join("error.log"))
                .ok();
            let app_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(std::path::Path::new(dir).join("app.log"))
                .ok();

            let error_layer = error_file.map(|file| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::WARN)
            });

            let app_layer = app_file.map(|file| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::INFO)
            });

            (error_layer, app_layer)
        }
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer_error)
        .with(file_layer_app)
        .init();

    let config = Arc::new(AppConfig::load());
    let upload_state = Arc::new(UploadState::new());
    let app_state = AppState {
        config: config.clone(),
        upload: upload_state.clone(),
        active_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        rate_limiter: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
    };

    let state_clone = app_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            state_clone.clean_old_rate_limits().await;
        }
    });

    let _ = fs::create_dir_all(&config.upload_dir);
    let _ = fs::create_dir_all(config.upload_dir.join(".metadata"));
    start_batch_cleanup(config.clone(), upload_state.clone());

    generate_pwa_manifest(&config);

    let api_routes = Router::new()
        .nest("/auth", crate::routes::auth::router())
        .nest("/upload", crate::routes::upload::router())
        .nest("/files", crate::routes::files::router())
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::routes::auth::rate_limit_middleware,
        ));

    let cors = if config.allowed_origins == "*" {
        tower_http::cors::CorsLayer::permissive()
    } else {
        let mut cors = tower_http::cors::CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::COOKIE,
                axum::http::header::HeaderName::from_static("x-pin"),
            ]);
        for origin in config.allowed_origins.split(',') {
            if let Ok(parsed) = origin.trim().parse::<axum::http::HeaderValue>() {
                cors = cors.allow_origin(parsed);
            }
        }
        cors.allow_credentials(true)
    };

    let app = Router::new()
        .nest("/api", api_routes)
        .route("/health", get(serve_health))
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/login.html", get(serve_login))
        .fallback_service(tower_http::services::ServeDir::new("frontend/dist"))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            hsts_middleware,
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(Extension(config.clone()))
        .with_state(app_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Beam server running on {}", config.base_url);
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
                "src": "favicon.svg",
                "type": "image/svg+xml",
                "sizes": "any"
            },
            {
                "src": "favicon.png",
                "type": "image/png",
                "sizes": "192x192"
            },
            {
                "src": "favicon.png",
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

async fn serve_health() -> impl IntoResponse {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": secs
    }))
}

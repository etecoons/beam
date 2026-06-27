mod config;
mod pwa;
mod routes;
mod security;
mod state;
mod static_files;
#[cfg(test)]
mod tests;
mod utils;

use axum::http::header::HeaderValue;
use axum::response::IntoResponse;
use axum::{
    Extension, Router,
    extract::State,
    routing::get,
};
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::config::AppConfig;
use crate::pwa::generate_pwa_manifest;
use crate::routes::upload::{UploadState, start_batch_cleanup};
use crate::security::security_headers_middleware;
use crate::state::AppState;
use crate::static_files::{serve_health, serve_html, serve_index, serve_login};

/// Server entry point.
#[tokio::main]
async fn main() {
    init_tracing();
    let config = Arc::new(AppConfig::load());
    let upload_state = Arc::new(UploadState::new());
    let app_state = AppState {
        config: config.clone(),
        upload: upload_state.clone(),
        active_sessions: Arc::new(Default::default()),
        rate_limiter: Arc::new(Default::default()),
    };

    spawn_rate_limit_cleaner(app_state.clone());
    bootstrap_directories(config.upload_dir.as_path());
    start_batch_cleanup(config.clone(), upload_state.clone());
    generate_pwa_manifest(&config);

    let app = build_router(config.clone(), app_state);
    run_server(config.server.port, app).await;
}

/// Install the global tracing subscriber (stdout + optional file logging).
fn init_tracing() {
    let log_dir = std::env::var("LOG_DIR").ok().or_else(|| {
        if Path::new("/app/data").is_dir() {
            Some("/app/data/log".to_string())
        } else {
            Some("/app/log".to_string())
        }
    });

    let (file_err, file_app) = match log_dir.as_deref() {
        Some(d) if !matches!(d, "off" | "none" | "false") => {
            let _ = std::fs::create_dir_all(d);
            let open = |name: &str| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(Path::new(d).join(name))
                    .ok()
            };
            let err = open("error.log").map(|f| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(f))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::WARN)
            });
            let app = open("app.log").map(|f| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(f))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::INFO)
            });
            (err, app)
        }
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(file_err)
        .with(file_app)
        .init();
}

/// Ensure upload dir + metadata subdir exist.
fn bootstrap_directories(upload_dir: &Path) {
    let _ = std::fs::create_dir_all(upload_dir);
    let _ = std::fs::create_dir_all(upload_dir.join(".metadata"));
}

/// Spawn the periodic rate-limit GC task.
fn spawn_rate_limit_cleaner(state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            state.clean_old_rate_limits().await;
        }
    });
}

/// Compose the full axum router with CORS / HSTS / security middleware.
fn build_router(config: Arc<AppConfig>, app_state: AppState) -> Router {
    let api_routes = Router::new()
        .nest("/auth", crate::routes::auth::router())
        .nest("/upload", crate::routes::upload::router())
        .nest("/files", crate::routes::files::router())
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::routes::auth::rate_limit_middleware,
        ));

    let cors = build_cors(&config.server.allowed_origins);

    Router::new()
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
        .with_state(app_state)
}

/// CORS layer built from `ALLOWED_ORIGINS`. `"*"` = permissive.
fn build_cors(allowed_origins: &str) -> tower_http::cors::CorsLayer {
    use axum::http::{header, Method};
    use tower_http::cors::CorsLayer;
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
    ];
    let headers = [
        header::CONTENT_TYPE,
        header::COOKIE,
        header::HeaderName::from_static("x-pin"),
    ];

    if allowed_origins.trim() == "*" {
        return CorsLayer::permissive();
    }

    let mut layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers)
        .allow_credentials(true);
    for origin in allowed_origins.split(',') {
        if let Ok(parsed) = header::HeaderValue::from_str(origin.trim()) {
            layer = layer.allow_origin(parsed);
        }
    }
    layer
}

/// Bind and serve with graceful shutdown on SIGINT/SIGTERM.
async fn run_server(port: u16, app: Router) {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(target: "bootstrap", "listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let svc = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    axum::serve(listener, svc).await.unwrap();
}

/// HSTS middleware: emit Strict-Transport-Security when the connection is HTTPS.
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
        .unwrap_or_else(|| config.server.base_url.starts_with("https"));

    let mut response = next.run(request).await;
    if is_secure {
        response.headers_mut().insert(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}
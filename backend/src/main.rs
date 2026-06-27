mod config;
mod pwa;
mod routes;
mod security;
mod state;
mod static_files;
#[cfg(test)]
mod tests;
mod utils;

use axum::{Extension, Router, routing::get};
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::config::AppConfig;
use crate::pwa::generate_pwa_manifest;
use crate::routes::upload::{UploadState, start_batch_cleanup};
use crate::state::AppState;
use crate::static_files::{serve_health, serve_index, serve_login};

use shared_assets::middleware::hsts::{HstsState, hsts_layer};
use shared_assets::middleware::title::{TitleState, title_injection_layer};
use shared_assets::middleware::{cors_layer, security_headers_layer};

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
///
/// All four security layers come from `shared-assets` so that beam stays
/// aligned with the rest of the portfolio:
/// - [`cors_layer`] — CORS allowlist, refuses credentials with `*`
/// - [`security_headers_layer`] — X-Frame-Options, CSP, X-Content-Type-Options
/// - [`hsts_layer`] — Strict-Transport-Security on HTTPS
/// - [`title_injection_layer`] — replaces `{{SITE_TITLE}}` in HTML responses
fn build_router(config: Arc<AppConfig>, app_state: AppState) -> Router {
    // The shared-assets middleware needs a `&ServerConfig` / `Arc<ServerConfig>`.
    // Extract it from the app config so the rest of the file can keep using
    // `Arc<AppConfig>`.
    let server_config: Arc<shared_assets::server::ServerConfig> = Arc::new(config.server.clone());

    let api_routes = Router::new()
        .nest("/auth", crate::routes::auth::router())
        .nest("/upload", crate::routes::upload::router())
        .nest("/files", crate::routes::files::router())
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::routes::auth::rate_limit_middleware,
        ));

    let cors = cors_layer(&config.server);

    Router::new()
        .nest("/api", api_routes)
        .route("/health", get(serve_health))
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/login.html", get(serve_login))
        .fallback_service(tower_http::services::ServeDir::new("frontend/dist"))
        .layer(axum::middleware::from_fn(security_headers_layer))
        .layer(axum::middleware::from_fn_with_state(
            TitleState(server_config.clone()),
            title_injection_layer,
        ))
        .layer(axum::middleware::from_fn_with_state(
            HstsState(server_config.clone()),
            hsts_layer,
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors)
        .layer(Extension(config.clone()))
        .with_state(app_state)
}

/// Bind and serve with graceful shutdown on SIGINT/SIGTERM.
async fn run_server(port: u16, app: Router) {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(target: "bootstrap", "listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let svc = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    axum::serve(listener, svc).await.unwrap();
}

// `serve_html` lives in `static_files` and is used by `serve_index`. The
// shared-assets `title_injection_layer` middleware (installed in
// `build_router` above) handles the simpler `{{SITE_TITLE}}` case for
// any HTML served by the `ServeDir` fallback (which would otherwise be
// missed by `serve_index`).

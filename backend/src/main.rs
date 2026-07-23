
mod ip;
mod config;
pub mod middleware;
mod routes;
pub mod services;
mod state;
#[cfg(test)]
mod tests;
mod utils;

use axum::{Extension, Router, routing::get};
use std::path::Path;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::middleware::static_files::{serve_health, serve_index, serve_login};
pub use crate::middleware::{security, static_files};
use crate::routes::upload::{UploadState, start_batch_cleanup};
pub use crate::services::pwa;
use crate::services::pwa::generate_pwa_manifest;
use crate::state::AppState;
use shared_backend::middleware::hsts::{HstsState, hsts_layer};

use shared_backend::middleware::title::{TitleState, title_injection_layer};

use shared_backend::middleware::{cors_layer, security_headers_layer};
mod cookie_auth;
mod session_id;

/// Server entry point.
#[tokio::main]
async fn main() {
    shared_backend::tracing_init::init_tracing(
        shared_backend::tracing_init::default_log_dir().as_deref(),
    );
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
    if let Err(e) = run_server(config.port, app).await {
        tracing::error!("Server error: {e}");
    }
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
    let server_config: Arc<crate::config::AppConfig> = Arc::new(config.clone());

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
async fn run_server(port: u16, app: Router) -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(target: "bootstrap", "listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let svc = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    axum::serve(listener, svc).await?;
    Ok(())
}

// `serve_html` lives in `static_files` and is used by `serve_index`. The
// shared-assets `title_injection_layer` middleware (installed in
// `build_router` above) handles the simpler `{{SITE_TITLE}}` case for
// any HTML served by the `ServeDir` fallback (which would otherwise be
// missed by `serve_index`).

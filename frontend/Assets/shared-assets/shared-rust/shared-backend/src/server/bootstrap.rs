//! Server bootstrap: tracing + bind + serve + graceful shutdown.
//!
//! Apps compose their own `axum::Router` and hand it to [`serve`] which
//! handles the common boilerplate: bind to `config.port`, install the
//! tracing subscriber, and run with `ConnectInfo<SocketAddr>` so handlers
//! can extract the client IP.

use super::ServerConfig;
use axum::Router;
use std::net::SocketAddr;
use std::time::Duration;

/// Bind to `config.port` and serve the provided router until shutdown.
///
/// Installs `tower-http` tracing and runs the server with
/// `into_make_service_with_connect_info::<SocketAddr>()` so handlers can
/// extract the connecting socket address for client-IP resolution.
pub async fn serve(config: ServerConfig, app: Router) -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(target: "bootstrap", "listening on {addr}");
    tracing::info!(target: "bootstrap", "site_title={}", config.site_title);
    tracing::info!(target: "bootstrap", "base_url={}", config.base_url);
    tracing::info!(target: "bootstrap", "pin_enabled={}", config.pin_enabled());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let drain = Duration::from_secs(config.shutdown_drain_seconds);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(drain))
    .await
}

/// Wait for SIGINT or SIGTERM (whichever comes first), then drain for
/// `drain` seconds before exiting.
async fn shutdown_signal(drain: Duration) {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => tracing::info!("received SIGINT"),
        _ = sigterm.recv() => tracing::info!("received SIGTERM"),
    }

    tracing::info!(target: "bootstrap", "draining connections ({}s)", drain.as_secs());
    tokio::time::sleep(drain).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_signal_compiles() {
        // Smoke test: just verify the function exists and has the expected
        // signature. We can't easily test the actual SIGINT handling in unit
        // tests.
        fn _exists() {
            let _: fn(Duration) -> _ = shutdown_signal;
        }
    }
}

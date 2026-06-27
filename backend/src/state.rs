//! Application state shared across all handlers.
//!
//! Held in an `Arc` inside axum's `State` extractor.

use axum::extract::FromRef;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::routes::upload::UploadState;

/// Per-request application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub upload: Arc<UploadState>,
    pub active_sessions: Arc<RwLock<HashSet<String>>>,
    pub rate_limiter: Arc<RwLock<HashMap<IpAddr, Vec<Instant>>>>,
}

impl AppState {
    /// Returns true if the request should be allowed (not over the rate limit).
    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        const MAX_REQUESTS: usize = 100;
        const WINDOW: Duration = Duration::from_secs(60);
        let now = Instant::now();

        let mut map = self.rate_limiter.write().await;
        let timestamps = map.entry(ip).or_insert_with(Vec::new);
        timestamps.retain(|&t| now.duration_since(t) < WINDOW);

        if timestamps.len() >= MAX_REQUESTS {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    /// Periodic GC that drops rate-limit entries outside the window.
    pub async fn clean_old_rate_limits(&self) {
        const WINDOW: Duration = Duration::from_secs(60);
        let now = Instant::now();
        let mut map = self.rate_limiter.write().await;
        map.retain(|_, timestamps| {
            timestamps.retain(|&t| now.duration_since(t) < WINDOW);
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
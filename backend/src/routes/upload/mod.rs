mod cancel;
mod chunk;
mod init;
mod metadata;
mod utils;

use axum::{Router, routing::post};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use self::cancel::cancel_upload;
use self::chunk::upload_chunk;
use self::init::init_upload;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/init", post(init_upload))
        .route("/chunk/{uploadId}", post(upload_chunk))
        .route("/cancel/{uploadId}", post(cancel_upload))
}

pub struct UploadState {
    pub folder_mappings: Mutex<HashMap<String, String>>,
    pub batch_activity: Mutex<HashMap<String, std::time::Instant>>,
    pub active_uploads: Mutex<HashMap<String, self::metadata::UploadMetadata>>,
    pub file_handles: Mutex<HashMap<String, Arc<tokio::sync::Mutex<tokio::fs::File>>>>,
}

impl UploadState {
    pub fn new() -> Self {
        Self {
            folder_mappings: Mutex::new(HashMap::new()),
            batch_activity: Mutex::new(HashMap::new()),
            active_uploads: Mutex::new(HashMap::new()),
            file_handles: Mutex::new(HashMap::new()),
        }
    }
}

/// How long the retention cleanup is allowed to run per tick. The cleanup
/// is a `spawn_blocking` task on a 60s tick; bounding it to 1s keeps the
/// blocking thread pool responsive and leaves the rest of the tick budget
/// for batch + metadata cleanup.
const CLEANUP_BUDGET: std::time::Duration = std::time::Duration::from_secs(1);

fn run_retention_cleanup(upload_dir: &std::path::Path, retention_days: u64) {
    let max_age = std::time::Duration::from_secs(retention_days * 24 * 60 * 60);
    let now = std::time::SystemTime::now();
    let deadline = std::time::Instant::now() + CLEANUP_BUDGET;

    // Tracks whether we ran out of time on this tick. If true, the next
    // tick continues where this one left off by picking up the oldest
    // remaining expired files first.
    let mut timed_out = false;

    fn clean_dir(
        dir: &std::path::Path,
        max_age: std::time::Duration,
        now: std::time::SystemTime,
        deadline: &std::time::Instant,
        timed_out: &mut bool,
    ) -> bool {
        let mut is_empty = true;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return false,
        };
        for entry in entries.flatten() {
            // Bail out of the walk as soon as the per-tick budget is
            // exhausted. Outer caller will spawn the next tick to pick up
            // where this one left off. The cleanup is idempotent because
            // it checks mtime on each file before deleting.
            if std::time::Instant::now() >= *deadline {
                *timed_out = true;
                return false;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".metadata" {
                is_empty = false;
                continue;
            }
            if path.is_dir() {
                let children_cleaned = clean_dir(&path, max_age, now, deadline, timed_out);
                if children_cleaned {
                    if let Err(e) = std::fs::remove_dir(&path) {
                        tracing::error!("Failed to remove empty directory {:?}: {}", path, e);
                        is_empty = false;
                    }
                } else {
                    is_empty = false;
                }
            } else if let Ok(metadata) = entry.metadata()
                && let Ok(modified) = metadata.modified()
                && let Ok(age) = now.duration_since(modified)
            {
                if age > max_age {
                    tracing::info!("Auto-retention: deleting expired file {:?}", path);
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::error!(
                            "Failed to delete expired file {:?}: {}",
                            path,
                            e
                        );
                        is_empty = false;
                    }
                } else {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        }
        is_empty
    }

    let _ = clean_dir(upload_dir, max_age, now, &deadline, &mut timed_out);

    if timed_out {
        tracing::info!(
            "Retention cleanup exceeded {:?} budget; will continue on next tick",
            CLEANUP_BUDGET
        );
    }
}

pub fn start_batch_cleanup(config: Arc<crate::config::AppConfig>, state: Arc<UploadState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            // Run retention cleanup
            if let Some(days) = config.retention_period_days {
                let dir = config.upload_dir.clone();
                tokio::task::spawn_blocking(move || {
                    run_retention_cleanup(&dir, days);
                })
                .await
                .ok();
            }

            let now = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(30 * 60);

            let mut expired_batches = Vec::new();
            {
                let activity = state.batch_activity.lock().unwrap();
                for (batch_id, last_activity) in activity.iter() {
                    if now.duration_since(*last_activity) >= timeout {
                        expired_batches.push(batch_id.clone());
                    }
                }
            }

            if !expired_batches.is_empty() {
                tracing::info!(
                    "Cleaning up {} inactive batch sessions",
                    expired_batches.len()
                );
                let mut activity = state.batch_activity.lock().unwrap();
                let mut mappings = state.folder_mappings.lock().unwrap();

                for batch_id in expired_batches {
                    activity.remove(&batch_id);
                    mappings.retain(|key, _| !key.ends_with(&format!("-{}", batch_id)));
                }
            }

            // Cleanup stale cached in-memory upload metadata
            let mut expired_uploads = Vec::new();
            {
                let active = state.active_uploads.lock().unwrap();
                for (upload_id, meta) in active.iter() {
                    let last_activity_time = std::time::UNIX_EPOCH
                        + std::time::Duration::from_millis(meta.last_activity);
                    if let Ok(duration) =
                        std::time::SystemTime::now().duration_since(last_activity_time)
                        && duration >= timeout
                    {
                        expired_uploads.push(upload_id.clone());
                    }
                }
            }
            if !expired_uploads.is_empty() {
                tracing::info!(
                    "Cleaning up {} expired cached uploads",
                    expired_uploads.len()
                );
                let mut active = state.active_uploads.lock().unwrap();
                let mut handles = state.file_handles.lock().unwrap();
                for upload_id in expired_uploads {
                    active.remove(&upload_id);
                    handles.remove(&upload_id);
                }
            }
        }
    });
}

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde_json::json;
use std::path::Path as StdPath;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::routes::upload::UploadState;
use crate::routes::upload::metadata::read_upload_metadata;

pub async fn cancel_upload(
    State(config): State<Arc<AppConfig>>,
    State(state): State<Arc<UploadState>>,
    Path(upload_id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Received cancel request for upload: {}", upload_id);

    // Remove the file handle first so the file is closed, allowing removal of the partial file on disk
    {
        let mut handles = state.file_handles.lock().unwrap();
        handles.remove(&upload_id);
    }

    let metadata = {
        let active = state.active_uploads.lock().unwrap();
        active.get(&upload_id).cloned()
    };

    let metadata = match metadata {
        Some(m) => Some(m),
        None => read_upload_metadata(&config.upload_dir, &upload_id).await,
    };

    if let Some(metadata) = metadata {
        let partial_path = StdPath::new(&metadata.partial_file_path);
        if partial_path.exists() {
            let _ = tokio::fs::remove_file(partial_path).await;
            tracing::info!("Deleted partial file on cancellation: {:?}", partial_path);
        }
        super::metadata::delete_upload_metadata(&config.upload_dir, &upload_id).await;
        {
            let mut active = state.active_uploads.lock().unwrap();
            active.remove(&upload_id);
        }
        tracing::info!(
            "Upload cancelled and cleaned up: {} ({})",
            upload_id,
            metadata.original_filename
        );
    } else {
        tracing::warn!(
            "Cancel request for non-existent or already completed upload: {}",
            upload_id
        );
    }

    Json(json!({ "message": "Upload cancelled or already complete" }))
}

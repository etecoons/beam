use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;
use crate::routes::upload::UploadState;
use crate::routes::upload::metadata::{
    UploadMetadata, delete_upload_metadata, write_upload_metadata,
};

#[derive(Deserialize)]
pub struct InitUploadPayload {
    pub filename: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
}

#[derive(Serialize)]
pub struct InitUploadResponse {
    #[serde(rename = "uploadId")]
    pub upload_id: String,
}

pub async fn init_upload(
    State(config): State<Arc<AppConfig>>,
    State(state): State<Arc<UploadState>>,
    _auth: RequirePin,
    headers: HeaderMap,
    Json(payload): Json<InitUploadPayload>,
) -> Response {
    if let Err((status, err_json)) =
        super::utils::validate_upload(&config, &payload.filename, payload.file_size)
    {
        return (status, Json(err_json)).into_response();
    }

    let size = payload.file_size;
    let client_batch_id = headers.get("x-batch-id").and_then(|h| h.to_str().ok());

    let batch_id = match client_batch_id {
        Some(bid) => {
            if !crate::utils::is_valid_batch_id(bid) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "Invalid batch ID format" })),
                )
                    .into_response();
            }
            bid.to_string()
        }
        None => super::utils::generate_batch_id(),
    };

    state
        .batch_activity
        .lock()
        .unwrap()
        .insert(batch_id.clone(), std::time::Instant::now());

    let sanitized = crate::utils::sanitize_path_preserve_dirs_safe(&payload.filename);
    let safe_filename = crate::utils::normalize_path(StdPath::new(&sanitized))
        .to_string_lossy()
        .replace('\\', "/");

    if let Err((status, err_json)) = super::utils::validate_extension(&config, &safe_filename) {
        return (status, Json(err_json)).into_response();
    }

    let upload_id = format!("{:x}", rand::random::<u128>());
    let mut final_file_path = config.upload_dir.join(&safe_filename);
    if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
        tracing::error!(
            "Path traversal detected in upload init: {} -> {:?}",
            safe_filename,
            final_file_path
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid file path" })),
        )
            .into_response();
    }

    let path_parts: Vec<&str> = safe_filename.split('/').filter(|s| !s.is_empty()).collect();
    if path_parts.len() > 1 {
        final_file_path =
            super::utils::get_remapped_folder_path(&config, &state, &path_parts, &batch_id);
        if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Invalid file path" })),
            )
                .into_response();
        }
    } else {
        let _ = fs::create_dir_all(&config.upload_dir);
    }

    final_file_path = super::utils::get_unique_filename(&final_file_path, &config.upload_dir);

    if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid file path" })),
        )
            .into_response();
    }

    if let Some(parent) = final_file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let partial_file_path = format!("{}.partial", final_file_path.to_string_lossy());
    if !crate::utils::is_path_within_upload_dir(
        StdPath::new(&partial_file_path),
        &config.upload_dir,
        false,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid file path" })),
        )
            .into_response();
    }

    let metadata = UploadMetadata {
        upload_id: upload_id.clone(),
        original_filename: safe_filename,
        file_path: final_file_path.to_string_lossy().to_string(),
        partial_file_path: partial_file_path.clone(),
        file_size: size,
        bytes_received: 0,
        batch_id,
        created_at: chrono::Utc::now().timestamp_millis() as u64,
        last_activity: chrono::Utc::now().timestamp_millis() as u64,
    };

    if let Err(e) = write_upload_metadata(&config.upload_dir, &upload_id, metadata.clone()).await {
        tracing::error!("Failed to write metadata: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to initialize upload" })),
        )
            .into_response();
    }

    {
        let mut active = state.active_uploads.lock().unwrap();
        active.insert(upload_id.clone(), metadata.clone());
    }

    tracing::info!(
        "Initialized persistent upload: {} for {} -> {:?}",
        upload_id,
        payload.filename,
        final_file_path
    );

    if size == 0 {
        if let Err(e) = fs::write(&final_file_path, "") {
            tracing::error!(
                "Failed to create zero-byte file {:?}: {}",
                final_file_path,
                e
            );
            delete_upload_metadata(&config.upload_dir, &upload_id).await;
            {
                let mut active = state.active_uploads.lock().unwrap();
                active.remove(&upload_id);
            }
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to complete zero-byte upload" })),
            )
                .into_response();
        }

        tracing::info!(
            "Completed zero-byte file upload: {} as {:?}",
            payload.filename,
            final_file_path
        );
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        {
            let mut active = state.active_uploads.lock().unwrap();
            active.remove(&upload_id);
        }
    }

    (StatusCode::OK, Json(InitUploadResponse { upload_id })).into_response()
}

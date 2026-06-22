use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::path::Path as StdPath;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;
use crate::routes::upload::UploadState;
use crate::routes::upload::metadata::{delete_upload_metadata, read_upload_metadata};

pub async fn upload_chunk(
    State(config): State<Arc<AppConfig>>,
    State(state): State<Arc<UploadState>>,
    _auth: RequirePin,
    Path(upload_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let chunk_size = body.len() as u64;
    if chunk_size == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Empty chunk received" })),
        )
            .into_response();
    }

    let cached_metadata = {
        let active = state.active_uploads.lock().unwrap();
        active.get(&upload_id).cloned()
    };

    let mut metadata = match cached_metadata {
        Some(m) => m,
        None => match read_upload_metadata(&config.upload_dir, &upload_id).await {
            Some(m) => {
                let mut active = state.active_uploads.lock().unwrap();
                active.insert(upload_id.clone(), m.clone());
                m
            }
            None => {
                let client_batch_id = headers
                    .get("x-batch-id")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("none");
                tracing::warn!(
                    "Upload metadata not found for chunk request: {}. Client Batch ID: {}.",
                    upload_id,
                    client_batch_id
                );
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Upload session not found or already completed" })),
                )
                    .into_response();
            }
        },
    };

    if !metadata.batch_id.is_empty() && crate::utils::is_valid_batch_id(&metadata.batch_id) {
        state
            .batch_activity
            .lock()
            .unwrap()
            .insert(metadata.batch_id.clone(), std::time::Instant::now());
    }

    if metadata.bytes_received >= metadata.file_size {
        let partial_path = StdPath::new(&metadata.partial_file_path);
        let final_path = StdPath::new(&metadata.file_path);
        if !final_path.exists() && partial_path.exists() {
            let _ = tokio::fs::rename(partial_path, final_path).await;
        }
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        state.active_uploads.lock().unwrap().remove(&upload_id);
        state.file_handles.lock().unwrap().remove(&upload_id);
        return Json(json!({ "bytesReceived": metadata.file_size, "progress": 100 }))
            .into_response();
    }

    let mut write_size = chunk_size;
    let mut chunk_bytes = body;
    if metadata.bytes_received + chunk_size > metadata.file_size {
        tracing::warn!(
            "Chunk for {} exceeds expected file size. Expecting {}, got {}. Truncating.",
            upload_id,
            metadata.file_size,
            metadata.bytes_received + chunk_size
        );
        let bytes_to_write = metadata.file_size.saturating_sub(metadata.bytes_received);
        write_size = bytes_to_write;
        if write_size > 0 {
            chunk_bytes = chunk_bytes.slice(0..(write_size as usize));
        } else {
            metadata.bytes_received = metadata.file_size;
        }
    }

    if write_size > 0 {
        let partial_path = StdPath::new(&metadata.partial_file_path);

        let existing_arc = {
            let handles = state.file_handles.lock().unwrap();
            handles.get(&upload_id).cloned()
        };

        let file_arc = match existing_arc {
            Some(h) => h,
            None => {
                let file = match tokio::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(partial_path)
                    .await
                {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::error!("Failed to open partial file {:?}: {}", partial_path, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "Failed to open partial file" })),
                        )
                            .into_response();
                    }
                };
                let arc = Arc::new(tokio::sync::Mutex::new(file));
                let mut handles = state.file_handles.lock().unwrap();
                handles.entry(upload_id.clone()).or_insert(arc).clone()
            }
        };

        let mut file_guard = file_arc.lock().await;
        if let Err(e) = file_guard.write_all(&chunk_bytes).await {
            tracing::error!(
                "Failed to write chunk to partial file {:?}: {}",
                partial_path,
                e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to write chunk" })),
            )
                .into_response();
        }
        let _ = file_guard.flush().await;

        metadata.bytes_received += write_size;
    }

    let progress = if metadata.file_size == 0 {
        100
    } else {
        std::cmp::min(
            (metadata.bytes_received as f64 / metadata.file_size as f64 * 100.0).round() as u64,
            100,
        )
    };

    tracing::debug!(
        "Chunk written for {}: {}/{} ({}%)",
        upload_id,
        metadata.bytes_received,
        metadata.file_size,
        progress
    );

    metadata.last_activity = chrono::Utc::now().timestamp_millis() as u64;
    {
        let mut active = state.active_uploads.lock().unwrap();
        active.insert(upload_id.clone(), metadata.clone());
    }

    if metadata.bytes_received >= metadata.file_size {
        // Remove and drop the file handle from state so it closes the file!
        {
            let mut handles = state.file_handles.lock().unwrap();
            handles.remove(&upload_id);
        }

        tracing::info!(
            "Upload {} ({}) completed {} bytes.",
            upload_id,
            metadata.original_filename,
            metadata.bytes_received
        );
        let partial_path = StdPath::new(&metadata.partial_file_path);
        let final_path = StdPath::new(&metadata.file_path);

        match tokio::fs::rename(partial_path, final_path).await {
            Ok(_) => {
                tracing::info!(
                    "Upload completed and finalized: {} as {:?}",
                    metadata.original_filename,
                    final_path
                );
                delete_upload_metadata(&config.upload_dir, &upload_id).await;
                {
                    let mut active = state.active_uploads.lock().unwrap();
                    active.remove(&upload_id);
                }

                let config_clone = config.clone();
                let filename_clone = metadata.original_filename.clone();
                let filesize_clone = metadata.file_size;
                tokio::spawn(async move {
                    crate::services::send_notification(
                        &filename_clone,
                        filesize_clone,
                        &config_clone,
                    )
                    .await;
                });
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        "Partial file {:?} missing during finalization, assuming completed elsewhere.",
                        partial_path
                    );
                    delete_upload_metadata(&config.upload_dir, &upload_id).await;
                    {
                        let mut active = state.active_uploads.lock().unwrap();
                        active.remove(&upload_id);
                    }
                } else {
                    tracing::error!(
                        "CRITICAL: Failed to rename partial file {:?} to {:?}: {}",
                        partial_path,
                        final_path,
                        e
                    );
                }
            }
        }
    }

    Json(json!({ "bytesReceived": metadata.bytes_received, "progress": progress })).into_response()
}

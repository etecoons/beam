use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::fs;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;

pub async fn delete_file(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
) -> impl IntoResponse {
    if !config.show_file_list {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "File deletion is disabled" })),
        )
            .into_response();
    }

    let decoded_path = percent_encoding::percent_decode_str(&path)
        .decode_utf8_lossy()
        .to_string();
    let file_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&file_path, &config.upload_dir, false) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Access denied" })),
        )
            .into_response();
    }

    let metadata = match fs::metadata(&file_path) {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "File or directory not found" })),
            )
                .into_response();
        }
    };

    if metadata.is_dir() {
        if let Err(e) = fs::remove_dir_all(&file_path) {
            tracing::error!("Failed to delete directory: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to delete directory" })),
            )
                .into_response();
        }
        tracing::info!("Directory deleted: {}", decoded_path);
        Json(json!({ "message": "Directory deleted successfully" })).into_response()
    } else {
        if let Err(e) = fs::remove_file(&file_path) {
            tracing::error!("Failed to delete file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to delete file" })),
            )
                .into_response();
        }
        tracing::info!("File deleted: {}", decoded_path);
        Json(json!({ "message": "File deleted successfully" })).into_response()
    }
}

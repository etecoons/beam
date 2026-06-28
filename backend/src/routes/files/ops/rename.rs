use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;

#[derive(Deserialize)]
pub struct RenamePayload {
    #[serde(rename = "newName")]
    pub new_name: String,
}

pub async fn rename_file(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
    Json(payload): Json<RenamePayload>,
) -> impl IntoResponse {
    if !config.show_file_list {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "File renaming is disabled" })),
        )
            .into_response();
    }

    if payload.new_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "New name is required" })),
        )
            .into_response();
    }

    let decoded_path = percent_encoding::percent_decode_str(&path)
        .decode_utf8_lossy()
        .to_string();
    let current_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&current_path, &config.upload_dir, false) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Access denied" })),
        )
            .into_response();
    }

    let metadata = match fs::metadata(&current_path) {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "File or directory not found" })),
            )
                .into_response();
        }
    };

    let sanitized_new_name = crate::utils::sanitize_filename_safe(payload.new_name.trim());
    if sanitized_new_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid or empty filename after sanitization" })),
        )
            .into_response();
    }

    let current_dir = current_path.parent().unwrap_or(&config.upload_dir);
    let new_path = current_dir.join(&sanitized_new_name);

    if !crate::utils::is_path_within_upload_dir(&new_path, &config.upload_dir, false) {
        tracing::warn!(
            "rename: rejected new path outside upload dir: {:?} -> {:?}",
            decoded_path,
            new_path
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid destination path" })),
        )
            .into_response();
    }

    if !crate::utils::is_path_within_upload_dir(&current_path, &config.upload_dir, true) {
        tracing::warn!(
            "rename: source path failed re-validation: {:?}",
            current_path
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid source path" })),
        )
            .into_response();
    }

    if new_path.exists() {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "A file or directory with that name already exists" })),
        )
            .into_response();
    }

    if let Err(e) = fs::rename(&current_path, &new_path) {
        tracing::error!("Rename failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to rename item" })),
        )
            .into_response();
    }

    let item_type = if metadata.is_dir() {
        "Directory"
    } else {
        "File"
    };
    tracing::info!(
        "{} renamed: \"{}\" -> \"{}\"",
        item_type,
        decoded_path,
        sanitized_new_name
    );

    let relative_new_path = match new_path.strip_prefix(&config.upload_dir) {
        Ok(p) => p.to_string_lossy().replace('\\', "/"),
        Err(_) => sanitized_new_name.clone(),
    };

    let old_basename = StdPath::new(&decoded_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&decoded_path);

    Json(json!({
        "message": format!("{} renamed successfully", item_type),
        "oldName": old_basename,
        "newName": sanitized_new_name,
        "newPath": relative_new_path
    }))
    .into_response()
}

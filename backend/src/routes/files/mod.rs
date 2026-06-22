pub mod helpers;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put, delete},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;
use self::helpers::{
    get_directory_contents, calculate_total_size, count_files, create_safe_content_disposition, get_content_type
};

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/info/*path", get(file_info))
        .route("/download/*path", get(download_file))
        .route("/delete/*path", delete(delete_file))
        .route("/rename/*path", put(rename_file))
}

async fn list_files(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
) -> impl IntoResponse {
    match get_directory_contents(&config.upload_dir, "") {
        Ok(items) => {
            let total_size = calculate_total_size(&items);
            let total_files = count_files(&items);
            let response = json!({
                "items": items,
                "totalFiles": total_files,
                "totalSize": total_size,
                "formattedTotalSize": crate::utils::format_file_size(total_size, None)
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list files: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to list files" }))).into_response()
        }
    }
}

async fn file_info(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let decoded_path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy().to_string();
    let file_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&file_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Access denied" }))).into_response();
    }

    match fs::metadata(&file_path) {
        Ok(metadata) => {
            let file_info = json!({
                "filename": decoded_path,
                "size": metadata.len(),
                "formattedSize": crate::utils::format_file_size(metadata.len(), None),
                "uploadDate": DateTime::<Utc>::from(metadata.modified().unwrap_or(std::time::SystemTime::now())),
                "mimetype": StdPath::new(&decoded_path).extension().and_then(|e| e.to_str()).unwrap_or_default(),
                "type": if metadata.is_dir() { "directory" } else { "file" }
            });
            (StatusCode::OK, Json(file_info)).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" }))).into_response()
    }
}

async fn download_file(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let decoded_path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy().to_string();
    let file_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&file_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Access denied" }))).into_response();
    }

    let file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" }))).into_response(),
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_disposition = create_safe_content_disposition(&decoded_path);
    let content_type = get_content_type(&decoded_path);

    Response::builder()
        .header(axum::http::header::CONTENT_DISPOSITION, content_disposition)
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn delete_file(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let decoded_path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy().to_string();
    let file_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&file_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Access denied" }))).into_response();
    }

    let metadata = match fs::metadata(&file_path) {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, Json(json!({ "error": "File or directory not found" }))).into_response(),
    };

    if metadata.is_dir() {
        if let Err(e) = fs::remove_dir_all(&file_path) {
            tracing::error!("Failed to delete directory: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to delete directory" }))).into_response();
        }
        tracing::info!("Directory deleted: {}", decoded_path);
        Json(json!({ "message": "Directory deleted successfully" })).into_response()
    } else {
        if let Err(e) = fs::remove_file(&file_path) {
            tracing::error!("Failed to delete file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to delete file" }))).into_response();
        }
        tracing::info!("File deleted: {}", decoded_path);
        Json(json!({ "message": "File deleted successfully" })).into_response()
    }
}

#[derive(Deserialize)]
struct RenamePayload {
    #[serde(rename = "newName")]
    new_name: String,
}

async fn rename_file(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
    Json(payload): Json<RenamePayload>,
) -> impl IntoResponse {
    if payload.new_name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "New name is required" }))).into_response();
    }

    let decoded_path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy().to_string();
    let current_path = config.upload_dir.join(&decoded_path);

    if !crate::utils::is_path_within_upload_dir(&current_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Access denied" }))).into_response();
    }

    let metadata = match fs::metadata(&current_path) {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, Json(json!({ "error": "File or directory not found" }))).into_response(),
    };

    let sanitized_new_name = crate::utils::sanitize_filename_safe(payload.new_name.trim());
    if sanitized_new_name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid or empty filename after sanitization" }))).into_response();
    }

    let current_dir = current_path.parent().unwrap_or(&config.upload_dir);
    let new_path = current_dir.join(&sanitized_new_name);

    if !crate::utils::is_path_within_upload_dir(&new_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid destination path" }))).into_response();
    }

    if new_path.exists() {
        return (StatusCode::CONFLICT, Json(json!({ "error": "A file or directory with that name already exists" }))).into_response();
    }

    if let Err(e) = fs::rename(&current_path, &new_path) {
        tracing::error!("Rename failed: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to rename item" }))).into_response();
    }

    let item_type = if metadata.is_dir() { "Directory" } else { "File" };
    tracing::info!("{} renamed: \"{}\" -> \"{}\"", item_type, decoded_path, sanitized_new_name);

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
    })).into_response()
}

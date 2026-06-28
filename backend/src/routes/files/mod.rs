pub mod helpers;
pub mod ops;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;

use self::helpers::{calculate_total_size, count_files, get_directory_contents};
use crate::config::AppConfig;
use crate::routes::auth::RequirePin;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/info/{*path}", get(file_info))
        .route("/download/{*path}", get(ops::download_file))
        .route("/delete/{*path}", delete(ops::delete_file))
        .route("/rename/{*path}", put(ops::rename_file))
}

async fn list_files(State(config): State<Arc<AppConfig>>, _auth: RequirePin) -> impl IntoResponse {
    if !config.show_file_list {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "File explorer listing is disabled" })),
        )
            .into_response();
    }

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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to list files" })),
            )
                .into_response()
        }
    }
}

async fn file_info(
    State(config): State<Arc<AppConfig>>,
    _auth: RequirePin,
    Path(path): Path<String>,
) -> impl IntoResponse {
    if !config.show_file_list {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "File explorer is disabled" })),
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
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "File not found" })),
        )
            .into_response(),
    }
}

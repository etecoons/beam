use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put, delete},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/info/*path", get(file_info))
        .route("/download/*path", get(download_file))
        .route("/delete/*path", delete(delete_file))
        .route("/rename/*path", put(rename_file))
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FileItem {
    File {
        name: String,
        path: String,
        size: u64,
        #[serde(rename = "formattedSize")]
        formatted_size: String,
        #[serde(rename = "uploadDate")]
        upload_date: DateTime<Utc>,
        extension: String,
    },
    Directory {
        name: String,
        path: String,
        size: u64,
        #[serde(rename = "formattedSize")]
        formatted_size: String,
        #[serde(rename = "uploadDate")]
        upload_date: DateTime<Utc>,
        children: Vec<FileItem>,
    },
}

fn get_directory_contents(dir_path: &StdPath, relative_path: &str) -> std::io::Result<Vec<FileItem>> {
    let mut items = Vec::new();
    
    if !dir_path.exists() {
        return Ok(items);
    }
    
    let entries = fs::read_dir(dir_path)?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        
        if name == ".metadata" || name.starts_with('.') {
            continue;
        }
        
        let full_path = entry.path();
        let item_relative_path = if relative_path.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", relative_path, name)
        };
        
        let metadata = entry.metadata()?;
        let upload_date: DateTime<Utc> = metadata.modified()?.into();
        
        if metadata.is_dir() {
            let children = get_directory_contents(&full_path, &item_relative_path)?;
            let size = calculate_total_size(&children);
            items.push(FileItem::Directory {
                name,
                path: item_relative_path,
                size,
                formatted_size: crate::utils::format_file_size(size, None),
                upload_date,
                children,
            });
        } else {
            let size = metadata.len();
            let extension = StdPath::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();
                
            items.push(FileItem::File {
                name,
                path: item_relative_path,
                size,
                formatted_size: crate::utils::format_file_size(size, None),
                upload_date,
                extension,
            });
        }
    }
    
    items.sort_by(|a, b| {
        let a_type = match a {
            FileItem::Directory { .. } => 0,
            FileItem::File { .. } => 1,
        };
        let b_type = match b {
            FileItem::Directory { .. } => 0,
            FileItem::File { .. } => 1,
        };
        
        if a_type != b_type {
            a_type.cmp(&b_type)
        } else {
            let a_name = match a {
                FileItem::Directory { name, .. } | FileItem::File { name, .. } => name,
            };
            let b_name = match b {
                FileItem::Directory { name, .. } | FileItem::File { name, .. } => name,
            };
            a_name.cmp(b_name)
        }
    });
    
    Ok(items)
}

fn calculate_total_size(items: &[FileItem]) -> u64 {
    items.iter().map(|item| match item {
        FileItem::File { size, .. } => *size,
        FileItem::Directory { size, .. } => *size,
    }).sum()
}

fn count_files(items: &[FileItem]) -> u64 {
    items.iter().map(|item| match item {
        FileItem::File { .. } => 1,
        FileItem::Directory { children, .. } => count_files(children),
    }).sum()
}

fn create_safe_content_disposition(filename: &str) -> String {
    let basename = StdPath::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);

    let sanitized: String = basename.chars()
        .map(|c| if c.is_ascii_control() || c == '"' || c == '\\' { '_' } else { c })
        .collect();

    let is_ascii_printable = sanitized.chars().all(|c| c >= ' ' && c <= '~');

    if is_ascii_printable {
        let escaped = sanitized.replace('\\', "\\\\").replace('"', "\\\"");
        format!("attachment; filename=\"{}\"", escaped)
    } else {
        let encoded = percent_encoding::utf8_percent_encode(&sanitized, percent_encoding::NON_ALPHANUMERIC).to_string();
        let ascii_safe: String = sanitized.chars()
            .map(|c| if c >= ' ' && c <= '~' { c } else { '_' })
            .collect();
        format!("attachment; filename=\"{}\"; filename*=UTF-8''{}", ascii_safe, encoded)
    }
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

    Response::builder()
        .header(axum::http::header::CONTENT_DISPOSITION, content_disposition)
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
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

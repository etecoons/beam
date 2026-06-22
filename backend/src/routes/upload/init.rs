use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;
use crate::routes::upload::UploadState;
use crate::routes::upload::metadata::{
    UploadMetadata, write_upload_metadata, get_unique_folder_path
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
    if payload.filename.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Missing filename" }))).into_response();
    }
    
    let size = payload.file_size;
    let max_size = config.max_file_size;
    if size > max_size {
        return (StatusCode::PAYLOAD_TOO_LARGE, Json(json!({ "error": "File too large", "limit": max_size }))).into_response();
    }

    if let Some(limit) = config.max_storage_limit {
        let items = crate::routes::files::helpers::get_directory_contents(&config.upload_dir, "").unwrap_or_default();
        let total_size = crate::routes::files::helpers::calculate_total_size(&items);
        if total_size + size > limit {
            tracing::warn!("Upload initialization blocked: storage limit exceeded (used: {}, limit: {})", total_size, limit);
            return (StatusCode::INSUFFICIENT_STORAGE, Json(json!({ "error": "Storage limit exceeded" }))).into_response();
        }
    }
    
    let client_batch_id = headers.get("x-batch-id")
        .and_then(|h| h.to_str().ok());
        
    let batch_id = match client_batch_id {
        Some(bid) => {
            if !crate::utils::is_valid_batch_id(bid) {
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid batch ID format" }))).into_response();
            }
            bid.to_string()
        }
        None => {
            let now = chrono::Utc::now().timestamp_millis();
            let rand_str: String = rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Alphanumeric)
                .take(9)
                .map(char::from)
                .collect::<String>()
                .to_lowercase();
            format!("{}-{}", now, rand_str)
        }
    };
    
    state.batch_activity.lock().unwrap().insert(batch_id.clone(), std::time::Instant::now());
    
    let sanitized = crate::utils::sanitize_path_preserve_dirs_safe(&payload.filename);
    let safe_filename = crate::utils::normalize_path(StdPath::new(&sanitized))
        .to_string_lossy()
        .replace('\\', "/");
        
    if let Some(ref allowed) = config.allowed_extensions {
        let file_ext = StdPath::new(&safe_filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();
            
        if !file_ext.is_empty() && !allowed.contains(&file_ext) {
            tracing::warn!("File type not allowed: {} (Extension: {})", safe_filename, file_ext);
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "File type not allowed", "receivedExtension": file_ext }))).into_response();
        }
    }
    
    let upload_id = format!("{:x}", rand::random::<u128>());
    
    let mut final_file_path = config.upload_dir.join(&safe_filename);
    if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
        tracing::error!("Path traversal detected in upload init: {} -> {:?}", safe_filename, final_file_path);
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid file path" }))).into_response();
    }
    
    let path_parts: Vec<&str> = safe_filename.split('/').filter(|s| !s.is_empty()).collect();
    if path_parts.len() > 1 {
        let original_folder_name = path_parts[0];
        let mapping_key = format!("{}-{}", original_folder_name, batch_id);
        
        let new_folder_name = {
            let mut mappings = state.folder_mappings.lock().unwrap();
            if let Some(mapped) = mappings.get(&mapping_key) {
                mapped.clone()
            } else {
                let base_folder_path = config.upload_dir.join(original_folder_name);
                let mapped_path = if base_folder_path.exists() {
                    let unique = get_unique_folder_path(&base_folder_path);
                    let name = unique.file_name().and_then(|n| n.to_str()).unwrap_or(original_folder_name).to_string();
                    tracing::info!("Folder \"{}\" exists or conflict, using unique \"{}\" for batch {}", original_folder_name, name, batch_id);
                    name
                } else {
                    original_folder_name.to_string()
                };
                
                let final_folder_path = config.upload_dir.join(&mapped_path);
                let _ = fs::create_dir_all(final_folder_path);
                
                mappings.insert(mapping_key, mapped_path.clone());
                mapped_path
            }
        };
        
        let mut remapped_parts = path_parts.clone();
        remapped_parts[0] = &new_folder_name;
        
        let remapped_path: PathBuf = remapped_parts.iter().collect();
        final_file_path = config.upload_dir.join(remapped_path);
        
        if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
            return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid file path" }))).into_response();
        }
    } else {
        let _ = fs::create_dir_all(&config.upload_dir);
    }
    
    let mut check_path = final_file_path.clone();
    let mut counter = 1;
    while check_path.exists() {
        tracing::warn!("Final destination file already exists: {:?}. Generating unique name.", check_path);
        let parent = final_file_path.parent().unwrap_or(&config.upload_dir);
        let ext = final_file_path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
        let stem = final_file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        
        check_path = parent.join(format!("{} ({}){}", stem, counter, ext));
        counter += 1;
    }
    
    final_file_path = check_path;
    if !crate::utils::is_path_within_upload_dir(&final_file_path, &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid file path" }))).into_response();
    }
    
    if let Some(parent) = final_file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    let partial_file_path = format!("{}.partial", final_file_path.to_string_lossy());
    if !crate::utils::is_path_within_upload_dir(StdPath::new(&partial_file_path), &config.upload_dir, false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid file path" }))).into_response();
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
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to initialize upload" }))).into_response();
    }
    
    {
        let mut active = state.active_uploads.lock().unwrap();
        active.insert(upload_id.clone(), metadata.clone());
    }
    
    tracing::info!("Initialized persistent upload: {} for {} -> {:?}", upload_id, payload.filename, final_file_path);
    
    if size == 0 {
        if let Err(e) = fs::write(&final_file_path, "") {
            tracing::error!("Failed to create zero-byte file {:?}: {}", final_file_path, e);
            delete_upload_metadata(&config.upload_dir, &upload_id).await;
            {
                let mut active = state.active_uploads.lock().unwrap();
                active.remove(&upload_id);
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to complete zero-byte upload" }))).into_response();
        }
        
        tracing::info!("Completed zero-byte file upload: {} as {:?}", payload.filename, final_file_path);
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        {
            let mut active = state.active_uploads.lock().unwrap();
            active.remove(&upload_id);
        }
        
        let config_clone = config.clone();
        let filename_clone = payload.filename.clone();
        tokio::spawn(async move {
            crate::services::notifications::send_notification(&filename_clone, 0, &config_clone).await;
        });
    }
    
    (StatusCode::OK, Json(InitUploadResponse { upload_id })).into_response()
}

async fn delete_upload_metadata(upload_dir: &StdPath, upload_id: &str) {
    if upload_id.contains("..") {
        return;
    }
    let path = upload_dir.join(".metadata").join(format!("{}.meta", upload_id));
    let _ = tokio::fs::remove_file(path).await;
}

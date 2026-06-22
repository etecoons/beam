use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;

use crate::config::AppConfig;
use crate::routes::auth::RequirePin;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/init", post(init_upload))
        .route("/chunk/:uploadId", post(upload_chunk))
        .route("/cancel/:uploadId", post(cancel_upload))
}

pub struct UploadState {
    pub folder_mappings: Mutex<HashMap<String, String>>,
    pub batch_activity: Mutex<HashMap<String, std::time::Instant>>,
}

impl UploadState {
    pub fn new() -> Self {
        Self {
            folder_mappings: Mutex::new(HashMap::new()),
            batch_activity: Mutex::new(HashMap::new()),
        }
    }
}

pub fn start_batch_cleanup(state: Arc<UploadState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
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
                tracing::info!("Cleaning up {} inactive batch sessions", expired_batches.len());
                let mut activity = state.batch_activity.lock().unwrap();
                let mut mappings = state.folder_mappings.lock().unwrap();
                
                for batch_id in expired_batches {
                    activity.remove(&batch_id);
                    mappings.retain(|key, _| !key.ends_with(&format!("-{}", batch_id)));
                }
            }
        }
    });
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UploadMetadata {
    #[serde(rename = "uploadId")]
    pub upload_id: String,
    #[serde(rename = "originalFilename")]
    pub original_filename: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "partialFilePath")]
    pub partial_file_path: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "bytesReceived")]
    pub bytes_received: u64,
    #[serde(rename = "batchId")]
    pub batch_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "lastActivity")]
    pub last_activity: u64,
}

fn get_metadata_path(upload_dir: &StdPath, upload_id: &str) -> PathBuf {
    upload_dir.join(".metadata").join(format!("{}.meta", upload_id))
}

async fn read_upload_metadata(upload_dir: &StdPath, upload_id: &str) -> Option<UploadMetadata> {
    if upload_id.contains("..") {
        return None;
    }
    let path = get_metadata_path(upload_dir, upload_id);
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    serde_json::from_str(&content).ok()
}

async fn write_upload_metadata(upload_dir: &StdPath, upload_id: &str, mut metadata: UploadMetadata) -> std::io::Result<()> {
    if upload_id.contains("..") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid upload ID"));
    }
    let metadata_dir = upload_dir.join(".metadata");
    tokio::fs::create_dir_all(&metadata_dir).await?;
    
    let path = get_metadata_path(upload_dir, upload_id);
    metadata.last_activity = chrono::Utc::now().timestamp_millis() as u64;
    
    let content = serde_json::to_string_pretty(&metadata)?;
    
    let temp_name = format!("{}.{}.tmp", upload_id, rand::random::<u32>());
    let temp_path = metadata_dir.join(&temp_name);
    
    tokio::fs::write(&temp_path, content).await?;
    if let Err(e) = tokio::fs::rename(&temp_path, &path).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e);
    }
    Ok(())
}

async fn delete_upload_metadata(upload_dir: &StdPath, upload_id: &str) {
    if upload_id.contains("..") {
        return;
    }
    let path = get_metadata_path(upload_dir, upload_id);
    let _ = tokio::fs::remove_file(path).await;
}

fn get_unique_folder_path(folder_path: &StdPath) -> PathBuf {
    let mut counter = 1;
    let mut final_path = folder_path.to_path_buf();
    
    while final_path.exists() {
        let parent = folder_path.parent().unwrap_or_else(|| StdPath::new(""));
        let folder_name = folder_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let new_name = format!("{} ({})", folder_name, counter);
        final_path = parent.join(new_name);
        counter += 1;
    }
    
    final_path
}

#[derive(Deserialize)]
struct InitUploadPayload {
    filename: String,
    #[serde(rename = "fileSize")]
    file_size: u64,
}

#[derive(Serialize)]
struct InitUploadResponse {
    #[serde(rename = "uploadId")]
    upload_id: String,
}

async fn init_upload(
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
    
    let client_batch_id = headers.get("x-batch-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
        
    let batch_id = match client_batch_id {
        Some(ref bid) => {
            if !crate::utils::is_valid_batch_id(bid) {
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid batch ID format" }))).into_response();
            }
            bid.clone()
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
    
    tracing::info!("Initialized persistent upload: {} for {} -> {:?}", upload_id, payload.filename, final_file_path);
    
    if size == 0 {
        if let Err(e) = fs::write(&final_file_path, "") {
            tracing::error!("Failed to create zero-byte file {:?}: {}", final_file_path, e);
            delete_upload_metadata(&config.upload_dir, &upload_id).await;
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to complete zero-byte upload" }))).into_response();
        }
        
        tracing::info!("Completed zero-byte file upload: {} as {:?}", payload.filename, final_file_path);
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        
        let config_clone = config.clone();
        let filename_clone = payload.filename.clone();
        tokio::spawn(async move {
            crate::services::notifications::send_notification(&filename_clone, 0, &config_clone).await;
        });
    }
    
    (StatusCode::OK, Json(InitUploadResponse { upload_id })).into_response()
}

async fn upload_chunk(
    State(config): State<Arc<AppConfig>>,
    State(state): State<Arc<UploadState>>,
    _auth: RequirePin,
    Path(upload_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let chunk_size = body.len() as u64;
    if chunk_size == 0 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Empty chunk received" }))).into_response();
    }
    
    let mut metadata = match read_upload_metadata(&config.upload_dir, &upload_id).await {
        Some(m) => m,
        None => {
            let client_batch_id = headers.get("x-batch-id").and_then(|h| h.to_str().ok()).unwrap_or("none");
            tracing::warn!("Upload metadata not found for chunk request: {}. Client Batch ID: {}.", upload_id, client_batch_id);
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Upload session not found or already completed" }))).into_response();
        }
    };
    
    if !metadata.batch_id.is_empty() && crate::utils::is_valid_batch_id(&metadata.batch_id) {
        state.batch_activity.lock().unwrap().insert(metadata.batch_id.clone(), std::time::Instant::now());
    }
    
    if metadata.bytes_received >= metadata.file_size {
        tracing::warn!("Received chunk for already completed upload {} ({}). Finalizing again.", upload_id, metadata.original_filename);
        let partial_path = StdPath::new(&metadata.partial_file_path);
        let final_path = StdPath::new(&metadata.file_path);
        if !final_path.exists() {
            if partial_path.exists() {
                let _ = tokio::fs::rename(partial_path, final_path).await;
            }
        }
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        return Json(json!({ "bytesReceived": metadata.file_size, "progress": 100 })).into_response();
    }
    
    let mut write_size = chunk_size;
    let mut chunk_bytes = body;
    if metadata.bytes_received + chunk_size > metadata.file_size {
        tracing::warn!("Chunk for {} exceeds expected file size. Expecting {}, got {}. Truncating.", upload_id, metadata.file_size, metadata.bytes_received + chunk_size);
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
        
        let mut file = match tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(partial_path)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to open partial file {:?}: {}", partial_path, e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to open partial file" }))).into_response();
            }
        };
        
        if let Err(e) = file.write_all(&chunk_bytes).await {
            tracing::error!("Failed to write chunk to partial file {:?}: {}", partial_path, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to write chunk" }))).into_response();
        }
        let _ = file.flush().await;
        
        metadata.bytes_received += write_size;
    }
    
    let progress = if metadata.file_size == 0 {
        100
    } else {
        std::cmp::min((metadata.bytes_received as f64 / metadata.file_size as f64 * 100.0).round() as u64, 100)
    };
    
    tracing::debug!("Chunk written for {}: {}/{} ({}%)", upload_id, metadata.bytes_received, metadata.file_size, progress);
    
    if let Err(e) = write_upload_metadata(&config.upload_dir, &upload_id, metadata.clone()).await {
        tracing::error!("Failed to save metadata update: {}", e);
    }
    
    if metadata.bytes_received >= metadata.file_size {
        tracing::info!("Upload {} ({}) completed {} bytes.", upload_id, metadata.original_filename, metadata.bytes_received);
        let partial_path = StdPath::new(&metadata.partial_file_path);
        let final_path = StdPath::new(&metadata.file_path);
        
        match tokio::fs::rename(partial_path, final_path).await {
            Ok(_) => {
                tracing::info!("Upload completed and finalized: {} as {:?}", metadata.original_filename, final_path);
                delete_upload_metadata(&config.upload_dir, &upload_id).await;
                
                let config_clone = config.clone();
                let filename_clone = metadata.original_filename.clone();
                let filesize_clone = metadata.file_size;
                tokio::spawn(async move {
                    crate::services::notifications::send_notification(&filename_clone, filesize_clone, &config_clone).await;
                });
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::warn!("Partial file {:?} missing during finalization, assuming completed elsewhere.", partial_path);
                    delete_upload_metadata(&config.upload_dir, &upload_id).await;
                } else {
                    tracing::error!("CRITICAL: Failed to rename partial file {:?} to {:?}: {}", partial_path, final_path, e);
                }
            }
        }
    }
    
    Json(json!({ "bytesReceived": metadata.bytes_received, "progress": progress })).into_response()
}

async fn cancel_upload(
    State(config): State<Arc<AppConfig>>,
    Path(upload_id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Received cancel request for upload: {}", upload_id);
    
    if let Some(metadata) = read_upload_metadata(&config.upload_dir, &upload_id).await {
        let partial_path = StdPath::new(&metadata.partial_file_path);
        if partial_path.exists() {
            let _ = tokio::fs::remove_file(partial_path).await;
            tracing::info!("Deleted partial file on cancellation: {:?}", partial_path);
        }
        delete_upload_metadata(&config.upload_dir, &upload_id).await;
        tracing::info!("Upload cancelled and cleaned up: {} ({})", upload_id, metadata.original_filename);
    } else {
        tracing::warn!("Cancel request for non-existent or already completed upload: {}", upload_id);
    }
    
    Json(json!({ "message": "Upload cancelled or already complete" }))
}

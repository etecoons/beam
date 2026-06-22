use axum::http::StatusCode;
use serde_json::json;
use std::path::{Path as StdPath, PathBuf};

pub fn generate_batch_id() -> String {
    let now = chrono::Utc::now().timestamp_millis();
    let rand_str: String =
        rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Alphanumeric)
            .take(9)
            .map(char::from)
            .collect::<String>()
            .to_lowercase();
    format!("{}-{}", now, rand_str)
}

pub fn get_unique_filename(base_path: &StdPath, upload_dir: &StdPath) -> PathBuf {
    let mut check_path = base_path.to_path_buf();
    let mut counter = 1;
    while check_path.exists() {
        let parent = base_path.parent().unwrap_or(upload_dir);
        let ext = base_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let stem = base_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        check_path = parent.join(format!("{} ({}){}", stem, counter, ext));
        counter += 1;
    }
    check_path
}

pub fn validate_upload(
    config: &crate::config::AppConfig,
    filename: &str,
    file_size: u64,
) -> Result<(), (StatusCode, serde_json::Value)> {
    if filename.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": "Missing filename" }),
        ));
    }

    let max_size = config.max_file_size;
    if file_size > max_size {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            json!({ "error": "File too large", "limit": max_size }),
        ));
    }

    if let Some(limit) = config.max_storage_limit {
        let items = crate::routes::files::helpers::get_directory_contents(&config.upload_dir, "")
            .unwrap_or_default();
        let total_size = crate::routes::files::helpers::calculate_total_size(&items);
        if total_size + file_size > limit {
            tracing::warn!(
                "Upload initialization blocked: storage limit exceeded (used: {}, limit: {})",
                total_size,
                limit
            );
            return Err((
                StatusCode::INSUFFICIENT_STORAGE,
                json!({ "error": "Storage limit exceeded" }),
            ));
        }
    }

    Ok(())
}

pub fn validate_extension(
    config: &crate::config::AppConfig,
    safe_filename: &str,
) -> Result<(), (StatusCode, serde_json::Value)> {
    if let Some(ref allowed) = config.allowed_extensions {
        let file_ext = StdPath::new(safe_filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();

        if !file_ext.is_empty() && !allowed.contains(&file_ext) {
            tracing::warn!(
                "File type not allowed: {} (Extension: {})",
                safe_filename,
                file_ext
            );
            return Err((
                StatusCode::BAD_REQUEST,
                json!({ "error": "File type not allowed", "receivedExtension": file_ext }),
            ));
        }
    }
    Ok(())
}

pub fn get_remapped_folder_path(
    config: &crate::config::AppConfig,
    state: &crate::routes::upload::UploadState,
    path_parts: &[&str],
    batch_id: &str,
) -> PathBuf {
    let original_folder_name = path_parts[0];
    let mapping_key = format!("{}-{}", original_folder_name, batch_id);

    let new_folder_name = {
        let mut mappings = state.folder_mappings.lock().unwrap();
        if let Some(mapped) = mappings.get(&mapping_key) {
            mapped.clone()
        } else {
            let base_folder_path = config.upload_dir.join(original_folder_name);
            let mapped_path = if base_folder_path.exists() {
                let unique = super::metadata::get_unique_folder_path(&base_folder_path);
                let name = unique
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(original_folder_name)
                    .to_string();
                tracing::info!(
                    "Folder \"{}\" exists or conflict, using unique \"{}\" for batch {}",
                    original_folder_name,
                    name,
                    batch_id
                );
                name
            } else {
                original_folder_name.to_string()
            };

            let final_folder_path = config.upload_dir.join(&mapped_path);
            let _ = std::fs::create_dir_all(final_folder_path);

            mappings.insert(mapping_key, mapped_path.clone());
            mapped_path
        }
    };

    let mut remapped_parts = path_parts.to_vec();
    remapped_parts[0] = &new_folder_name;

    let remapped_path: PathBuf = remapped_parts.iter().collect();
    config.upload_dir.join(remapped_path)
}

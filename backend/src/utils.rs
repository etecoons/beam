use std::fs;
use std::path::{Path, PathBuf, Component};

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek() {
        let buf = PathBuf::from(c.as_os_str());
        components.next();
        buf
    } else {
        PathBuf::new()
    };

    let mut normalized = Vec::new();
    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
        }
    }
    for component in normalized {
        ret.push(component);
    }
    ret
}

pub fn is_path_within_upload_dir(file_path: &Path, upload_dir: &Path, require_exists: bool) -> bool {
    let real_upload_dir = match fs::canonicalize(upload_dir) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let resolved_file_path = if require_exists {
        if !file_path.exists() {
            return false;
        }
        match fs::canonicalize(file_path) {
            Ok(p) => p,
            Err(_) => return false,
        }
    } else {
        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(file_path)
        };
        normalize_path(&absolute_path)
    };

    resolved_file_path.starts_with(real_upload_dir)
}

pub fn sanitize_filename_safe(filename: &str) -> String {
    if filename.is_empty() {
        return "unnamed_file.txt".to_string();
    }

    let path = Path::new(filename);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    
    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed_file");

    // Replace spaces and + with underscores
    let mut base_name = stem.replace(|c: char| c.is_whitespace() || c == '+', "_");

    // Remove unsafe characters (only keep alphanumeric, -, _, .)
    base_name = base_name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    // Replace multiple underscores with single
    while base_name.contains("__") {
        base_name = base_name.replace("__", "_");
    }

    // Remove leading/trailing dots, underscores, hyphens
    let trimmed = base_name.trim_matches(|c| c == '.' || c == '_' || c == '-').to_string();
    let mut final_base = if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed
    };

    // Check for Windows reserved names
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
    ];
    if reserved_names.contains(&final_base.to_uppercase().as_str()) {
        final_base.push_str("_file");
    }

    if final_base.len() > 200 {
        final_base.truncate(200);
    }

    let clean_ext: String = ext.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.')
        .collect();

    if clean_ext.is_empty() {
        final_base
    } else if clean_ext.starts_with('.') {
        format!("{}{}", final_base, clean_ext)
    } else {
        format!("{}.{}", final_base, clean_ext)
    }
}

pub fn sanitize_path_preserve_dirs_safe(file_path: &str) -> String {
    if file_path.is_empty() {
        return "unnamed_file.txt".to_string();
    }

    let parts: Vec<String> = file_path
        .split('/')
        .map(|part| part.replace('\\', "/"))
        .flat_map(|part| part.split('/').map(|p| p.to_string()).collect::<Vec<String>>())
        .filter(|part| !part.is_empty() && part != "." && part != "..")
        .map(|part| sanitize_filename_safe(&part))
        .collect();

    if parts.is_empty() {
        "unnamed_file.txt".to_string()
    } else {
        parts.join("/")
    }
}

pub fn format_file_size(bytes: u64, unit: Option<&str>) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    
    if let Some(u) = unit {
        let requested = u.to_uppercase();
        if let Some(idx) = units.iter().position(|&x| x == requested) {
            let size = bytes as f64 / 1024_f64.powi(idx as i32);
            return format!("{:.2}{}", size, requested);
        }
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2}{}", size, units[unit_idx])
}

pub fn calculate_directory_size<P: AsRef<Path>>(dir: P) -> u64 {
    let mut total_size = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    total_size += calculate_directory_size(entry.path());
                }
            }
        }
    }
    total_size
}

pub fn is_valid_batch_id(batch_id: &str) -> bool {
    let parts: Vec<&str> = batch_id.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let second = parts[1];
    if second.len() < 8 || second.len() > 9 {
        return false;
    }
    second.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

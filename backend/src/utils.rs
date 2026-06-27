use std::fs;
use std::path::{Component, Path, PathBuf};

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

/// Returns `true` if `file_path` (after canonicalization) lives inside
/// `upload_dir`.
///
/// Canonicalization is the only sound defense against symlink-based path
/// traversal: a string-level check of `..` is insufficient because a
/// symlink in the upload dir that points outside (e.g. `/uploads/escape ->
/// /etc`) would otherwise pass parent-based containment checks while the
/// kernel follows the symlink at write time.
///
/// For non-existent targets (e.g. an in-progress upload), we walk the path
/// component by component, canonicalize the deepest EXISTING ancestor, then
/// re-attach the non-existent suffix. Any `..` component in the input is
/// rejected as a traversal attempt (caller-side sanitization is the right
/// place to normalize `..`, not here).
#[must_use]
pub fn is_path_within_upload_dir(
    file_path: &Path,
    upload_dir: &Path,
    require_exists: bool,
) -> bool {
    let real_upload_dir = match fs::canonicalize(upload_dir) {
        Ok(p) => p,
        Err(_) => return false,
    };

    if require_exists {
        if !file_path.exists() {
            return false;
        }
        return match fs::canonicalize(file_path) {
            Ok(p) => p.starts_with(&real_upload_dir),
            Err(_) => false,
        };
    }

    // Non-existent target. Reject any `..` component as a traversal signal
    // before walking — the caller should have normalized the path through
    // `sanitize_path_preserve_dirs_safe` already, which removes `..`. A
    // `..` slipping through is a security incident.
    if file_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return false;
    }

    // Make the path absolute relative to the upload dir's parent. This
    // handles both relative `file_path` (interpreted under CWD) and
    // absolute `file_path` (used as-is).
    let absolute_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(file_path)
    };

    // Walk component-by-component, canonicalize the deepest existing
    // ancestor, re-attach the non-existent suffix.
    //
    // The key insight: for each `Normal(c)` component, we check whether
    // `existing.join(c)` exists on disk. If yes, we extend `existing`. If
    // no, we push `c` to the suffix. This way we always end up with the
    // DEEPEST existing ancestor in `existing` and the rest in `suffix`.
    let mut existing = PathBuf::new();
    let mut suffix = PathBuf::new();
    for component in absolute_path.components() {
        match component {
            Component::Prefix(p) => {
                existing = PathBuf::from(p.as_os_str());
            }
            Component::RootDir => {
                existing = PathBuf::from(std::path::MAIN_SEPARATOR.to_string());
            }
            Component::CurDir => {
                // Skip "." silently.
            }
            Component::ParentDir => {
                // Already filtered above.
                unreachable!("filtered above");
            }
            Component::Normal(c) => {
                let candidate = existing.join(c);
                if candidate.exists() {
                    existing = candidate;
                } else {
                    suffix.push(c);
                }
            }
        }
    }

    // If nothing on the path exists, fall back to a string-level
    // containment check on the normalized path. This handles the
    // edge case where the upload dir itself doesn't exist yet (e.g.
    // bootstrap) but the user is supplying a relative path under it.
    if suffix == existing || existing.as_os_str().is_empty() {
        return normalize_path(&absolute_path).starts_with(&real_upload_dir);
    }

    let canonical_existing = match fs::canonicalize(&existing) {
        Ok(p) => p,
        Err(_) => return false,
    };

    // The deepest existing ancestor must be the upload dir itself or a
    // descendant of it. If it's an ancestor (e.g. /tmp when the upload
    // dir is /tmp/beam_test_X), that means the upload dir doesn't
    // exist — but the user is trying to write to a path under it. We
    // still allow this as long as the candidate (existing + suffix) is
    // under real_upload_dir.
    if !canonical_existing.starts_with(&real_upload_dir) {
        return false;
    }

    let candidate = canonical_existing.join(&suffix);
    candidate.starts_with(&real_upload_dir)
}

pub fn sanitize_filename_safe(filename: &str) -> String {
    if filename.is_empty() {
        return "unnamed_file.txt".to_string();
    }

    let path = Path::new(filename);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed_file");

    // Replace spaces and + with underscores
    let mut base_name = stem.replace(|c: char| c.is_whitespace() || c == '+', "_");

    // Remove unsafe characters (only keep alphanumeric, -, _, .)
    base_name = base_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    // Replace multiple underscores with single
    while base_name.contains("__") {
        base_name = base_name.replace("__", "_");
    }

    // Remove leading/trailing dots, underscores, hyphens
    let trimmed = base_name
        .trim_matches(|c| c == '.' || c == '_' || c == '-')
        .to_string();
    let mut final_base = if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed
    };

    // Check for Windows reserved names
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved_names.contains(&final_base.to_uppercase().as_str()) {
        final_base.push_str("_file");
    }

    if final_base.len() > 200 {
        final_base.truncate(200);
    }

    let clean_ext: String = ext
        .chars()
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

    // Defense in depth: any `..` component (whether encoded, escaped, or
    // inline) is a path-traversal attempt and must be rejected outright.
    // The caller should treat the resulting name as the FINAL safe value;
    // a `..` slipping through here is a security incident.
    if file_path.split(['/', '\\']).any(|p| p == "..") {
        tracing::warn!(
            "sanitize_path_preserve_dirs_safe: rejected path containing '..': {file_path}"
        );
        return "unnamed_file.txt".to_string();
    }

    let parts: Vec<String> = file_path
        .split('/')
        .map(|part| part.replace('\\', "/"))
        .flat_map(|part| {
            part.split('/')
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
        })
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
    second
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

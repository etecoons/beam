use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::path::Path as StdPath;

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

pub fn get_directory_contents(
    dir_path: &StdPath,
    relative_path: &str,
) -> std::io::Result<Vec<FileItem>> {
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

pub fn calculate_total_size(items: &[FileItem]) -> u64 {
    items
        .iter()
        .map(|item| match item {
            FileItem::File { size, .. } => *size,
            FileItem::Directory { size, .. } => *size,
        })
        .sum()
}

pub fn count_files(items: &[FileItem]) -> u64 {
    items
        .iter()
        .map(|item| match item {
            FileItem::File { .. } => 1,
            FileItem::Directory { children, .. } => count_files(children),
        })
        .sum()
}

/// Extensions that can be rendered by the browser and would lead to stored
/// XSS if served with their native content type. We override these to
/// `application/octet-stream` and force `Content-Disposition: attachment`
/// so the browser downloads the file rather than rendering it.
const DANGEROUS_EXTENSIONS: &[&str] = &[
    "html", "htm", "svg", "mht", "mhtml",
    "xhtml", "xht",
    "js", "mjs", "jsx", "ts", "tsx",
    "xml", "xsl", "xslt", "rss", "atom",
    "pdf",  // downloaded by default for safety (could be inline-rendered with a viewer)
    "swf",
    "html5",
];

pub fn is_dangerous_extension(ext: &str) -> bool {
    DANGEROUS_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
}

pub fn create_safe_content_disposition(filename: &str) -> String {
    let basename = StdPath::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);

    let sanitized: String = basename
        .chars()
        .map(|c| {
            if c.is_ascii_control() || c == '"' || c == '\\' {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Always force `attachment` so the browser downloads rather than
    // rendering. This is the defense against stored XSS via uploaded files:
    // even if `get_content_type` returns a renderable type (e.g. `image/png`
    // for a polyglot), the browser will not navigate to it inline.
    let is_ascii_printable = sanitized.chars().all(|c| (' '..='~').contains(&c));

    if is_ascii_printable {
        let escaped = sanitized.replace('\\', "\\\\").replace('"', "\\\"");
        format!("attachment; filename=\"{}\"", escaped)
    } else {
        let encoded =
            percent_encoding::utf8_percent_encode(&sanitized, percent_encoding::NON_ALPHANUMERIC)
                .to_string();
        let ascii_safe: String = sanitized
            .chars()
            .map(|c| if (' '..='~').contains(&c) { c } else { '_' })
            .collect();
        format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            ascii_safe, encoded
        )
    }
}

pub fn get_content_type(filename: &str) -> &'static str {
    let ext = StdPath::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // Defense in depth: dangerous extensions always get octet-stream, even
    // if a future match arm tries to give them a renderable type. See
    // `create_safe_content_disposition` for the matching Content-Disposition
    // override.
    if is_dangerous_extension(&ext) {
        return "application/octet-stream";
    }

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "css" => "text/css",
        "json" => "application/json",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_extensions_get_octet_stream() {
        for ext in [
            "html", "htm", "svg", "xhtml", "xht", "mhtml",
            "js", "mjs", "xml", "xsl", "pdf",
        ] {
            let filename = format!("test.{ext}");
            assert_eq!(
                get_content_type(&filename),
                "application/octet-stream",
                "extension {ext} should map to octet-stream"
            );
        }
    }

    #[test]
    fn dangerous_extensions_case_insensitive() {
        for ext in ["HTML", "Svg", "XHTML", "JS"] {
            let filename = format!("test.{ext}");
            assert_eq!(get_content_type(&filename), "application/octet-stream");
        }
    }

    #[test]
    fn safe_image_extensions_keep_their_types() {
        assert_eq!(get_content_type("photo.jpg"), "image/jpeg");
        assert_eq!(get_content_type("photo.png"), "image/png");
        assert_eq!(get_content_type("photo.gif"), "image/gif");
    }

    #[test]
    fn unknown_extensions_default_to_octet_stream() {
        assert_eq!(get_content_type("file.qwxyz"), "application/octet-stream");
    }

    #[test]
    fn content_disposition_is_always_attachment() {
        let d = create_safe_content_disposition("photo.png");
        assert!(d.starts_with("attachment;"), "got: {d}");
        assert!(d.contains("photo.png"), "got: {d}");
    }

    #[test]
    fn content_disposition_handles_unicode_safely() {
        let d = create_safe_content_disposition("файл.txt");
        assert!(d.starts_with("attachment;"));
        assert!(d.contains("filename*=UTF-8''"), "got: {d}");
    }

    #[test]
    fn content_disposition_sanitizes_quotes_and_backslashes() {
        let d = create_safe_content_disposition("evil\"name\\.txt");
        assert!(d.starts_with("attachment;"));
        // Quotes should be escaped or replaced
        assert!(!d.contains("evil\"name"), "raw quote in disposition: {d}");
    }
}

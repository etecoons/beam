//! PWA manifest generation.
//!
//! Walks `frontend/dist` after a Trunk build and writes:
//! - `asset-manifest.json` — list of every static asset URL
//! - `manifest.json` — PWA web-app manifest

use std::fs;
use std::path::Path;

use crate::config::AppConfig;

pub fn generate_pwa_manifest(config: &AppConfig) {
    let mut manifest_assets = Vec::new();
    walk_dir(Path::new("frontend/dist"), "", &mut manifest_assets);

    let asset_path = Path::new("frontend/dist/asset-manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&manifest_assets) {
        let _ = fs::write(asset_path, json);
    }

    let pwa_manifest = serde_json::json!({
        "name": &config.server.site_title,
        "short_name": &config.server.site_title,
        "description": "A simple file upload application",
        "start_url": "/",
        "display": "standalone",
        "background_color": "#ffffff",
        "theme_color": "#000000",
        "icons": [
            { "src": "favicon.svg", "type": "image/svg+xml", "sizes": "any" },
            { "src": "favicon.png", "type": "image/png",     "sizes": "192x192" },
            { "src": "favicon.png", "type": "image/png",     "sizes": "512x512" },
        ],
        "orientation": "any"
    });

    let pwa_path = Path::new("frontend/dist/manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&pwa_manifest) {
        let _ = fs::write(pwa_path, json);
    }
}

fn walk_dir(dir: &Path, base: &str, assets: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let full = entry.path();
        let rel = if base.is_empty() {
            name.clone()
        } else {
            format!("{base}/{name}")
        };
        if full.is_dir() {
            walk_dir(&full, &rel, assets);
        } else {
            assets.push(format!("/{rel}"));
        }
    }
}
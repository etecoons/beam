use axum::{Json, extract::State};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Serialize)]
pub struct FrontendConfig {
    pub site_title: String,
    pub auto_upload: bool,
    pub show_file_list: bool,
    pub pin_required: bool,
    pub pin_length: usize,
    pub max_file_size: u64,
    pub client_max_retries: u32,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
    pub show_version: bool,
    pub show_github: bool,
}

pub async fn get_config(State(config): State<Arc<AppConfig>>) -> Json<FrontendConfig> {
    Json(FrontendConfig {
        site_title: config.site_title.clone(),
        auto_upload: config.auto_upload,
        show_file_list: config.show_file_list,
        pin_required: config.pin.is_some(),
        pin_length: config.pin.as_ref().map(|p| p.len()).unwrap_or(0),
        max_file_size: config.max_file_size,
        client_max_retries: config.client_max_retries,
        enable_translation: config.enable_translation,
        enable_themes: config.enable_themes,
        enable_print: config.enable_print,
        show_version: config.show_version,
        show_github: config.show_github,
    })
}

pub async fn pin_required(State(config): State<Arc<AppConfig>>) -> Json<serde_json::Value> {
    let length = config.pin.as_ref().map(|p| p.len()).unwrap_or(0);
    Json(json!({
        "required": config.pin.is_some(),
        "length": length,
        "enable_translation": config.enable_translation,
        "enable_themes": config.enable_themes,
        "enable_print": config.enable_print,
    }))
}

#![allow(dead_code)]
use crate::types::Language;

mod de;
mod en;
mod es;
mod fr;
mod ja;
mod pt;
mod ru;
mod zh;

pub struct Translations {
    pub enter_pin: &'static str,
    pub locked_out: &'static str,
    pub pin_required: &'static str,
    pub drag_drop_prompt: &'static str,
    pub or: &'static str,
    pub browse_files: &'static str,
    pub browse_folders: &'static str,
    pub uploaded_files: &'static str,
    pub no_files: &'static str,
    pub name: &'static str,
    pub size: &'static str,
    pub actions: &'static str,
    pub rename_item: &'static str,
    pub cancel: &'static str,
    pub rename: &'static str,
    pub delete: &'static str,
    pub copy_link: &'static str,
    pub log_out: &'static str,
    pub file_singular: &'static str,
    pub file_plural: &'static str,
    pub uploading: &'static str,
    pub theme: &'static str,

    // Additional localizations
    pub loading_files: &'static str,
    pub download: &'static str,
    pub queued: &'static str,
    pub initializing: &'static str,
    pub complete: &'static str,
    pub error: &'static str,
    pub of: &'static str,
    pub upload_files_btn: &'static str,
    pub invalid_pin: &'static str,

    pub delete_confirm_prefix: &'static str,
    pub delete_confirm_suffix: &'static str,
    pub deleted_prefix: &'static str,
    pub delete_failed_prefix: &'static str,
    pub renamed_prefix: &'static str,
    pub rename_failed_prefix: &'static str,
    pub failed_load_files_prefix: &'static str,
    pub file_uploaded_prefix: &'static str,
    pub upload_failed_prefix: &'static str,
    pub failed_process_drop_prefix: &'static str,
    pub download_link_copied: &'static str,
    pub failed_copy_link: &'static str,
    pub authentication_success: &'static str,
    pub online_restored: &'static str,
    pub online_lost: &'static str,
    pub file_too_large_prefix: &'static str,
}

pub fn get_translations(lang: Language) -> Translations {
    match lang {
        Language::English => en::get_translations(),
        Language::Chinese => zh::get_translations(),
        Language::Spanish => es::get_translations(),
        Language::German => de::get_translations(),
        Language::Japanese => ja::get_translations(),
        Language::French => fr::get_translations(),
        Language::Portuguese => pt::get_translations(),
        Language::Russian => ru::get_translations(),
    }
}

use crate::storage::StorageService;

pub fn get_saved_language() -> Language {
    let stored = StorageService::get_item("lang", "");
    if !stored.is_empty() {
        match stored.as_str() {
            "zh" => Language::Chinese,
            "es" => Language::Spanish,
            "de" => Language::German,
            "ja" => Language::Japanese,
            "fr" => Language::French,
            "pt" => Language::Portuguese,
            "ru" => Language::Russian,
            _ => Language::English,
        }
    } else {
        let window = web_sys::window().unwrap();
        if let Some(nav) = window.navigator().language() {
            let nav = nav.to_lowercase();
            if nav.starts_with("zh") {
                return Language::Chinese;
            } else if nav.starts_with("es") {
                return Language::Spanish;
            } else if nav.starts_with("de") {
                return Language::German;
            } else if nav.starts_with("ja") {
                return Language::Japanese;
            } else if nav.starts_with("fr") {
                return Language::French;
            } else if nav.starts_with("pt") {
                return Language::Portuguese;
            } else if nav.starts_with("ru") {
                return Language::Russian;
            }
        }
        Language::English
    }
}

pub fn save_language(lang: Language) {
    StorageService::set_item("lang", lang.code());
}

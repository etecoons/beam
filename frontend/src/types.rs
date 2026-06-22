#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrontendConfig {
    pub site_title: String,
    pub auto_upload: bool,
    pub show_file_list: bool,
    pub pin_required: bool,
    pub pin_length: usize,
    pub max_file_size: u64,
    pub client_max_retries: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileListResponse {
    pub items: Vec<FileItem>,
    #[serde(rename = "totalFiles")]
    pub total_files: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    #[serde(rename = "formattedTotalSize")]
    pub formatted_total_size: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FileItem {
    File {
        name: String,
        path: String,
        size: u64,
        #[serde(rename = "formattedSize")]
        formatted_size: String,
        #[serde(rename = "uploadDate")]
        upload_date: String,
        extension: String,
    },
    Directory {
        name: String,
        path: String,
        size: u64,
        #[serde(rename = "formattedSize")]
        formatted_size: String,
        #[serde(rename = "uploadDate")]
        upload_date: String,
        children: Vec<FileItem>,
    },
}

#[derive(Clone, Debug)]
pub struct UploadProgress {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub uploaded: u64,
    pub rate: f64,
    pub status: String,
    pub error_color: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RenameData {
    pub item_path: String,
    pub current_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    Chinese,
    Spanish,
    German,
    Japanese,
    French,
    Portuguese,
    Russian,
}

pub enum Msg {
    Nothing,

    // Core Configuration & Theme
    LoadConfig(Result<FrontendConfig, String>),
    ToggleTheme,
    SwitchLanguage(Language),

    // Authentication / PIN digits
    PinInputChanged(String),
    VerifyPin,
    PinVerificationResult(Result<bool, String>),
    Logout,

    // Upload interaction
    DragOver(bool),
    FilesSelected(Vec<web_sys::File>),
    FoldersSelected(Vec<web_sys::File>),
    DropProcessed(Result<Vec<web_sys::File>, String>),
    StartUploads,

    // Upload callbacks from async tasks
    UploadInit(String, String), // path, upload_id
    UploadProgressUpdate(String, u64, f64, String, Option<String>), // path, uploaded_bytes, rate, status, error_color
    UploadCompleted(String),                                        // path
    UploadFailed(String, String),                                   // path, error

    // Loaded files interaction
    LoadFileList(Result<FileListResponse, String>),
    RefreshFiles,
    DeleteFile(String),
    DeleteResult(Result<String, String>),

    // Rename Modal
    StartRename(String, String), // path, current_name
    CancelRename,
    ConfirmRename,
    RenameInputChanged(String),
    RenameResult(Result<String, String>),

    // Toast alerts
    AddToast(String, String), // message, type
    RemoveToast(usize),
}

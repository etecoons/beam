use std::collections::HashMap;
use yew::prelude::*;
use yew::html::Scope;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Inline JS to recursively retrieve files (preserving webkitRelativePath) from drag-and-drop DataTransfer.
#[wasm_bindgen(inline_js = r#"
export async function getFilesFromDataTransfer(dataTransfer) {
    const items = dataTransfer.items;
    if (!items) return [];
    
    let fileEntries = [];
    let rootFolderName = null;

    async function traverseEntry(entry, path = '') {
        if (entry.isFile) {
            const file = await new Promise((resolve, reject) => {
                entry.file((file) => {
                    if (!rootFolderName && path) {
                        rootFolderName = path.split('/')[0];
                    }
                    const fullPath = path ? (path + '/' + entry.name) : entry.name;
                    const fileWithPath = new File([file], entry.name, {
                        type: file.type,
                        lastModified: file.lastModified,
                    });

                    if (rootFolderName) {
                        const relativePath = fullPath.startsWith(rootFolderName)
                            ? fullPath
                            : (rootFolderName + '/' + fullPath);
                        Object.defineProperty(fileWithPath, 'webkitRelativePath', {
                            value: relativePath,
                            writable: false,
                            configurable: true,
                        });
                    } else {
                        Object.defineProperty(fileWithPath, 'webkitRelativePath', {
                            value: fullPath,
                            writable: false,
                            configurable: true,
                        });
                    }
                    resolve(fileWithPath);
                }, reject);
            });
            fileEntries.push(file);
        } else if (entry.isDirectory) {
            if (!path && !rootFolderName) {
                rootFolderName = entry.name;
            }
            const dirReader = entry.createReader();
            let entries = [];

            let readEntries = await new Promise((resolve, reject) => {
                const readNextBatch = () => {
                    dirReader.readEntries((batch) => {
                        if (batch.length > 0) {
                            entries = entries.concat(batch);
                            readNextBatch();
                        } else {
                            resolve(entries);
                        }
                    }, reject);
                };
                readNextBatch();
            });

            const dirPath = path ? (path + '/' + entry.name) : entry.name;
            for (const childEntry of entries) {
                await traverseEntry(childEntry, dirPath);
            }
        }
    }

    for (const item of items) {
        if (item.webkitGetAsEntry) {
            const entry = item.webkitGetAsEntry();
            if (entry) {
                await traverseEntry(entry);
            }
        }
    }

    fileEntries.sort((a, b) => a.webkitRelativePath.localeCompare(b.webkitRelativePath));
    return fileEntries;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = getFilesFromDataTransfer, catch)]
    async fn get_files_from_data_transfer(data_transfer: &web_sys::DataTransfer) -> Result<js_sys::Array, JsValue>;
}

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
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub toast_type: String, // "success" | "error" | "info"
}

#[derive(Clone, Debug)]
pub struct RenameData {
    pub item_path: String,
    pub current_name: String,
}

pub enum Msg {
    Nothing,
    
    // Core Configuration & Theme
    LoadConfig(Result<FrontendConfig, String>),
    ToggleTheme,
    
    // Authentication / PIN digits
    PinDigitInput(usize, String),
    PinBackspace(usize),
    PinPaste(String),
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
    UploadCompleted(String), // path
    UploadFailed(String, String), // path, error
    
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

pub struct App {
    // Configuration
    config: Option<FrontendConfig>,
    is_authenticated: bool,
    theme: String,
    
    // PIN entry inputs
    pin_digits: Vec<String>,
    pin_refs: Vec<NodeRef>,
    error_message: Option<String>,
    is_lockout: bool,
    
    // Upload tracking
    upload_queue: Vec<web_sys::File>,
    active_uploads: HashMap<String, UploadProgress>, // key: path
    is_uploading: bool,
    drag_over: bool,
    file_input_ref: NodeRef,
    folder_input_ref: NodeRef,
    
    // File list & Explorer
    uploaded_files: Option<FileListResponse>,
    
    // Rename Modal
    rename_target: Option<RenameData>,
    rename_input_val: String,
    
    // Toasts
    toasts: Vec<Toast>,
    toast_timeouts: HashMap<usize, gloo_timers::callback::Timeout>,
    next_toast_id: usize,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Theme
        let theme = get_saved_theme();
        set_theme_attribute(&theme);
        
        // Fetch config
        let link = ctx.link().clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_config().await {
                Ok(conf) => link.send_message(Msg::LoadConfig(Ok(conf))),
                Err(err) => link.send_message(Msg::LoadConfig(Err(err))),
            }
        });

        Self {
            config: None,
            is_authenticated: false,
            theme,
            pin_digits: Vec::new(),
            pin_refs: Vec::new(),
            error_message: None,
            is_lockout: false,
            upload_queue: Vec::new(),
            active_uploads: HashMap::new(),
            is_uploading: false,
            drag_over: false,
            file_input_ref: NodeRef::default(),
            folder_input_ref: NodeRef::default(),
            uploaded_files: None,
            rename_target: None,
            rename_input_val: String::new(),
            toasts: Vec::new(),
            toast_timeouts: HashMap::new(),
            next_toast_id: 0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Nothing => false,
            
            Msg::LoadConfig(res) => {
                match res {
                    Ok(conf) => {
                        let pin_len = conf.pin_length;
                        self.pin_digits = vec!["".to_string(); pin_len];
                        self.pin_refs = (0..pin_len).map(|_| NodeRef::default()).collect();
                        
                        let site_title = conf.site_title.clone();
                        self.config = Some(conf.clone());
                        
                        // Set document title dynamically
                        if let Some(doc) = gloo_utils::document().default_view().and_then(|w| w.document()) {
                            doc.set_title(&format!("{} - Simple File Upload", site_title));
                        }
                        
                        if !conf.pin_required {
                            self.is_authenticated = true;
                            ctx.link().send_message(Msg::RefreshFiles);
                        } else {
                            // Verify if already authenticated via session/cookie
                            let link = ctx.link().clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if check_already_authenticated().await {
                                    link.send_message(Msg::PinVerificationResult(Ok(true)));
                                } else {
                                    link.send_message(Msg::PinVerificationResult(Err("".to_string())));
                                }
                            });
                        }
                    }
                    Err(e) => {
                        self.show_toast(ctx, &format!("Failed to load configuration: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::ToggleTheme => {
                self.theme = if self.theme == "light" { "dark".to_string() } else { "light".to_string() };
                save_theme(&self.theme);
                set_theme_attribute(&self.theme);
                true
            }
            
            Msg::PinDigitInput(idx, val) => {
                if self.is_lockout {
                    return false;
                }
                
                let filtered: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
                
                if !filtered.is_empty() {
                    // Update input value
                    let single_char = filtered.chars().next().unwrap().to_string();
                    self.pin_digits[idx] = single_char.clone();
                    
                    if let Some(input) = self.pin_refs[idx].cast::<web_sys::HtmlInputElement>() {
                        input.set_value(&single_char);
                    }
                    
                    // Move focus
                    if idx < self.pin_digits.len() - 1 {
                        if let Some(next_input) = self.pin_refs[idx + 1].cast::<web_sys::HtmlInputElement>() {
                            let _ = next_input.focus();
                        }
                    } else {
                        // Submit on last digit filled
                        if self.pin_digits.iter().all(|d| !d.is_empty()) {
                            ctx.link().send_message(Msg::VerifyPin);
                        }
                    }
                } else {
                    self.pin_digits[idx] = "".to_string();
                    if let Some(input) = self.pin_refs[idx].cast::<web_sys::HtmlInputElement>() {
                        input.set_value("");
                    }
                }
                true
            }
            
            Msg::PinBackspace(idx) => {
                if self.is_lockout {
                    return false;
                }
                
                if self.pin_digits[idx].is_empty() && idx > 0 {
                    self.pin_digits[idx - 1] = "".to_string();
                    if let Some(prev_input) = self.pin_refs[idx - 1].cast::<web_sys::HtmlInputElement>() {
                        prev_input.set_value("");
                        let _ = prev_input.focus();
                    }
                    true
                } else {
                    false
                }
            }
            
            Msg::PinPaste(text) => {
                if self.is_lockout {
                    return false;
                }
                
                let digits: Vec<char> = text.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.is_empty() {
                    return false;
                }
                
                for (i, digit) in digits.into_iter().enumerate() {
                    if i < self.pin_digits.len() {
                        self.pin_digits[i] = digit.to_string();
                        if let Some(input) = self.pin_refs[i].cast::<web_sys::HtmlInputElement>() {
                            input.set_value(&digit.to_string());
                        }
                    }
                }
                
                if self.pin_digits.iter().all(|d| !d.is_empty()) {
                    ctx.link().send_message(Msg::VerifyPin);
                } else {
                    // Find first empty index and focus it
                    if let Some(first_empty) = self.pin_digits.iter().position(|d| d.is_empty()) {
                        if let Some(input) = self.pin_refs[first_empty].cast::<web_sys::HtmlInputElement>() {
                            let _ = input.focus();
                        }
                    }
                }
                true
            }
            
            Msg::VerifyPin => {
                let pin = self.pin_digits.join("");
                if pin.len() < self.pin_digits.len() {
                    return false;
                }
                
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match verify_pin_api(&pin).await {
                        Ok(success) => {
                            if success {
                                link.send_message(Msg::PinVerificationResult(Ok(true)));
                            } else {
                                link.send_message(Msg::PinVerificationResult(Err("Invalid PIN.".to_string())));
                            }
                        }
                        Err(e) => {
                            link.send_message(Msg::PinVerificationResult(Err(e)));
                        }
                    }
                });
                false
            }
            
            Msg::PinVerificationResult(res) => {
                match res {
                    Ok(true) => {
                        self.is_authenticated = true;
                        self.error_message = None;
                        self.is_lockout = false;
                        self.show_toast(ctx, "Authentication successful", "success");
                        ctx.link().send_message(Msg::RefreshFiles);
                    }
                    Ok(false) => {
                        self.error_message = Some("Invalid PIN".to_string());
                        self.reset_pin_inputs();
                    }
                    Err(e) => {
                        if !e.is_empty() {
                            self.error_message = Some(e.clone());
                            if e.contains("Too many") || e.contains("locked") {
                                self.is_lockout = true;
                            } else {
                                self.reset_pin_inputs();
                            }
                        }
                    }
                }
                true
            }
            
            Msg::Logout => {
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = logout_api().await;
                    link.send_message(Msg::RefreshFiles);
                });
                self.is_authenticated = false;
                self.reset_pin_inputs();
                true
            }
            
            Msg::DragOver(over) => {
                if self.drag_over != over {
                    self.drag_over = over;
                    true
                } else {
                    false
                }
            }
            
            Msg::FilesSelected(files) => {
                self.upload_queue = files;
                self.active_uploads.clear();
                
                if let Some(ref config) = self.config {
                    if config.auto_upload {
                        ctx.link().send_message(Msg::StartUploads);
                    }
                }
                true
            }
            
            Msg::FoldersSelected(files) => {
                self.upload_queue = files;
                self.active_uploads.clear();
                
                if let Some(ref config) = self.config {
                    if config.auto_upload {
                        ctx.link().send_message(Msg::StartUploads);
                    }
                }
                true
            }
            
            Msg::DropProcessed(res) => {
                match res {
                    Ok(new_files) => {
                        self.upload_queue = new_files;
                        self.active_uploads.clear();
                        
                        if let Some(ref config) = self.config {
                            if config.auto_upload {
                                ctx.link().send_message(Msg::StartUploads);
                            }
                        }
                    }
                    Err(e) => {
                        self.show_toast(ctx, &format!("Failed to process drop: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::StartUploads => {
                if self.upload_queue.is_empty() || self.is_uploading {
                    return false;
                }
                
                self.is_uploading = true;
                
                // Initialize progress entries
                for file in &self.upload_queue {
                    let path = get_file_path(file);
                    self.active_uploads.insert(path.clone(), UploadProgress {
                        name: file.name(),
                        path: path.clone(),
                        size: file.size() as u64,
                        uploaded: 0,
                        rate: 0.0,
                        status: "queued".to_string(),
                        error_color: None,
                    });
                }
                
                let link = ctx.link().clone();
                let files = self.upload_queue.clone();
                let batch_id = generate_batch_id();
                let max_retries = self.config.as_ref().map(|c| c.client_max_retries as usize).unwrap_or(5);
                
                wasm_bindgen_futures::spawn_local(async move {
                    for file in files {
                        let path = get_file_path(&file);
                        let size = file.size() as u64;
                        
                        // 1. Initialize file upload
                        link.send_message(Msg::UploadProgressUpdate(
                            path.clone(), 
                            0, 
                            0.0, 
                            "initializing...".to_string(), 
                            None
                        ));
                        
                        let upload_id = match init_upload(&path, size, &batch_id).await {
                            Ok(uid) => uid,
                            Err(e) => {
                                link.send_message(Msg::UploadFailed(path.clone(), e));
                                continue;
                            }
                        };
                        
                        link.send_message(Msg::UploadInit(path.clone(), upload_id.clone()));
                        
                        // 2. Perform chunked uploads
                        let chunk_size = 1024 * 1024; // 1MB chunks
                        let mut position = 0u64;
                        let mut failed = false;
                        
                        let mut last_uploaded_bytes = 0u64;
                        let window_obj = web_sys::window().unwrap();
                        let perf = window_obj.performance().unwrap();
                        let mut last_upload_time = perf.now();
                        
                        if size == 0 {
                            link.send_message(Msg::UploadProgressUpdate(
                                path.clone(), 
                                0, 
                                0.0, 
                                "complete".to_string(), 
                                None
                            ));
                            link.send_message(Msg::UploadCompleted(path.clone()));
                            continue;
                        }
                        
                        while position < size {
                            let start = position;
                            let end = std::cmp::min(position + chunk_size, size);
                            
                            // Slice chunk
                            let blob = match file.slice_with_f64_and_f64(start as f64, end as f64) {
                                Ok(b) => b,
                                Err(e) => {
                                    link.send_message(Msg::UploadFailed(path.clone(), format!("Slice failed: {:?}", e)));
                                    failed = true;
                                    break;
                                }
                            };
                            
                            // Read chunk to Vec<u8>
                            let array_buffer_val = match wasm_bindgen_futures::JsFuture::from(blob.array_buffer()).await {
                                Ok(ab) => ab,
                                Err(e) => {
                                    link.send_message(Msg::UploadFailed(path.clone(), format!("Read buffer failed: {:?}", e)));
                                    failed = true;
                                    break;
                                }
                            };
                            
                            let array_buffer = js_sys::ArrayBuffer::from(array_buffer_val);
                            let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                            let mut chunk_data = vec![0u8; uint8_array.length() as usize];
                            uint8_array.copy_to(&mut chunk_data);
                            
                            // Upload chunk with retry logic
                            let mut chunk_success = false;
                            let mut chunk_error_msg = String::new();
                            
                            for attempt in 0..=max_retries {
                                if attempt > 0 {
                                    link.send_message(Msg::UploadProgressUpdate(
                                        path.clone(),
                                        position,
                                        0.0,
                                        format!("Retrying attempt {}/{}...", attempt, max_retries),
                                        Some("var(--warning-color)".to_string())
                                    ));
                                    
                                    // Exponential backoff delay
                                    let delay = std::cmp::min(1000 * 2_u64.pow(attempt as u32 - 1), 30000);
                                    gloo_timers::future::sleep(std::time::Duration::from_millis(delay)).await;
                                }
                                
                                match upload_chunk(&upload_id, &batch_id, chunk_data.clone()).await {
                                    Ok(_progress) => {
                                        chunk_success = true;
                                        
                                        // Calculate rates
                                        let current_time = perf.now();
                                        let time_diff = (current_time - last_upload_time) / 1000.0; // convert to secs
                                        let bytes_diff = end - last_uploaded_bytes;
                                        
                                        let rate = if time_diff > 0.0 {
                                            bytes_diff as f64 / time_diff
                                        } else {
                                            0.0
                                        };
                                        
                                        position = end;
                                        last_uploaded_bytes = position;
                                        last_upload_time = current_time;
                                        
                                        link.send_message(Msg::UploadProgressUpdate(
                                            path.clone(),
                                            position,
                                            rate,
                                            "uploading...".to_string(),
                                            None
                                        ));
                                        break;
                                    }
                                    Err(err) => {
                                        // Special 404 handler on retry: assume completed
                                        if attempt > 0 && err.contains("404") {
                                            chunk_success = true;
                                            position = size;
                                            link.send_message(Msg::UploadProgressUpdate(
                                                path.clone(),
                                                size,
                                                0.0,
                                                "complete".to_string(),
                                                None
                                            ));
                                            break;
                                        }
                                        chunk_error_msg = err;
                                    }
                                }
                            }
                            
                            if !chunk_success {
                                failed = true;
                                let _ = cancel_upload(&upload_id).await;
                                link.send_message(Msg::UploadFailed(path.clone(), format!("Chunk upload failed: {}", chunk_error_msg)));
                                break;
                            }
                        }
                        
                        if !failed {
                            link.send_message(Msg::UploadCompleted(path.clone()));
                        }
                    }
                });
                true
            }
            
            Msg::UploadInit(path, _upload_id) => {
                if let Some(upload) = self.active_uploads.get_mut(&path) {
                    upload.status = "initializing".to_string();
                }
                true
            }
            
            Msg::UploadProgressUpdate(path, uploaded, rate, status, error_color) => {
                if let Some(upload) = self.active_uploads.get_mut(&path) {
                    upload.uploaded = uploaded;
                    upload.rate = rate;
                    upload.status = status;
                    upload.error_color = error_color;
                }
                true
            }
            
            Msg::UploadCompleted(path) => {
                if let Some(upload) = self.active_uploads.get_mut(&path) {
                    upload.uploaded = upload.size;
                    upload.status = "complete".to_string();
                }
                
                // Show notification and clean queue item
                self.show_toast(ctx, &format!("File uploaded: {}", path.split('/').last().unwrap_or(&path)), "success");
                
                // Check if all uploads complete
                let all_complete = self.active_uploads.values().all(|up| up.status == "complete" || up.status.starts_with("Error"));
                if all_complete {
                    self.is_uploading = false;
                    self.upload_queue.clear();
                    ctx.link().send_message(Msg::RefreshFiles);
                    
                    // Clear inputs
                    if let Some(input) = self.file_input_ref.cast::<web_sys::HtmlInputElement>() {
                        input.set_value("");
                    }
                    if let Some(input) = self.folder_input_ref.cast::<web_sys::HtmlInputElement>() {
                        input.set_value("");
                    }
                }
                true
            }
            
            Msg::UploadFailed(path, err) => {
                if let Some(upload) = self.active_uploads.get_mut(&path) {
                    upload.status = format!("Error: {}", err);
                    upload.error_color = Some("var(--danger-color)".to_string());
                }
                
                self.show_toast(ctx, &format!("Upload failed for {}: {}", path.split('/').last().unwrap_or(&path), err), "error");
                
                let all_complete = self.active_uploads.values().all(|up| up.status == "complete" || up.status.starts_with("Error"));
                if all_complete {
                    self.is_uploading = false;
                    self.upload_queue.clear();
                    ctx.link().send_message(Msg::RefreshFiles);
                }
                true
            }
            
            Msg::LoadFileList(res) => {
                match res {
                    Ok(data) => {
                        self.uploaded_files = Some(data);
                    }
                    Err(e) => {
                        self.show_toast(ctx, &format!("Failed to load files: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::RefreshFiles => {
                if !self.is_authenticated {
                    return false;
                }
                
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match fetch_files().await {
                        Ok(data) => link.send_message(Msg::LoadFileList(Ok(data))),
                        Err(e) => link.send_message(Msg::LoadFileList(Err(e))),
                    }
                });
                false
            }
            
            Msg::DeleteFile(path) => {
                let name = path.split('/').last().unwrap_or(&path).to_string();
                let window = web_sys::window().unwrap();
                let confirm_msg = format!("Are you sure you want to delete \"{}\"?", name);
                
                if window.confirm_with_message(&confirm_msg).unwrap_or(false) {
                    let link = ctx.link().clone();
                    let path_c = path.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match delete_file_api(&path_c).await {
                            Ok(_) => link.send_message(Msg::DeleteResult(Ok(path_c))),
                            Err(e) => link.send_message(Msg::DeleteResult(Err(e))),
                        }
                    });
                }
                false
            }
            
            Msg::DeleteResult(res) => {
                match res {
                    Ok(path) => {
                        let name = path.split('/').last().unwrap_or(&path).to_string();
                        self.show_toast(ctx, &format!("Deleted: {}", name), "success");
                        ctx.link().send_message(Msg::RefreshFiles);
                    }
                    Err(e) => {
                        self.show_toast(ctx, &format!("Delete failed: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::StartRename(path, current_name) => {
                self.rename_target = Some(RenameData {
                    item_path: path,
                    current_name: current_name.clone(),
                });
                self.rename_input_val = current_name;
                true
            }
            
            Msg::CancelRename => {
                self.rename_target = None;
                self.rename_input_val.clear();
                true
            }
            
            Msg::RenameInputChanged(val) => {
                self.rename_input_val = val;
                true
            }
            
            Msg::ConfirmRename => {
                if self.rename_input_val.trim().is_empty() {
                    return false;
                }
                
                if let Some(target) = self.rename_target.take() {
                    let new_name = self.rename_input_val.trim().to_string();
                    let link = ctx.link().clone();
                    
                    wasm_bindgen_futures::spawn_local(async move {
                        match rename_file_api(&target.item_path, &new_name).await {
                            Ok(_) => link.send_message(Msg::RenameResult(Ok(new_name))),
                            Err(e) => link.send_message(Msg::RenameResult(Err(e))),
                        }
                    });
                }
                false
            }
            
            Msg::RenameResult(res) => {
                self.rename_target = None;
                self.rename_input_val.clear();
                
                match res {
                    Ok(new_name) => {
                        self.show_toast(ctx, &format!("Renamed to: {}", new_name), "success");
                        ctx.link().send_message(Msg::RefreshFiles);
                    }
                    Err(e) => {
                        self.show_toast(ctx, &format!("Rename failed: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::AddToast(message, toast_type) => {
                let id = self.next_toast_id;
                self.next_toast_id += 1;
                
                self.toasts.push(Toast {
                    id,
                    message,
                    toast_type,
                });
                
                let link = ctx.link().clone();
                let timeout = gloo_timers::callback::Timeout::new(3000, move || {
                    link.send_message(Msg::RemoveToast(id));
                });
                
                self.toast_timeouts.insert(id, timeout);
                true
            }
            
            Msg::RemoveToast(id) => {
                self.toasts.retain(|t| t.id != id);
                self.toast_timeouts.remove(&id);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let site_title = self.config.as_ref().map(|c| c.site_title.as_str()).unwrap_or("RustDrop");
        
        let on_dragover = ctx.link().callback(|e: DragEvent| {
            e.prevent_default();
            Msg::DragOver(true)
        });
        
        let on_dragenter = ctx.link().callback(|e: DragEvent| {
            e.prevent_default();
            Msg::DragOver(true)
        });
        
        let on_dragleave = ctx.link().callback(|e: DragEvent| {
            e.prevent_default();
            Msg::DragOver(false)
        });
        
        let link_c = ctx.link().clone();
        let on_drop = ctx.link().callback(move |e: DragEvent| {
            e.prevent_default();
            e.stop_propagation();
            
            let data_transfer = e.data_transfer().unwrap();
            let link = link_c.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                match get_files_from_data_transfer(&data_transfer).await {
                    Ok(arr) => {
                        let mut files = Vec::new();
                        for i in 0..arr.length() {
                            let val = arr.get(i);
                            let file: web_sys::File = val.unchecked_into();
                            files.push(file);
                        }
                        link.send_message(Msg::DropProcessed(Ok(files)));
                    }
                    Err(err) => {
                        link.send_message(Msg::DropProcessed(Err(format!("{:?}", err))));
                    }
                }
            });
            
            Msg::DragOver(false)
        });

        let on_file_input_change = ctx.link().callback(|e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let file_list = input.files().unwrap();
            let mut files = Vec::new();
            for i in 0..file_list.length() {
                if let Some(file) = file_list.item(i) {
                    files.push(file);
                }
            }
            Msg::FilesSelected(files)
        });

        let on_folder_input_change = ctx.link().callback(|e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let file_list = input.files().unwrap();
            let mut files = Vec::new();
            for i in 0..file_list.length() {
                if let Some(file) = file_list.item(i) {
                    files.push(file);
                }
            }
            Msg::FoldersSelected(files)
        });

        html! {
            <div class="container">
                // Theme Toggle
                <button class="theme-toggle" onclick={ctx.link().callback(|_| Msg::ToggleTheme)} aria-label="Toggle theme">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      class="theme-toggle-icon"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      {if self.theme == "light" {
                          html! {
                              <path class="moon" d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                          }
                      } else {
                          html! {
                              <>
                                  <circle class="sun" cx="12" cy="12" r="5" />
                                  <line class="sun" x1="12" y1="1" x2="12" y2="3" />
                                  <line class="sun" x1="12" y1="21" x2="12" y2="23" />
                                  <line class="sun" x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                                  <line class="sun" x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                                  <line class="sun" x1="1" y1="12" x2="3" y2="12" />
                                  <line class="sun" x1="21" y1="12" x2="23" y2="12" />
                                  <line class="sun" x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                                  <line class="sun" x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
                              </>
                          }
                      }}
                    </svg>
                </button>
                
                <h1>{site_title}</h1>
                
                {if !self.is_authenticated {
                    // PIN Authentication Form
                    html! {
                        <div class="login-container">
                            <div class="pin-header">
                                <h2>{"Enter PIN"}</h2>
                            </div>
                            <form id="pin-form" onsubmit={ctx.link().callback(|e: SubmitEvent| { e.prevent_default(); Msg::VerifyPin })}>
                                {for self.pin_refs.iter().enumerate().map(|(idx, r)| {
                                    let pin_digits = self.pin_digits.clone();
                                    html! {
                                        <input
                                            ref={r.clone()}
                                            type="password"
                                            class={classes!(
                                                "pin-digit",
                                                if self.is_lockout { Some("locked") } else { None },
                                                if !pin_digits[idx].is_empty() { Some("filled") } else { None }
                                            )}
                                            maxlength="1"
                                            pattern="[0-9]"
                                            inputmode="numeric"
                                            autocomplete="off"
                                            required=true
                                            disabled={self.is_lockout}
                                            value={self.pin_digits[idx].clone()}
                                            oninput={ctx.link().callback(move |e: InputEvent| {
                                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                Msg::PinDigitInput(idx, input.value())
                                            })}
                                            onkeydown={ctx.link().callback(move |e: KeyboardEvent| {
                                                if e.key() == "Backspace" {
                                                    Msg::PinBackspace(idx)
                                                } else {
                                                    Msg::Nothing
                                                }
                                            })}
                                            onpaste={ctx.link().callback(move |e: Event| {
                                                let clipboard_event: web_sys::ClipboardEvent = e.unchecked_into();
                                                if let Some(dt) = clipboard_event.clipboard_data() {
                                                    if let Ok(text) = dt.get_data("text") {
                                                        Msg::PinPaste(text)
                                                    } else {
                                                        Msg::Nothing
                                                    }
                                                } else {
                                                    Msg::Nothing
                                                }
                                            })}
                                        />
                                    }
                                })}
                            </form>
                            {if let Some(ref err) = self.error_message {
                                html! { <p id="pin-error" class="error-message">{err}</p> }
                            } else {
                                html! {}
                            }}
                        </div>
                    }
                } else {
                    // Upload & File Explorer Interface
                    html! {
                        <>
                            <div 
                                class={classes!("upload-container", self.drag_over.then(|| "highlight"))}
                                ondragover={on_dragover}
                                ondragenter={on_dragenter}
                                ondragleave={on_dragleave}
                                ondrop={on_drop}
                                onclick={ctx.link().callback(|_| Msg::Nothing)}
                            >
                                <div class="upload-content">
                                  <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    width="50"
                                    height="50"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                  >
                                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                                    <polyline points="17 8 12 3 7 8" />
                                    <line x1="12" y1="3" x2="12" y2="15" />
                                  </svg>
                                  <p>{"Drag and drop files or folders here"}<br />{"or"}</p>
                                  
                                  <input 
                                      ref={self.file_input_ref.clone()}
                                      type="file" 
                                      id="fileInput" 
                                      multiple=true 
                                      hidden=true 
                                      onchange={on_file_input_change}
                                  />
                                  <input 
                                      ref={self.folder_input_ref.clone()}
                                      type="file" 
                                      id="folderInput" 
                                      webkitdirectory=true
                                      multiple=true 
                                      hidden=true 
                                      onchange={on_folder_input_change}
                                  />
                                  
                                  <div class="button-group">
                                    <button onclick={
                                        let r = self.file_input_ref.clone();
                                        Callback::from(move |_| {
                                            if let Some(input) = r.cast::<web_sys::HtmlInputElement>() {
                                                input.click();
                                            }
                                        })
                                    }>{"Browse Files"}</button>
                                    
                                    <button onclick={
                                        let r = self.folder_input_ref.clone();
                                        Callback::from(move |_| {
                                            if let Some(input) = r.cast::<web_sys::HtmlInputElement>() {
                                                input.click();
                                            }
                                        })
                                    }>{"Browse Folders"}</button>
                                  </div>
                                </div>
                            </div>
                            
                            // Selected Files Queued
                            <div id="fileList" class="file-list">
                                {if !self.is_uploading && !self.upload_queue.is_empty() {
                                    html! {
                                        <>
                                            {for self.upload_queue.iter().map(|file| {
                                                let path = get_file_path(file);
                                                html! {
                                                    <div class="file-item">
                                                        {format!("📄 {} ({})", path, format_file_size(file.size() as u64))}
                                                    </div>
                                                }
                                            })}
                                        </>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                            
                            // Upload progress bars
                            <div id="uploadProgress">
                                {for self.active_uploads.values().map(|upload| {
                                    let percent = if upload.size > 0 {
                                        (upload.uploaded as f64 / upload.size as f64) * 100.0
                                    } else {
                                        100.0
                                    };
                                    
                                    // Speed text
                                    let rate_text = if upload.rate > 0.0 {
                                        let units = ["B/s", "KB/s", "MB/s", "GB/s"];
                                        let mut i = 0;
                                        let mut r = upload.rate;
                                        while r >= 1024.0 && i < units.len() - 1 {
                                            r /= 1024.0;
                                            i += 1;
                                        }
                                        format!("{:.1} {}", r, units[i])
                                    } else {
                                        "0.0 B/s".to_string()
                                    };
                                    
                                    let details_text = format!("{} of {} ({:.1}%)", format_file_size(upload.uploaded), format_file_size(upload.size), percent);
                                    let is_complete = upload.status == "complete";
                                    
                                    html! {
                                        <div class="progress-container" style={if is_complete { "display: none;" } else { "" }}>
                                            <div class="progress-label">{&upload.path}</div>
                                            <div class="progress">
                                                <div class="progress-bar" style={format!("width: {:.1}%", percent)}></div>
                                            </div>
                                            <div class="progress-status">
                                                <div class="progress-info" style={upload.error_color.as_ref().map(|c| format!("color: {}", c)).unwrap_or_default()}>
                                                    {if is_complete { "complete".to_string() } else { format!("{} · {}", rate_text, upload.status) }}
                                                </div>
                                                <div class="progress-details">{details_text}</div>
                                            </div>
                                        </div>
                                    }
                                })}
                            </div>
                            
                            // Manual Upload Button (if auto_upload is disabled)
                            {if !self.is_uploading && !self.upload_queue.is_empty() && self.config.as_ref().map(|c| !c.auto_upload).unwrap_or(true) {
                                html! {
                                    <button 
                                        id="uploadButton" 
                                        class="upload-button" 
                                        onclick={ctx.link().callback(|_| Msg::StartUploads)}
                                    >
                                        {"Upload Files"}
                                    </button>
                                }
                            } else {
                                html! {}
                            }}
                            
                            // Remote File Listing Explorer
                            {if self.config.as_ref().map(|c| c.show_file_list).unwrap_or(false) {
                                html! {
                                    <div id="uploadedFilesList" class="uploaded-files-section">
                                        <div class="uploaded-files-header">
                                            <h2>{"Uploaded Files"}</h2>
                                            <div class="uploaded-files-stats">
                                                <span id="totalFiles">
                                                    {format!("{} file{}", 
                                                        self.uploaded_files.as_ref().map(|f| f.total_files).unwrap_or(0),
                                                        if self.uploaded_files.as_ref().map(|f| f.total_files).unwrap_or(0) != 1 { "s" } else { "" }
                                                    )}
                                                </span>
                                                {" • "}
                                                <span id="totalSize">
                                                    {self.uploaded_files.as_ref().map(|f| f.formatted_total_size.clone()).unwrap_or_else(|| "0 Bytes".to_string())}
                                                </span>
                                                <button id="refreshFilesBtn" class="refresh-btn" onclick={ctx.link().callback(|_| Msg::RefreshFiles)}>
                                                    {"🔄 Refresh"}
                                                </button>
                                                {if self.config.as_ref().map(|c| c.pin_required).unwrap_or(false) {
                                                    html! {
                                                        <button class="refresh-btn" style="background-color: var(--danger-color);" onclick={ctx.link().callback(|_| Msg::Logout)}>
                                                            {"Logout"}
                                                        </button>
                                                    }
                                                } else {
                                                    html! {}
                                                }}
                                            </div>
                                        </div>
                                        <div id="uploadedFilesContent" class="uploaded-files-content">
                                            {match &self.uploaded_files {
                                                None => html! { <div class="loading-message">{"Loading files..."}</div> },
                                                Some(data) => {
                                                    if data.items.is_empty() {
                                                        html! { <div class="empty-message">{"No files uploaded yet"}</div> }
                                                    } else {
                                                        render_file_items(&data.items, 0, ctx.link().clone())
                                                    }
                                                }
                                            }}
                                        </div>
                                    </div>
                                }
                            } else {
                                // Logout button when file list is hidden but PIN is required
                                if self.config.as_ref().map(|c| c.pin_required).unwrap_or(false) {
                                    html! {
                                        <div class="uploaded-files-stats" style="justify-content: center; margin-top: 20px;">
                                            <button class="refresh-btn" style="background-color: var(--danger-color); padding: 8px 16px;" onclick={ctx.link().callback(|_| Msg::Logout)}>
                                                {"Logout"}
                                            </button>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }}
                        </>
                    }
                }}

                // Rename Modal Dialog
                {if self.rename_target.is_some() {
                    html! {
                        <div id="renameModal" class="rename-modal show" onclick={ctx.link().callback(|e: MouseEvent| {
                                                            let target_el: web_sys::HtmlElement = e.target_unchecked_into();
                                                            if target_el.id() == "renameModal" {
                                Msg::CancelRename
                            } else {
                                Msg::Nothing
                            }
                        })}>
                            <div class="rename-modal-content">
                                <h3>{"Rename Item"}</h3>
                                <input 
                                    type="text" 
                                    id="renameInput" 
                                    class="rename-input" 
                                    value={self.rename_input_val.clone()} 
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        Msg::RenameInputChanged(input.value())
                                    })}
                                    onkeydown={ctx.link().callback(|e: KeyboardEvent| {
                                        if e.key() == "Enter" {
                                            Msg::ConfirmRename
                                        } else if e.key() == "Escape" {
                                            Msg::CancelRename
                                        } else {
                                            Msg::Nothing
                                        }
                                    })}
                                />
                                <div class="rename-actions">
                                    <button class="modal-btn modal-btn-cancel" onclick={ctx.link().callback(|_| Msg::CancelRename)}>
                                        {"Cancel"}
                                    </button>
                                    <button class="modal-btn modal-btn-confirm" onclick={ctx.link().callback(|_| Msg::ConfirmRename)}>
                                        {"Rename"}
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}

                // Toast Notification Overlay
                <div class="toast-container">
                    {for self.toasts.iter().map(|toast| {
                        html! {
                            <div key={toast.id} class={classes!("toast", format!("toast-{}", toast.toast_type))}>
                                {&toast.message}
                            </div>
                        }
                    })}
                </div>
            </div>
        }
    }
}

// Render helper for hierarchical recursive file list
fn render_file_items(items: &[FileItem], level: usize, link: Scope<App>) -> Html {
    html! {
        <>
            {for items.iter().map(|item| {
                match item {
                    FileItem::File { name, path, size: _, formatted_size, upload_date, extension: _ } => {
                        let path_c = path.clone();
                        let name_c = name.clone();
                        let path_d = path.clone();
                        let link_c = link.clone();
                        let link_d = link.clone();
                        
                        html! {
                            <div class="uploaded-file-item" style={format!("margin-left: {}px", level * 20)}>
                                <div class="uploaded-file-info">
                                    <div class="uploaded-file-name">{"📄 "}{name}</div>
                                    <div class="uploaded-file-details">
                                        {format!("{} • {}", formatted_size, format_date(upload_date))}
                                    </div>
                                </div>
                                <div class="uploaded-file-actions">
                                    <button class="action-btn download-btn" onclick={
                                        let p = path_c.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            download_file(&p);
                                        })
                                    }>
                                        {"Download"}
                                    </button>
                                    <button class="action-btn rename-btn" onclick={
                                        let p = path_d.clone();
                                        let n = name_c.clone();
                                        let l = link_c.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            l.send_message(Msg::StartRename(p.clone(), n.clone()));
                                        })
                                    }>
                                        {"Rename"}
                                    </button>
                                    <button class="action-btn delete-btn" onclick={
                                        let p = path_c.clone();
                                        let l = link_d.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            l.send_message(Msg::DeleteFile(p.clone()));
                                        })
                                    }>
                                        {"Delete"}
                                    </button>
                                </div>
                            </div>
                        }
                    }
                    FileItem::Directory { name, path, size: _, formatted_size, children, upload_date: _ } => {
                        let name_c = name.clone();
                        let path_c = path.clone();
                        let path_d = path.clone();
                        let file_count = count_files_in_dir(children);
                        let link_c = link.clone();
                        let link_d = link.clone();
                        let link_e = link.clone();
                        
                        html! {
                            <>
                                <div class="uploaded-file-item directory-item" style={format!("margin-left: {}px", level * 20)}>
                                    <div class="uploaded-file-info">
                                        <div class="uploaded-file-name">{"📁 "}{name}</div>
                                        <div class="uploaded-file-details">
                                            {format!("{} • {} file{}", formatted_size, file_count, if file_count != 1 { "s" } else { "" })}
                                        </div>
                                    </div>
                                    <div class="uploaded-file-actions">
                                        <button class="action-btn rename-btn" onclick={
                                            let p = path_c.clone();
                                            let n = name_c.clone();
                                            let l = link_c.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.stop_propagation();
                                                l.send_message(Msg::StartRename(p.clone(), n.clone()));
                                            })
                                        }>
                                            {"Rename"}
                                        </button>
                                        <button class="action-btn delete-btn" onclick={
                                            let p = path_d.clone();
                                            let l = link_d.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.stop_propagation();
                                                l.send_message(Msg::DeleteFile(p.clone()));
                                            })
                                        }>
                                            {"Delete"}
                                        </button>
                                    </div>
                                </div>
                                {if !children.is_empty() {
                                    html! {
                                        <div class="directory-children">
                                            {render_file_items(children, level + 1, link_e.clone())}
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </>
                        }
                    }
                }
            })}
        </>
    }
}

fn count_files_in_dir(children: &[FileItem]) -> usize {
    children.iter().map(|child| {
        match child {
            FileItem::File { .. } => 1,
            FileItem::Directory { children: sub_children, .. } => count_files_in_dir(sub_children),
        }
    }).sum()
}

// App helper functions
impl App {
    fn show_toast(&mut self, ctx: &Context<Self>, message: &str, toast_type: &str) {
        ctx.link().send_message(Msg::AddToast(message.to_string(), toast_type.to_string()));
    }
    
    fn reset_pin_inputs(&mut self) {
        for digit in &mut self.pin_digits {
            *digit = "".to_string();
        }
        for r in &self.pin_refs {
            if let Some(input) = r.cast::<web_sys::HtmlInputElement>() {
                input.set_value("");
            }
        }
        // Focus first PIN input
        if !self.pin_refs.is_empty() {
            if let Some(input) = self.pin_refs[0].cast::<web_sys::HtmlInputElement>() {
                let _ = input.focus();
            }
        }
    }
}

// Theme utilities
fn get_saved_theme() -> String {
    let window = web_sys::window().unwrap();
    let local_storage = window.local_storage().unwrap().unwrap();
    if let Ok(Some(theme)) = local_storage.get_item("theme") {
        theme
    } else {
        let media_query = window.match_media("(prefers-color-scheme: dark)").unwrap().unwrap();
        if media_query.matches() {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    }
}

fn save_theme(theme: &str) {
    let window = web_sys::window().unwrap();
    let local_storage = window.local_storage().unwrap().unwrap();
    let _ = local_storage.set_item("theme", theme);
}

fn set_theme_attribute(theme: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let html = document.document_element().unwrap();
    let _ = html.set_attribute("data-theme", theme);
}

// Formatting utilities
fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 Bytes".to_string();
    }
    let k = 1024.0;
    let sizes = ["Bytes", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(k).floor() as usize;
    let val = bytes as f64 / k.powi(i as i32);
    format!("{:.2} {}", val, sizes[i])
}

fn format_date(date_str: &str) -> String {
    if date_str.len() >= 10 {
        date_str[0..10].to_string()
    } else {
        date_str.to_string()
    }
}

// Generate batch ID
fn generate_batch_id() -> String {
    let window = web_sys::window().unwrap();
    let now = window.performance().unwrap().now() as u64;
    let random: u32 = js_sys::Math::random().to_bits() as u32;
    format!("{}-{:x}", now, random)
}

// API Functions
async fn fetch_config() -> Result<FrontendConfig, String> {
    let res = gloo_net::http::Request::get("/api/auth/config")
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("Failed to fetch config: HTTP {}", res.status()));
    }
    
    let config: FrontendConfig = res.json().await.map_err(|e| e.to_string())?;
    Ok(config)
}

async fn check_already_authenticated() -> bool {
    let res = gloo_net::http::Request::get("/api/files")
        .send()
        .await;
        
    match res {
        Ok(response) => response.status() == 200,
        Err(_) => false,
    }
}

async fn verify_pin_api(pin: &str) -> Result<bool, String> {
    let res = gloo_net::http::Request::post("/api/auth/verify-pin")
        .json(&serde_json::json!({ "pin": pin }))
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if res.status() == 200 {
        Ok(true)
    } else if res.status() == 429 {
        Err("Too many PIN verification attempts. Please wait before trying again.".to_string())
    } else {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Authentication failed");
        Err(err_msg.to_string())
    }
}

async fn logout_api() -> Result<(), String> {
    let _ = gloo_net::http::Request::post("/api/auth/logout")
        .send()
        .await;
    Ok(())
}

async fn fetch_files() -> Result<FileListResponse, String> {
    let res = gloo_net::http::Request::get("/api/files")
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("HTTP {}", res.status()));
    }
    
    let list: FileListResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(list)
}

async fn delete_file_api(file_path: &str) -> Result<(), String> {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/delete/{}", encoded_path);
    
    let res = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to delete item");
        return Err(err_msg.to_string());
    }
    
    Ok(())
}

async fn rename_file_api(file_path: &str, new_name: &str) -> Result<(), String> {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/rename/{}", encoded_path);
    
    let res = gloo_net::http::Request::put(&url)
        .json(&serde_json::json!({ "newName": new_name }))
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to rename item");
        return Err(err_msg.to_string());
    }
    
    Ok(())
}

fn download_file(file_path: &str) {
    let encoded_path = encode_path(file_path);
    let url = format!("/api/files/download/{}", encoded_path);
    let window = web_sys::window().unwrap();
    let _ = window.open_with_url_and_target(&url, "_blank");
}

fn get_file_path(file: &web_sys::File) -> String {
    let path = js_sys::Reflect::get(file, &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if path.is_empty() {
        file.name()
    } else {
        path
    }
}

fn encode_path(file_path: &str) -> String {
    file_path
        .split('/')
        .map(|part| {
            js_sys::encode_uri_component(part)
                .as_string()
                .unwrap_or_else(|| part.to_string())
        })
        .collect::<Vec<String>>()
        .join("/")
}

async fn init_upload(filename: &str, file_size: u64, batch_id: &str) -> Result<String, String> {
    let url = "/api/upload/init";
    let body = serde_json::json!({
        "filename": filename.replace('\\', "/"),
        "fileSize": file_size
    });
    
    let res = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/json")
        .header("X-Batch-ID", batch_id)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        let err_json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let err_msg = err_json.get("details")
            .or_else(|| err_json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Upload initialization failed");
        return Err(err_msg.to_string());
    }
    
    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let upload_id = data.get("uploadId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing uploadId in response".to_string())?;
        
    Ok(upload_id.to_string())
}

async fn upload_chunk(upload_id: &str, batch_id: &str, data: Vec<u8>) -> Result<f64, String> {
    let url = format!("/api/upload/chunk/{}", upload_id);
    
    let res = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/octet-stream")
        .header("X-Batch-ID", batch_id)
        .body(data)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.ok() {
        return Err(format!("HTTP {} {}", res.status(), res.status_text()));
    }
    
    let data_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let progress = data_json.get("progress")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
        
    Ok(progress)
}

async fn cancel_upload(upload_id: &str) -> Result<(), String> {
    let url = format!("/api/upload/cancel/{}", upload_id);
    let _ = gloo_net::http::Request::post(&url)
        .send()
        .await;
    Ok(())
}

fn main() {
    yew::Renderer::<App>::new().render();
}

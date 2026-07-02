use yew::prelude::*;

use crate::app::App;
use crate::app::upload_task::perform_file_upload;
use crate::types::{Msg, UploadProgress};
use crate::utils::{generate_batch_id, get_file_path};

impl App {
    pub fn update_upload(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::DragOver(over) => {
                if self.drag_over != over {
                    self.drag_over = over;
                    true
                } else {
                    false
                }
            }

            Msg::FilesSelected(files) => {
                self.upload_queue = self.validate_and_filter_files(ctx, files);
                self.active_uploads.clear();

                if let Some(ref config) = self.config
                    && config.auto_upload
                {
                    ctx.link().send_message(Msg::StartUploads);
                }
                true
            }

            Msg::FoldersSelected(files) => {
                self.upload_queue = self.validate_and_filter_files(ctx, files);
                self.active_uploads.clear();

                if let Some(ref config) = self.config
                    && config.auto_upload
                {
                    ctx.link().send_message(Msg::StartUploads);
                }
                true
            }

            Msg::DropProcessed(res) => {
                match res {
                    Ok(new_files) => {
                        self.upload_queue = self.validate_and_filter_files(ctx, new_files);
                        self.active_uploads.clear();

                        if let Some(ref config) = self.config
                            && config.auto_upload
                        {
                            ctx.link().send_message(Msg::StartUploads);
                        }
                    }
                    Err(e) => {
                        let translations = crate::i18n::get_translations(self.language);
                        self.show_toast(
                            ctx,
                            &format!("{}{}", translations.failed_process_drop_prefix, e),
                            "error",
                        );
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
                    self.active_uploads.insert(
                        path.clone(),
                        UploadProgress {
                            name: file.name(),
                            path: path.clone(),
                            size: file.size() as u64,
                            uploaded: 0,
                            rate: 0.0,
                            status: "queued".to_string(),
                            error_color: None,
                        },
                    );
                }

                let link = ctx.link().clone();
                let files = self.upload_queue.clone();
                let batch_id = generate_batch_id();
                let max_retries = self
                    .config
                    .as_ref()
                    .map(|c| c.client_max_retries as usize)
                    .unwrap_or(5);

                wasm_bindgen_futures::spawn_local(async move {
                    for file in files {
                        perform_file_upload(file, batch_id.clone(), max_retries, link.clone())
                            .await;
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
                let translations = crate::i18n::get_translations(self.language);
                self.show_toast(
                    ctx,
                    &format!(
                        "{}{}",
                        translations.file_uploaded_prefix,
                        path.split('/').next_back().unwrap_or(&path)
                    ),
                    "success",
                );

                // Check if all uploads complete
                let all_complete = self
                    .active_uploads
                    .values()
                    .all(|up| up.status == "complete" || up.status.starts_with("Error"));
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

                let translations = crate::i18n::get_translations(self.language);
                self.show_toast(
                    ctx,
                    &format!(
                        "{}{}: {}",
                        translations.upload_failed_prefix,
                        path.split('/').next_back().unwrap_or(&path),
                        err
                    ),
                    "error",
                );

                let all_complete = self
                    .active_uploads
                    .values()
                    .all(|up| up.status == "complete" || up.status.starts_with("Error"));
                if all_complete {
                    self.is_uploading = false;
                    self.upload_queue.clear();
                    ctx.link().send_message(Msg::RefreshFiles);
                }
                true
            }
            _ => false,
        }
    }

    fn validate_and_filter_files(&mut self, ctx: &Context<Self>, files: Vec<web_sys::File>) -> Vec<web_sys::File> {
        let max_size = self.config.as_ref().map(|c| c.max_file_size).unwrap_or(20 * 1024 * 1024 * 1024);
        let mut valid_files = Vec::new();
        let mut too_large_files = Vec::new();
        for file in files {
            if file.size() as u64 > max_size {
                too_large_files.push(file.name());
            } else {
                valid_files.push(file);
            }
        }
        if !too_large_files.is_empty() {
            let limit_str = if max_size >= 1024 * 1024 * 1024 {
                format!("{} GB", max_size / (1024 * 1024 * 1024))
            } else {
                format!("{} MB", max_size / (1024 * 1024))
            };
            let translations = crate::i18n::get_translations(self.language);
            let msg = format!(
                "{} {} (max {}): {}",
                translations.file_too_large_prefix,
                too_large_files.len(),
                limit_str,
                too_large_files.join(", ")
            );
            self.show_toast(ctx, &msg, "error");
        }
        valid_files
    }
}

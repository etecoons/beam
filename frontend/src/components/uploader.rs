use wasm_bindgen::JsCast;
use yew::prelude::*;

use crate::app::App;
use crate::js_api::get_files_from_data_transfer;
use crate::types::Msg;
use crate::utils::{format_file_size, get_file_path};

impl App {
    pub fn render_uploader(&self, ctx: &Context<Self>) -> Html {
        let translations = crate::i18n::get_translations(self.language);

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
            <div class="login-container uploader-container">
                <div class="login-box uploader-box">
                    <div
                        class={classes!("upload-container", self.drag_over.then_some("highlight"))}
                        ondragover={on_dragover}
                        ondragenter={on_dragenter}
                        ondragleave={on_dragleave}
                        ondrop={on_drop}
                        onclick={ctx.link().callback(|_| Msg::Nothing)}
                    >
                        <div class="upload-content">
                          <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="36"
                            height="36"
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
                          <p>{translations.drag_drop_prompt}<br />{translations.or}</p>

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
                            }>{translations.browse_files}</button>
                          </div>
                        </div>
                    </div>

                    // Selected Files Queued
                    {if !self.is_uploading && !self.upload_queue.is_empty() {
                        html! {
                            <div id="fileList" class="file-list">
                                {for self.upload_queue.iter().map(|file| {
                                    let path = get_file_path(file);
                                    html! {
                                        <div class="file-item">
                                            {format!("📄 {} ({})", path, format_file_size(file.size() as u64))}
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    } else {
                        html! {}
                    }}

                    // Upload progress bars
                    {if !self.active_uploads.is_empty() {
                        html! {
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

                                    let details_text = format!("{} {} {} ({:.1}%)", format_file_size(upload.uploaded), translations.of, format_file_size(upload.size), percent);
                                    let is_complete = upload.status == "complete";

                                    html! {
                                        <div class="progress-container" style={if is_complete { "display: none;" } else { "" }}>
                                            <div class="progress-label">{&upload.path}</div>
                                            <div class="progress">
                                                <div class="progress-bar" style={format!("width: {:.1}%", percent)}></div>
                                            </div>
                                            <div class="progress-status">
                                                <div class="progress-info" style={upload.error_color.as_ref().map(|c| format!("color: {}", c)).unwrap_or_default()}>
                                                    {if is_complete {
                                                        translations.complete.to_string()
                                                    } else {
                                                        let display_status = match upload.status.as_str() {
                                                            "queued" => translations.queued.to_string(),
                                                            "initializing" => translations.initializing.to_string(),
                                                            s if s.starts_with("Error:") => {
                                                                format!("{}: {}", translations.error, s.strip_prefix("Error:").unwrap_or(s).trim())
                                                            }
                                                            s => s.to_string(),
                                                        };
                                                        format!("{} · {}", rate_text, display_status)
                                                    }}
                                                </div>
                                                <div class="progress-details">{details_text}</div>
                                            </div>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    } else {
                        html! {}
                    }}

                    // Manual Upload Button (if auto_upload is disabled)
                    {if !self.is_uploading && !self.upload_queue.is_empty() && self.config.as_ref().map(|c| !c.auto_upload).unwrap_or(true) {
                        html! {
                            <button
                                id="uploadButton"
                                class="upload-button"
                                onclick={ctx.link().callback(|_| Msg::StartUploads)}
                            >
                                {translations.upload_files_btn}
                            </button>
                        }
                    } else {
                        html! {}
                    }}
                </div>
            </div>
        }
    }
}

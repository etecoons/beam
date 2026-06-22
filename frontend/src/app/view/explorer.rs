use yew::prelude::*;
use yew::html::Scope;

use crate::app::App;
use crate::types::{Msg, FileItem};
use crate::api::download_file;

impl App {
    pub fn render_explorer(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="uploadedFilesList" class="uploaded-files-section" style="padding: 0; background: transparent; box-shadow: none; margin: 0;">
                <div id="uploadedFilesContent" class="uploaded-files-content">
                    {match &self.uploaded_files {
                        None => html! { <div class="loading-message">{"Loading files..."}</div> },
                        Some(data) => {
                            if data.items.is_empty() {
                                html! { <div class="empty-message">{"No files uploaded yet"}</div> }
                            } else {
                                let flat_items = flatten_files(&data.items);
                                if flat_items.is_empty() {
                                    html! { <div class="empty-message">{"No files uploaded yet"}</div> }
                                } else {
                                    render_file_items(&flat_items, 0, ctx.link().clone())
                                }
                            }
                        }
                    }}
                </div>
            </div>
        }
    }
}

fn flatten_files(items: &[FileItem]) -> Vec<FileItem> {
    let mut files = Vec::new();
    for item in items {
        match item {
            FileItem::File { .. } => {
                files.push(item.clone());
            }
            FileItem::Directory { children, .. } => {
                files.extend(flatten_files(children));
            }
        }
    }
    files
}

// Render helper for flat file list
fn render_file_items(items: &[FileItem], _level: usize, link: Scope<App>) -> Html {
    html! {
        <>
            {for items.iter().map(|item| {
                match item {
                    FileItem::File { name, path, size: _, formatted_size, upload_date: _, extension: _ } => {
                        let path_c = path.clone();
                        let path_s = path.clone();
                        let link_d = link.clone();
                        let link_s = link.clone();
                        
                        html! {
                            <div class="uploaded-file-item">
                                <div class="uploaded-file-name" style="word-break: break-all;">
                                    {"📄 "}{name}
                                </div>
                                <div class="uploaded-file-size">{formatted_size}</div>
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
                                    <button class="action-btn share-btn" onclick={
                                        let p = path_s.clone();
                                        let l = link_s.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            let window = web_sys::window().unwrap();
                                            let origin = window.location().origin().unwrap_or_default();
                                            let encoded_path = crate::utils::encode_path(&p);
                                            let full_url = format!("{}/api/files/download/{}", origin, encoded_path);
                                            
                                            if crate::js_api::copy_text_to_clipboard(&full_url) {
                                                l.send_message(Msg::AddToast("Download link copied!".to_string(), "success".to_string()));
                                            } else {
                                                l.send_message(Msg::AddToast("Failed to copy link".to_string(), "error".to_string()));
                                            }
                                        })
                                    }>
                                        {"Copy Link"}
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
                    _ => html! {}
                }
            })}
        </>
    }
}


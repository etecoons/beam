use yew::prelude::*;
use yew::html::Scope;

use crate::app::App;
use crate::types::{Msg, FileItem};
use crate::utils::format_date;
use crate::api::download_file;

impl App {
    pub fn render_explorer(&self, ctx: &Context<Self>) -> Html {
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
                        let path_s = path.clone();
                        let link_c = link.clone();
                        let link_d = link.clone();
                        let link_s = link.clone();
                        
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

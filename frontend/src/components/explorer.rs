use yew::html::Scope;
use yew::prelude::*;

use crate::api::download_file;
use crate::app::App;
use crate::i18n::Translations;
use crate::types::{FileItem, Msg};

impl App {
    pub fn render_explorer(&self, ctx: &Context<Self>) -> Html {
        let translations = crate::i18n::get_translations(self.language);
        html! {
            <div id="uploadedFilesList" class="uploaded-files-section" style="padding: 0; background: transparent; box-shadow: none; margin: 0;">
                <div id="uploadedFilesContent" class="uploaded-files-content">
                    {match &self.uploaded_files {
                        None => html! { <div class="loading-message">{translations.loading_files}</div> },
                        Some(data) => {
                            if data.items.is_empty() {
                                html! { <div class="empty-message">{translations.no_files}</div> }
                            } else {
                                let flat_items = flatten_files(&data.items);
                                if flat_items.is_empty() {
                                    html! { <div class="empty-message">{translations.no_files}</div> }
                                } else {
                                    render_file_items(&flat_items, 0, ctx.link().clone(), &translations)
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

fn get_file_icon(extension: &str) -> &'static str {
    match extension {
        ".jpg" | ".jpeg" | ".png" | ".webp" | ".gif" | ".svg" | ".bmp" | ".ico" | ".tiff" => "🖼️ ",
        ".mp4" | ".mkv" | ".avi" | ".mov" | ".webm" | ".flv" | ".wmv" | ".mpeg" => "🎬 ",
        ".mp3" | ".wav" | ".ogg" | ".m4a" | ".flac" | ".aac" | ".wma" => "🎵 ",
        ".pdf" => "📖 ",
        ".zip" | ".rar" | ".7z" | ".tar" | ".gz" | ".bz2" | ".xz" | ".tgz" => "📦 ",
        ".txt" | ".md" | ".log" | ".json" | ".xml" | ".yml" | ".yaml" | ".toml" | ".ini"
        | ".conf" | ".sh" | ".bat" | ".rs" | ".js" | ".ts" | ".py" | ".c" | ".cpp" | ".h"
        | ".html" | ".css" => "📝 ",
        _ => "📄 ",
    }
}

// Render helper for flat file list
fn render_file_items(
    items: &[FileItem],
    _level: usize,
    link: Scope<App>,
    translations: &Translations,
) -> Html {
    html! {
        <>
            {for items.iter().map(|item| {
                match item {
                    FileItem::File { name, path, size: _, formatted_size, upload_date: _, extension } => {
                        let path_c = path.clone();
                        let path_s = path.clone();
                        let link_d = link.clone();
                        let link_s = link.clone();
                        let icon = get_file_icon(extension);

                        let copy_link_txt = translations.copy_link.to_string();
                        let download_txt = translations.download.to_string();
                        let delete_txt = translations.delete.to_string();

                        let copied_success = translations.download_link_copied.to_string();
                        let copied_error = translations.failed_copy_link.to_string();

                        html! {
                            <div class="uploaded-file-item">
                                <div class="uploaded-file-name" style="word-break: break-all;">
                                    {icon}{name}
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
                                        {download_txt}
                                    </button>
                                    <button class="action-btn share-btn" onclick={
                                        let p = path_s.clone();
                                        let l = link_s.clone();
                                        let success_msg = copied_success.clone();
                                        let error_msg = copied_error.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            let window = web_sys::window().unwrap();
                                            let origin = window.location().origin().unwrap_or_default();
                                            let encoded_path = crate::utils::encode_path(&p);
                                            let full_url = format!("{}/api/files/download/{}", origin, encoded_path);

                                            if crate::js_api::copy_text_to_clipboard(&full_url) {
                                                l.send_message(Msg::AddToast(success_msg.clone(), "success".to_string()));
                                            } else {
                                                l.send_message(Msg::AddToast(error_msg.clone(), "error".to_string()));
                                            }
                                        })
                                    }>
                                        {copy_link_txt}
                                    </button>
                                    <button class="action-btn delete-btn" onclick={
                                        let p = path_c.clone();
                                        let l = link_d.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            l.send_message(Msg::DeleteFile(p.clone()));
                                        })
                                    }>
                                        {delete_txt}
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

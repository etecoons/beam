pub mod explorer;
pub mod pin_entry;
pub mod uploader;

use yew::prelude::*;

use crate::app::App;
use crate::types::Msg;

impl App {
    pub fn render_view(&self, ctx: &Context<Self>) -> Html {
        let translations = crate::i18n::get_translations(self.language);
        let site_title = self
            .config
            .as_ref()
            .map(|c| c.site_title.as_str())
            .unwrap_or("Beam");
        let pin_required = self
            .config
            .as_ref()
            .map(|c| c.pin_required)
            .unwrap_or(false);

        let show_version = self.config.as_ref().map(|c| c.show_version).unwrap_or(true);
        let show_github = self.config.as_ref().map(|c| c.show_github).unwrap_or(true);
        let version = env!("CARGO_PKG_VERSION").to_string();

        html! {
            <>
                <crate::header::Header
                    site_title={site_title.to_string()}
                    theme={self.theme.clone()}
                    is_authenticated={self.is_authenticated}
                    pin_required={pin_required}
                    language={self.language}
                    toggle_theme={ctx.link().callback(|_| Msg::ToggleTheme)}
                    on_logout={ctx.link().callback(|_| Msg::Logout)}
                    on_language_change={ctx.link().callback(Msg::SwitchLanguage)}
                    logout_tooltip={translations.log_out.to_string()}
                    disable_print={self.uploaded_files.as_ref().map(|f| f.items.is_empty()).unwrap_or(true)}
                    enable_translation={self.config.as_ref().map(|c| c.enable_translation).unwrap_or(false)}
                    enable_themes={self.config.as_ref().map(|c| c.enable_themes).unwrap_or(true)}
                    enable_print={self.config.as_ref().map(|c| c.enable_print).unwrap_or(false)}
                />
                <div class="container">
                    {if !self.is_authenticated {
                        self.render_pin_entry(ctx)
                    } else {
                        html! {
                            <>
                                <main>
                                    {self.render_uploader(ctx)}

                                    <div style="margin-top: 1.5rem; padding: 0; display: flex; flex-direction: column; flex: 1; min-height: 0;">
                                        {self.render_explorer(ctx)}
                                    </div>
                                </main>
                                {if let Some(ref data) = self.uploaded_files {
                                    html! {
                                        <div class="file-summary" style="margin-top: 2rem; margin-bottom: 1.5rem; font-size: 0.85rem; color: var(--text-color-secondary); opacity: 0.6; text-align: center;">
                                            {format!("{} {} • {}",
                                                data.total_files,
                                                if data.total_files == 1 { &translations.file_singular } else { &translations.file_plural },
                                                data.formatted_total_size
                                            )}
                                        </div>
                                    }
                                } else {
                                    html! {}
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
                                <h3>{translations.rename_item}</h3>
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
                                        {&translations.cancel}
                                    </button>
                                    <button class="modal-btn modal-btn-confirm" onclick={ctx.link().callback(|_| Msg::ConfirmRename)}>
                                        {&translations.rename}
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}

            </div>
            <crate::footer::Footer {show_version} {version} {show_github}>
                {
                    if let Some((msg, cls)) = &self.active_notification {
                        html! { <div class={format!("footer-status-text {}", cls)}>{ msg }</div> }
                    } else {
                        html! { <div class="footer-status-text success">{"Ready"}</div> }
                    }
                }
            </crate::footer::Footer>
            </>
        }
    }
}

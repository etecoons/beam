pub mod pin_entry;
pub mod uploader;
pub mod explorer;

use yew::prelude::*;

use crate::app::App;
use crate::types::Msg;

impl App {
    pub fn render_view(&self, ctx: &Context<Self>) -> Html {
        let site_title = self.config.as_ref().map(|c| c.site_title.as_str()).unwrap_or("RustDrop");

        html! {
            <div class="container">
                {if !self.is_authenticated {
                    self.render_pin_entry(ctx)
                } else {
                    html! {
                        <>
                            <header>
                                <div class="notepad-controls">
                                    <button id="refresh-button" class="icon-button" onclick={ctx.link().callback(|_| Msg::RefreshFiles)} data-tooltip="Refresh Files">
                                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
                                        </svg>
                                    </button>
                                </div>
                                <div id="header-title">
                                    <h1>{site_title}</h1>
                                </div>
                                <div class="header-right">
                                    <button id="theme-toggle" class="icon-button" onclick={ctx.link().callback(|_| Msg::ToggleTheme)} aria-label="Toggle theme">
                                        {if self.theme == "dark" {
                                            html! { <svg id="sun-icon" class="sun" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2" /><path d="M12 20v2" /><path d="M4.93 4.93l1.41 1.41" /><path d="M17.66 17.66l1.41 1.41" /><path d="M2 12h2" /><path d="M20 12h2" /><path d="M6.34 17.66l-1.41 1.41" /><path d="M19.07 4.93l-1.41 1.41" /></svg> }
                                        } else {
                                            html! { <svg id="moon-icon" class="moon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" /></svg> }
                                        }}
                                    </button>
                                    {if self.config.as_ref().map(|c| c.pin_required).unwrap_or(false) {
                                        html! {
                                            <button id="logout-button" class="icon-button" onclick={ctx.link().callback(|_| Msg::Logout)} data-tooltip="Log Out">
                                                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                                                    <polyline points="16 17 21 12 16 7" />
                                                    <line x1="21" y1="12" x2="9" y2="12" />
                                                </svg>
                                            </button>
                                        }
                                    } else {
                                        html! {}
                                    }}
                                </div>
                            </header>
                            <main>
                                {self.render_uploader(ctx)}
                                
                                <div style="margin-top: 1.5rem; padding: 0; overflow-y: auto;">
                                    {self.render_explorer(ctx)}
                                </div>
                            </main>
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

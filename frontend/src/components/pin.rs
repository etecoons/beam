use yew::prelude::*;

use crate::app::App;
use crate::types::Msg;

impl App {
    pub fn render_pin_entry(&self, ctx: &Context<Self>) -> Html {
        let translations = crate::i18n::get_translations(self.language);
        let pin_len = self.config.as_ref().map(|c| c.pin_length).unwrap_or(4);

        html! {
            <div class="login-container">
                <div class="login-box">
                    <div class="pin-header">
                        <h2 id="pin-description">
                            {if self.is_lockout { translations.locked_out } else { translations.enter_pin }}
                        </h2>
                    </div>
                    <form id="pin-form" onsubmit={ctx.link().callback(|e: SubmitEvent| { e.prevent_default(); Msg::VerifyPin })}>
                        <div class="pin-wrapper">
                            <input
                                ref={self.pin_ref.clone()}
                                type="password"
                                class="pin-input-field"
                                disabled={self.is_lockout}
                                value={self.pin_input.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    Msg::PinInputChanged(input.value())
                                })}
                                placeholder={"• ".repeat(pin_len).trim().to_string()}
                                maxlength={pin_len.to_string()}
                                autofocus=true
                            />
                        </div>
                    </form>
                    <div class="pin-status">
                        {if let Some(ref err) = self.error_message {
                            let display_err = if err == "Invalid PIN" || err == "Invalid PIN." {
                                translations.invalid_pin
                            } else if err.contains("Too many") || err.contains("locked") || err.contains("Locked out") {
                                translations.locked_out
                            } else {
                                err.as_str()
                            };
                            html! { <p id="pin-error" class="pin-error" style="display: block;">{display_err}</p> }
                        } else {
                            html! {}
                        }}
                    </div>
                </div>
            </div>
        }
    }
}

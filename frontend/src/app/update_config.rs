use yew::prelude::Context;

use crate::app::App;
use crate::types::Msg;
use crate::utils::{save_theme, set_theme_attribute};

impl App {
    pub fn update_config(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadConfig(res) => {
                match res {
                    Ok(conf) => {
                        self.pin_input = String::new();

                        let site_title = conf.site_title.clone();
                        self.config = Some(conf.clone());

                        if !conf.enable_themes {
                            self.theme = "tourian".to_string();
                            set_theme_attribute("tourian");
                        }

                        // Set document title dynamically
                        if let Some(doc) = gloo_utils::document()
                            .default_view()
                            .and_then(|w| w.document())
                        {
                            doc.set_title(&site_title);
                        }

                        if !conf.pin_required {
                            self.is_authenticated = true;
                            ctx.link().send_message(Msg::RefreshFiles);
                        } else {
                            self.is_authenticated = false;
                            self.reset_pin_inputs();
                        }
                    }
                    Err(e) => {
                        self.show_toast(
                            ctx,
                            &format!("Failed to load configuration: {}", e),
                            "error",
                        );
                    }
                }
                true
            }

            Msg::ToggleTheme => {
                self.theme = match self.theme.as_str() {
                    "crateria" => "brinstar".to_string(),
                    "brinstar" => "norfair".to_string(),
                    "norfair" => "wrecked_ship".to_string(),
                    "wrecked_ship" => "maridia".to_string(),
                    "maridia" => "tourian".to_string(),
                    _ => "crateria".to_string(),
                };
                save_theme(&self.theme);
                set_theme_attribute(&self.theme);
                true
            }
            Msg::SwitchLanguage(lang) => {
                self.language = lang;
                crate::i18n::save_language(lang);
                true
            }
            _ => false,
        }
    }
}

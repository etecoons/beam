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
                        
                        // Set document title dynamically
                        if let Some(doc) = gloo_utils::document().default_view().and_then(|w| w.document()) {
                            doc.set_title(&format!("{} - Simple File Upload", site_title));
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
                        self.show_toast(ctx, &format!("Failed to load configuration: {}", e), "error");
                    }
                }
                true
            }
            
            Msg::ToggleTheme => {
                self.theme = match self.theme.as_str() {
                    "light" => "dark".to_string(),
                    "dark" => "nord".to_string(),
                    "nord" => "dracula".to_string(),
                    "dracula" => "sepia".to_string(),
                    _ => "light".to_string(),
                };
                save_theme(&self.theme);
                set_theme_attribute(&self.theme);
                true
            }
            _ => false,
        }
    }
}

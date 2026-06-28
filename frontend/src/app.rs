pub mod update_config;
pub mod update_files;
pub mod update_pin;
pub mod update_toast;
pub mod update_upload;
pub mod upload_task;
pub mod view;

use std::collections::HashMap;
use yew::prelude::*;

use crate::api::fetch_config;
use crate::types::{FileListResponse, FrontendConfig, Language, Msg, RenameData, UploadProgress};
use crate::utils::{get_saved_theme, set_theme_attribute};

pub struct App {
    // Configuration
    pub config: Option<FrontendConfig>,
    pub is_authenticated: bool,
    pub theme: String,
    pub language: Language,

    // PIN entry inputs
    pub pin_input: String,
    pub pin_ref: NodeRef,
    pub error_message: Option<String>,
    pub is_lockout: bool,

    // Upload tracking
    pub upload_queue: Vec<web_sys::File>,
    pub active_uploads: HashMap<String, UploadProgress>, // key: path
    pub is_uploading: bool,
    pub drag_over: bool,
    pub file_input_ref: NodeRef,
    pub folder_input_ref: NodeRef,

    // File list & Explorer
    pub uploaded_files: Option<FileListResponse>,

    // Rename Modal
    pub rename_target: Option<RenameData>,
    pub rename_input_val: String,

    // Active Notification on Footer
    pub active_notification: Option<(String, String)>,
    pub active_timeout: Option<gloo_timers::callback::Timeout>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Theme
        let theme = get_saved_theme();
        set_theme_attribute(&theme);

        // Language
        let language = crate::i18n::get_saved_language();

        // Fetch config
        let link = ctx.link().clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_config().await {
                Ok(conf) => link.send_message(Msg::LoadConfig(Ok(conf))),
                Err(err) => link.send_message(Msg::LoadConfig(Err(err))),
            }
        });

        Self {
            config: None,
            is_authenticated: false,
            theme,
            language,
            pin_input: String::new(),
            pin_ref: NodeRef::default(),
            error_message: None,
            is_lockout: false,
            upload_queue: Vec::new(),
            active_uploads: HashMap::new(),
            is_uploading: false,
            drag_over: false,
            file_input_ref: NodeRef::default(),
            folder_input_ref: NodeRef::default(),
            uploaded_files: None,
            rename_target: None,
            rename_input_val: String::new(),
            active_notification: None,
            active_timeout: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Nothing => false,
            Msg::LoadConfig(_) | Msg::ToggleTheme | Msg::SwitchLanguage(_) => {
                self.update_config(ctx, msg)
            }
            Msg::PinInputChanged(_)
            | Msg::VerifyPin
            | Msg::PinVerificationResult(_)
            | Msg::Logout => self.update_pin(ctx, msg),
            Msg::DragOver(_)
            | Msg::FilesSelected(_)
            | Msg::FoldersSelected(_)
            | Msg::DropProcessed(_)
            | Msg::StartUploads
            | Msg::UploadInit(_, _)
            | Msg::UploadProgressUpdate(_, _, _, _, _)
            | Msg::UploadCompleted(_)
            | Msg::UploadFailed(_, _) => self.update_upload(ctx, msg),
            Msg::LoadFileList(_)
            | Msg::RefreshFiles
            | Msg::DeleteFile(_)
            | Msg::DeleteResult(_)
            | Msg::StartRename(_, _)
            | Msg::CancelRename
            | Msg::ConfirmRename
            | Msg::RenameInputChanged(_)
            | Msg::RenameResult(_) => self.update_files(ctx, msg),
            Msg::AddToast(_, _) | Msg::RemoveToast(_) => self.update_toast(ctx, msg),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        self.view_app(ctx)
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if !self.is_authenticated
            && !self.is_lockout
            && let Some(input) = self.pin_ref.cast::<web_sys::HtmlInputElement>()
        {
            let _ = input.focus();
            let input_clone = input.clone();
            gloo_timers::callback::Timeout::new(50, move || {
                let _ = input_clone.focus();
            })
            .forget();
        }
    }
}

impl App {
    pub fn show_toast(&mut self, ctx: &Context<Self>, message: &str, toast_type: &str) {
        ctx.link()
            .send_message(Msg::AddToast(message.to_string(), toast_type.to_string()));
    }

    pub fn reset_pin_inputs(&mut self) {
        self.pin_input = String::new();
        if let Some(input) = self.pin_ref.cast::<web_sys::HtmlInputElement>() {
            input.set_value("");
            let _ = input.focus();
            let input_clone = input.clone();
            gloo_timers::callback::Timeout::new(50, move || {
                let _ = input_clone.focus();
            })
            .forget();
        }
    }
}

use wasm_bindgen::JsValue;

pub fn get_saved_theme() -> String {
    let window = web_sys::window().unwrap();
    let local_storage = window.local_storage().unwrap().unwrap();
    if let Ok(Some(theme)) = local_storage.get_item("theme") {
        theme
    } else {
        let media_query = window.match_media("(prefers-color-scheme: dark)").unwrap().unwrap();
        if media_query.matches() {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    }
}

pub fn save_theme(theme: &str) {
    let window = web_sys::window().unwrap();
    let local_storage = window.local_storage().unwrap().unwrap();
    let _ = local_storage.set_item("theme", theme);
}

pub fn set_theme_attribute(theme: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let html = document.document_element().unwrap();
    let _ = html.set_attribute("data-theme", theme);
}

pub fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 Bytes".to_string();
    }
    let k = 1024.0;
    let sizes = ["Bytes", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(k).floor() as usize;
    let val = bytes as f64 / k.powi(i as i32);
    format!("{:.2} {}", val, sizes[i])
}

#[allow(dead_code)]
pub fn format_date(date_str: &str) -> String {
    if date_str.len() >= 10 {
        date_str[0..10].to_string()
    } else {
        date_str.to_string()
    }
}

pub fn generate_batch_id() -> String {
    let window = web_sys::window().unwrap();
    let now = window.performance().unwrap().now() as u64;
    let random: u32 = js_sys::Math::random().to_bits() as u32;
    format!("{}-{:x}", now, random)
}

pub fn encode_path(file_path: &str) -> String {
    file_path
        .split('/')
        .map(|part| {
            js_sys::encode_uri_component(part)
                .as_string()
                .unwrap_or_else(|| part.to_string())
        })
        .collect::<Vec<String>>()
        .join("/")
}

pub fn get_file_path(file: &web_sys::File) -> String {
    let path = js_sys::Reflect::get(file, &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    if path.is_empty() {
        file.name()
    } else {
        path
    }
}

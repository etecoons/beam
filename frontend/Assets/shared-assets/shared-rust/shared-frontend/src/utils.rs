use yew::Callback;
use wasm_bindgen::JsCast;

/// Auto-copies highlighted text from the current selection.
/// Returns Some(copied_text) if text was successfully copied, otherwise None.
pub fn copy_selection_to_clipboard() -> Option<String> {
    let window = web_sys::window()?;
    let selection = window.get_selection().ok()??;
    let selected_text = String::from(selection.to_string());

    if selected_text.trim().is_empty() {
        return None;
    }

    let navigator = window.navigator();
    let clipboard = navigator.clipboard();

    // Write text to clipboard
    let _ = clipboard.write_text(&selected_text);

    Some(selected_text)
}

/// Helper utility to read a web_sys::File asynchronously as a vector of bytes.
pub fn read_file_as_bytes(file: &web_sys::File, on_loaded: Callback<Vec<u8>>) {
    let file_reader = web_sys::FileReader::new().unwrap();
    let file_reader_c = file_reader.clone();

    let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
        if let Ok(result) = file_reader_c.result() {
            let array_buffer = js_sys::ArrayBuffer::from(result);
            let uint8_array = js_sys::Uint8Array::new(&array_buffer);
            let bytes = uint8_array.to_vec();
            on_loaded.emit(bytes);
        }
    }) as Box<dyn FnMut(web_sys::Event)>);

    file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    let _ = file_reader.read_as_array_buffer(file);
}

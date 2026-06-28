//! Beam Frontend Application
//!
//! A clean, responsive file uploader built using Yew and WebAssembly.
//! This module serves as the application entry point and mounts the
//! root component to the DOM.
//!
//! # Architecture
//!
//! - `api`: Outbound fetch logic for the backend REST routes.
//! - `app`: Main layout component, message loop, and state container.
//! - `js_api`: FFI / wasm_bindgen interface for drag-and-drop actions.
//! - `types`: Core configurations and Yew messages definitions.
//! - `utils`: Styling themes and number format helpers.

mod api;
mod app;
mod components;
mod i18n;
mod js_api;
mod storage;
mod types;
mod utils;

use app::App;

/// The main entry point for the WebAssembly client.
///
/// It initializes the Yew renderer with our root `App` component
/// and starts the rendering loop.
fn main() {
    yew::Renderer::<App>::new().render();
}

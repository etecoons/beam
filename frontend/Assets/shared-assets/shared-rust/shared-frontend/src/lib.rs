pub mod components;
pub mod theme;
pub mod utils;

// Re-export i18n from shared-core so frontend components can use it via crate::i18n
pub use shared_core::i18n;

// Re-exports for ergonomics
pub use components::{footer, footer::Footer, header, header::Header};

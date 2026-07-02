//! Shared axum middleware factories.
//!
//! Each factory takes configuration from [`crate::server::ServerConfig`] and
//! returns a `tower` layer ready to install on a router.

pub mod cors;
pub mod hsts;
pub mod security_headers;
pub mod title;

pub use cors::cors_layer;
pub use hsts::{HstsState, hsts_layer};
pub use security_headers::security_headers_layer;
pub use title::{TitleState, title_injection_layer};

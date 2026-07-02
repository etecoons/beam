//! Backend server primitives shared by every UberMetroid companion app.
//!
//! Provides:
//!
//! - [`ServerConfig`] — common env-driven configuration struct
//! - [`bootstrap::serve`] — bind + serve with graceful shutdown
//! - [`error::ServerError`] — `IntoResponse` error type
//! - [`ip`] — client IP extraction helpers
//! - [`version`] — `CARGO_PKG_VERSION` helper

pub mod bootstrap;
pub mod config;
pub mod error;
pub mod ip;
pub mod version;

#[cfg(test)]
mod tests;

pub use bootstrap::serve;
pub use config::ServerConfig;
pub use error::ServerError;
pub use ip::{get_client_ip, normalize_ip};
pub use version::CARGO_PKG_VERSION;

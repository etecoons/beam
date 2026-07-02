//! Compile-time version string for the consuming crate.

/// Exposes the consuming crate's `CARGO_PKG_VERSION` via `env!` so callers
/// don't have to depend on a build script. Useful for `X-App-Version` headers
/// and footer version labels.
pub const fn cargo_pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Re-export under a shorter name for ergonomics.
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!cargo_pkg_version().is_empty());
        assert!(!CARGO_PKG_VERSION.is_empty());
    }
}

//! Build the authentication `Set-Cookie` value used by this app's
//! auth-flow handlers.
//!
//! Per-app copy of the prior shared helper, kept local so each app
//! tunes its own cookie semantics (clamp range, cookie name,
//! SameSite policy).

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

const MIN_LIFETIME_SECONDS: u64 = 60;
const MAX_LIFETIME_SECONDS: u64 = 30 * 24 * 3600;

/// Name of the auth cookie used by this app.
const COOKIE_NAME: &str = "BEAM_PIN";

/// Construct an authentication cookie carrying `value` under
/// [`COOKIE_NAME`].
#[must_use]
pub fn build_cookie(value: &str, max_age_hours: i64, secure: bool) -> Cookie<'static> {
    let max_age_seconds = clamp_seconds(max_age_hours.saturating_mul(3600));
    Cookie::build((COOKIE_NAME, value.to_string()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(Duration::seconds(max_age_seconds as i64))
        .build()
}

/// Build an `expired` cookie used to clear the session on logout.
#[must_use]
pub fn build_clear_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(Duration::ZERO)
        .build()
}

/// Decide whether an auth cookie should be marked `Secure`.
#[must_use]
pub fn cookie_should_be_secure(headers: &axum::http::HeaderMap, base_url: &str) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
        || base_url.starts_with("https")
}

fn clamp_seconds(seconds: i64) -> u64 {
    if seconds <= 0 {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime non-positive; clamping to 60s"
        );
        return MIN_LIFETIME_SECONDS;
    }
    let unsigned = u64::try_from(seconds).unwrap_or(MAX_LIFETIME_SECONDS);
    if unsigned < MIN_LIFETIME_SECONDS {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime too short; clamping to 60s"
        );
        return MIN_LIFETIME_SECONDS;
    }
    if unsigned > MAX_LIFETIME_SECONDS {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime too long; clamping to 30 days"
        );
        return MAX_LIFETIME_SECONDS;
    }
    unsigned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_cookie_uses_cookie_name() {
        let c = build_cookie("deadbeef", 24, false);
        assert_eq!(c.name(), COOKIE_NAME);
        assert_eq!(c.value(), "deadbeef");
        assert_eq!(c.path(), Some("/"));
        assert!(c.http_only().unwrap_or(false));
        assert_eq!(c.secure(), Some(false));
    }

    #[test]
    fn auth_cookie_secure_flag_propagates() {
        let c = build_cookie("x", 24, true);
        assert_eq!(c.secure(), Some(true));
    }

    #[test]
    fn clear_cookie_has_zero_max_age() {
        let c = build_clear_cookie(false);
        assert_eq!(c.value(), "");
        assert_eq!(c.max_age(), Some(Duration::ZERO));
    }

    #[test]
    fn clamps_negative_hours_to_one_minute() {
        let c = build_cookie("x", -1, false);
        assert_eq!(
            c.max_age(),
            Some(Duration::seconds(MIN_LIFETIME_SECONDS as i64))
        );
    }

    #[test]
    fn clamps_overly_large_hours_to_thirty_days() {
        let c = build_cookie("x", 24 * 365 * 100, false);
        assert_eq!(
            c.max_age(),
            Some(Duration::seconds(MAX_LIFETIME_SECONDS as i64))
        );
    }

    #[test]
    fn cookie_secure_via_xfp() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-forwarded-proto", "https".parse().unwrap());
        assert!(cookie_should_be_secure(&h, "http://example.com"));
    }

    #[test]
    fn cookie_secure_via_base_url() {
        let h = axum::http::HeaderMap::new();
        assert!(cookie_should_be_secure(&h, "https://app.example"));
        assert!(!cookie_should_be_secure(&h, "http://app.example"));
    }

    #[test]
    fn cookie_secure_handles_uppercase_https_header() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-forwarded-proto", "HTTPS".parse().unwrap());
        assert!(cookie_should_be_secure(&h, "http://app.example"));
    }
}

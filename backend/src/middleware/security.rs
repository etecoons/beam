//! Security primitives shim.
//!
//! Historically this module contained full implementations of the
//! constant-time PIN compare, the per-IP lockout state, the X-Forwarded-For
//! parser, and the security headers. All of those are now provided by
//! [`shared_backend`] — see:
//!
//! - `shared_backend::auth::{is_locked_out, record_attempt, reset_attempts,
//!   lockout_remaining_secs}` — process-global per-IP lockout
//! - `crate::ip::get_client_ip` — X-Forwarded-For with
//!   trusted-proxy allowlist (no unsafe default when the allowlist is
//!   empty)
//! - `crate::middleware::security_headers_layer` — replaces the
//!   `security_headers_middleware` axum function previously defined here
//! - `crate::middleware::hsts_layer` — replaces the
//!   `hsts_middleware` axum function previously defined here
//!
//! This file exists as a thin adapter so existing call sites
//! (`crate::security::{is_locked_out, record_attempt, ...}`) keep
//! working without a large refactor. New code should prefer the
//! `shared_backend` paths directly.

use axum::http::HeaderMap;
use constant_time_eq::constant_time_eq;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

/// How long a per-IP lockout lasts after the maximum number of failed
/// attempts. Mirrors the prior hard-coded value; the env-tunable version
/// lives in shared-assets's `ServerConfig::lockout_duration`.
const LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60);

/// Read `MAX_ATTEMPTS` from the environment, defaulting to 5.
pub fn get_max_attempts() -> u32 {
    std::env::var("MAX_ATTEMPTS")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(5)
}

/// Constant-time compare of two byte strings.
pub fn safe_compare(a: &str, b: &str) -> bool {
    constant_time_eq(a.as_bytes(), b.as_bytes())
}

/// Returns `true` if `ip` is currently locked out from further PIN attempts.
pub fn is_locked_out(ip: &str) -> bool {
    shared_backend::auth::is_locked_out(ip, get_max_attempts(), LOCKOUT_DURATION)
}

/// Record a failed PIN attempt for `ip` and return the updated record.
pub fn record_attempt(ip: &str) -> shared_backend::auth::Attempt {
    shared_backend::auth::record_attempt(ip)
}

/// Clear the attempt record for `ip` (called after a successful login).
pub fn reset_attempts(ip: &str) {
    shared_backend::auth::reset_attempts(ip);
}

/// Seconds remaining in the lockout for `ip`, or `0` if not locked out.
pub fn get_lockout_time_remaining(ip: &str) -> u64 {
    shared_backend::auth::lockout_remaining_secs(ip, LOCKOUT_DURATION)
}

/// Resolve the client IP from request metadata, with a trusted-proxy
/// allowlist.
///
/// `trusted_proxy_ips` is a list of CIDRs or single IPs that the operator
/// has configured as trusted reverse proxies. When the list is non-empty,
/// `X-Forwarded-For` is honored only if the connecting socket IP is in the
/// list. When the list is empty, `X-Forwarded-For` is ignored entirely —
/// this is the SAFE default; the previous implementation honored the
/// header unconditionally when `trust_proxy=true` regardless of whether
/// the allowlist was populated, which is a known bypass.
pub fn get_client_ip(
    headers: &HeaderMap,
    addr: SocketAddr,
    trust_proxy: bool,
    trusted_proxy_ips: Option<&[String]>,
) -> String {
    use ipnet::IpNet;

    let socket_ip = shared_backend::server::normalize_ip(addr.ip());

    if !trust_proxy {
        return socket_ip.to_string();
    }

    let Some(forwarded) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) else {
        return socket_ip.to_string();
    };

    let Some(first) = forwarded.split(',').next() else {
        return socket_ip.to_string();
    };
    let trimmed = first.trim();

    // SAFETY: require an allowlist. Without one, an attacker can forge
    // X-Forwarded-For to rotate their lockout-key IP at will. This is the
    // critical behavior change vs. the previous implementation.
    let trusted = match trusted_proxy_ips {
        Some(list) if !list.is_empty() => list,
        _ => {
            tracing::warn!(
                "X-Forwarded-For received but no trusted_proxy_ips configured; \
                 ignoring the header and using the connecting socket IP. \
                 Set TRUSTED_PROXY_IPS to the CIDR of your reverse proxy \
                 (e.g. 127.0.0.1/32) to enable forwarded-IP resolution."
            );
            return socket_ip.to_string();
        }
    };

    let trusted_nets: Vec<IpNet> = trusted
        .iter()
        .filter_map(
            |s| match (IpNet::from_str(s.trim()), s.trim().parse::<IpAddr>()) {
                (Ok(net), _) => Some(net),
                (Err(_), Ok(ip)) => IpNet::from_str(&format!("{ip}/32")).ok(),
                _ => None,
            },
        )
        .collect();

    if !trusted_nets.iter().any(|net| net.contains(&socket_ip)) {
        return socket_ip.to_string();
    }

    trimmed
        .parse::<IpAddr>()
        .map(shared_backend::server::normalize_ip)
        .map_or_else(|_| socket_ip.to_string(), |ip| ip.to_string())
}

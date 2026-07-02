//! Client IP extraction helpers.
//!
//! Resolves the connecting client's IP from a request's `ConnectInfo` and
//! optionally a `X-Forwarded-For` header (when behind a trusted proxy).

use std::net::{IpAddr, SocketAddr};

/// Normalize an IPv4-mapped IPv6 address to plain IPv4.
#[must_use]
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        v4 => v4,
    }
}

/// Resolve the client IP from request metadata.
///
/// Order of resolution:
/// 1. If `trust_proxy` is true and a trusted proxy IP list is configured,
///    verify the connecting socket IP is in that list, then use the first
///    entry from `X-Forwarded-For`.
/// 2. If `trust_proxy` is true and no trusted list is configured, use the
///    first `X-Forwarded-For` entry.
/// 3. Otherwise, fall back to the socket IP.
#[must_use]
pub fn get_client_ip(
    headers: &axum::http::HeaderMap,
    socket_addr: SocketAddr,
    trust_proxy: bool,
    trusted_proxies: &[ipnet::IpNet],
) -> String {
    let socket_ip = normalize_ip(socket_addr.ip());

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
    //
    // We allow `X-Forwarded-For` only when:
    //   1. `trusted_proxies` is non-empty, AND
    //   2. the connecting socket IP is contained in at least one of them.
    //
    // If `trusted_proxies` is empty (operator forgot to set
    // `TRUSTED_PROXY_IPS`), fall through to the socket IP rather than
    // trusting the spoofable header.
    if trusted_proxies.is_empty() || !trusted_proxies.iter().any(|net| net.contains(&socket_ip)) {
        return socket_ip.to_string();
    }

    trimmed
        .parse::<IpAddr>()
        .map(normalize_ip)
        .map_or_else(|_| socket_ip.to_string(), |ip| ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::net::{Ipv4Addr, SocketAddrV4};

    fn socket_v4(ip: [u8; 4]) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            4401,
        ))
    }

    #[test]
    fn no_proxy_returns_socket_ip() {
        let headers = HeaderMap::new();
        let ip = get_client_ip(&headers, socket_v4([10, 0, 0, 1]), false, &[]);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn proxy_without_trusted_list_uses_socket_ip() {
        // `trust_proxy=true` with an empty `trusted_proxies` allowlist
        // must fail-safe to the connecting socket IP. Trusting
        // X-Forwarded-For under those conditions would let an attacker
        // rotate their lockout-key IP at will by forging the header.
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
        let ip = get_client_ip(&headers, socket_v4([10, 0, 0, 1]), true, &[]);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn proxy_with_trusted_list_rejects_untrusted_socket() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
        // Socket is not in the trusted list, so we ignore X-Forwarded-For.
        let trusted: ipnet::IpNet = "192.168.1.0/24".parse().unwrap();
        let ip = get_client_ip(&headers, socket_v4([10, 0, 0, 1]), true, &[trusted]);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn proxy_with_trusted_list_accepts_trusted_socket() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
        let trusted: ipnet::IpNet = "10.0.0.0/8".parse().unwrap();
        let ip = get_client_ip(&headers, socket_v4([10, 0, 0, 1]), true, &[trusted]);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn ipv6_mapped_normalized_to_v4() {
        let socket: SocketAddr = "[::ffff:1.2.3.4]:80".parse().unwrap();
        let ip = get_client_ip(&HeaderMap::new(), socket, false, &[]);
        assert_eq!(ip, "1.2.3.4");
    }
}

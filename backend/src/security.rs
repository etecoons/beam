use axum::http::HeaderMap;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};


const LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct Attempt {
    pub count: u32,
    pub last_attempt: Instant,
}

fn login_attempts() -> &'static Mutex<HashMap<String, Attempt>> {
    static ATTEMPTS: OnceLock<Mutex<HashMap<String, Attempt>>> = OnceLock::new();
    ATTEMPTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn reset_attempts(ip: &str) {
    if let Ok(mut attempts) = login_attempts().lock() {
        attempts.remove(ip);
    }
}

pub fn is_locked_out(ip: &str) -> bool {
    if let Ok(mut attempts) = login_attempts().lock()
        && let Some(attempt) = attempts.get(ip)
        && attempt.count >= get_max_attempts()
    {
        if attempt.last_attempt.elapsed() < LOCKOUT_DURATION {
            return true;
        }
        attempts.remove(ip);
    }
    false
}

pub fn record_attempt(ip: &str) -> Attempt {
    if let Ok(mut attempts) = login_attempts().lock() {
        let now = Instant::now();
        let attempt = attempts.entry(ip.to_string()).or_insert(Attempt {
            count: 0,
            last_attempt: now,
        });
        attempt.count += 1;
        attempt.last_attempt = now;
        attempt.clone()
    } else {
        Attempt {
            count: 1,
            last_attempt: Instant::now(),
        }
    }
}

pub fn get_lockout_time_remaining(ip: &str) -> u64 {
    if let Ok(attempts) = login_attempts().lock()
        && let Some(attempt) = attempts.get(ip)
    {
        let elapsed = attempt.last_attempt.elapsed();
        if elapsed < LOCKOUT_DURATION {
            let remaining = LOCKOUT_DURATION - elapsed;
            return remaining.as_secs();
        }
    }
    0
}

pub fn safe_compare(a: &str, b: &str) -> bool {
    constant_time_eq::constant_time_eq(a.as_bytes(), b.as_bytes())
}

pub fn get_max_attempts() -> u32 {
    std::env::var("MAX_ATTEMPTS")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(5)
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                IpAddr::V4(ipv4)
            } else {
                IpAddr::V6(ipv6)
            }
        }
        IpAddr::V4(ipv4) => IpAddr::V4(ipv4),
    }
}

pub fn get_client_ip(
    headers: &HeaderMap,
    addr: SocketAddr,
    trust_proxy: bool,
    trusted_proxy_ips: Option<&[String]>,
) -> String {
    let socket_ip = normalize_ip(addr.ip());

    if trust_proxy
        && let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok())
        && let Some(first_ip_str) = forwarded_for.split(',').next()
    {
        let trimmed = first_ip_str.trim();
        if let Some(trusted) = trusted_proxy_ips {
            // For security, if trusted proxy IPs are configured, verify the connecting socket IP is in that list.
            let is_trusted = trusted.iter().any(|t_str| {
                if let Ok(t_ip) = t_str.parse::<IpAddr>() {
                    normalize_ip(t_ip) == socket_ip
                } else {
                    false
                }
            });
            if is_trusted && let Ok(ip) = trimmed.parse::<IpAddr>() {
                return normalize_ip(ip).to_string();
            }
        } else if let Ok(ip) = trimmed.parse::<IpAddr>() {
            return normalize_ip(ip).to_string();
        }
    }
    socket_ip.to_string()
}

pub fn hash_pin(pin: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect::<String>()
}

pub async fn security_headers_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    headers.insert("X-Frame-Options", axum::http::header::HeaderValue::from_static("DENY"));
    headers.insert("X-Content-Type-Options", axum::http::header::HeaderValue::from_static("nosniff"));
    headers.insert("Referrer-Policy", axum::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"));
    headers.insert(
        "Content-Security-Policy", 
        axum::http::header::HeaderValue::from_static(
            "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: blob: https:; connect-src 'self' ws: wss: http: https:; font-src 'self'; manifest-src 'self';"
        )
    );
    
    response
}


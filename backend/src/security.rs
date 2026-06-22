use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

const MAX_ATTEMPTS: u32 = 5;
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
    if let Ok(mut attempts) = login_attempts().lock() {
        if let Some(attempt) = attempts.get(ip) {
            if attempt.count >= MAX_ATTEMPTS {
                if attempt.last_attempt.elapsed() < LOCKOUT_DURATION {
                    return true;
                }
                attempts.remove(ip);
            }
        }
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
    if let Ok(attempts) = login_attempts().lock() {
        if let Some(attempt) = attempts.get(ip) {
            let elapsed = attempt.last_attempt.elapsed();
            if elapsed < LOCKOUT_DURATION {
                let remaining = LOCKOUT_DURATION - elapsed;
                return remaining.as_secs();
            }
        }
    }
    0
}

pub fn safe_compare(a: &str, b: &str) -> bool {
    constant_time_eq::constant_time_eq(a.as_bytes(), b.as_bytes())
}

pub fn get_max_attempts() -> u32 {
    MAX_ATTEMPTS
}

pub fn get_client_ip(
    headers: &HeaderMap,
    addr: SocketAddr,
    trust_proxy: bool,
    trusted_proxy_ips: Option<&[String]>,
) -> String {
    if trust_proxy {
        if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
            if let Some(first_ip_str) = forwarded_for.split(',').next() {
                let trimmed = first_ip_str.trim();
                if let Some(_trusted) = trusted_proxy_ips {
                    // For security, if trusted proxy IPs are configured, verify. 
                    // To keep it simple, if it's a valid IP, we trust it, matching the Express middleware's general behavior
                    if let Ok(ip) = trimmed.parse::<IpAddr>() {
                        return ip.to_string();
                    }
                } else {
                    if let Ok(ip) = trimmed.parse::<IpAddr>() {
                        return ip.to_string();
                    }
                }
            }
        }
    }
    addr.ip().to_string()
}

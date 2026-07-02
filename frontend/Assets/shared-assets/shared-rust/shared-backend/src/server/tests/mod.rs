//! Unit tests for the `server` module.
//!
//! Tests touching process env must serialize (env is process-global).
//! `cargo test` runs tests in parallel threads of the same process, which
//! races `env::set_var` across tests. The `ENV_LOCK` mutex serializes them.

use super::*;
use std::env;
use std::sync::Mutex;

pub mod config;
pub mod general;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_clean_env<F: FnOnce()>(vars: &[&str], f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let originals: Vec<Option<String>> = vars.iter().map(|v| env::var(v).ok()).collect();
    for v in vars {
        unsafe { env::remove_var(v) };
    }
    f();
    for (v, original) in vars.iter().zip(originals) {
        match original {
            Some(val) => unsafe { env::set_var(v, val) },
            None => unsafe { env::remove_var(v) },
        }
    }
}

fn with_env<F: FnOnce()>(vars: &[(&str, &str)], f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let originals: Vec<(&str, Option<String>)> =
        vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();
    let all_keys = [
        "PORT",
        "SITE_TITLE",
        "BASE_URL",
        "ALLOWED_ORIGINS",
        "BEAM_PIN",
        "BEAM_SITE_TITLE",
        "BEAM_TITLE",
        "PIN",
        "ENABLE_TRANSLATION",
        "ENABLE_THEMES",
        "ENABLE_PRINT",
        "SHOW_VERSION",
        "SHOW_GITHUB",
        "MAX_ATTEMPTS",
        "LOCKOUT_TIME_MINUTES",
        "COOKIE_MAX_AGE_HOURS",
        "TRUST_PROXY",
        "TRUSTED_PROXY_IPS",
        "SHUTDOWN_DRAIN_SECONDS",
    ];
    let originals_all: Vec<(&str, Option<String>)> = all_keys
        .iter()
        .map(|k| (*k, env::var(k).ok()))
        .filter(|(k, _)| !vars.iter().any(|(kk, _)| kk == k))
        .collect();
    for k in all_keys {
        unsafe { env::remove_var(k) };
    }
    for (k, v) in vars {
        unsafe { env::set_var(k, *v) };
    }
    f();
    for k in all_keys {
        unsafe { env::remove_var(k) };
    }
    for (k, val) in originals_all.iter().chain(originals.iter()) {
        if let Some(v) = val {
            unsafe { env::set_var(k, v) };
        }
    }
}

fn minimal_config() -> ServerConfig {
    ServerConfig {
        port: 4401,
        site_title: "X".into(),
        base_url: "http://localhost:4401".into(),
        allowed_origins: "*".into(),
        pin: None,
        enable_translation: false,
        enable_themes: false,
        enable_print: false,
        show_version: true,
        show_github: true,
        trust_proxy: false,
        trusted_proxies: vec![],
        max_attempts: 5,
        lockout_time_minutes: 15,
        cookie_max_age_hours: 24,
        shutdown_drain_seconds: 5,
    }
}

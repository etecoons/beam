use super::{minimal_config, with_clean_env, with_env};
use crate::server::ServerConfig;
use std::env;

#[test]
fn pin_enabled_reflects_config() {
    let mut cfg = minimal_config();
    cfg.pin = Some("12345678".into());
    assert!(cfg.pin_enabled());
    cfg.pin = None;
    assert!(!cfg.pin_enabled());
}

#[test]
fn lockout_duration_scales_with_minutes() {
    let mut cfg = minimal_config();
    cfg.lockout_time_minutes = 30;
    assert_eq!(cfg.lockout_duration().as_secs(), 30 * 60);
}

#[test]
fn parse_trusted_proxies_handles_cidrs() {
    with_clean_env(&["TRUSTED_PROXY_IPS"], || {
        unsafe { env::set_var("TRUSTED_PROXY_IPS", "10.0.0.0/8, 192.168.1.0/24, garbage") };
        let cfg = ServerConfig::from_env("X");
        assert_eq!(cfg.trusted_proxies.len(), 2, "garbage should be skipped");
    });
}

#[test]
fn port_parses_correctly() {
    with_env(&[("PORT", "8080")], || {
        assert_eq!(ServerConfig::from_env("X").port, 8080);
    });
}

#[test]
fn port_falls_back_on_invalid() {
    with_env(&[("PORT", "not_a_number")], || {
        assert_eq!(ServerConfig::from_env("X").port, 4401);
    });
}

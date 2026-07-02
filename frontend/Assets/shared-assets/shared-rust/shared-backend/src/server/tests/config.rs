use super::{with_clean_env, with_env};
use crate::server::ServerConfig;
use std::env;

#[test]
fn defaults_when_no_env_set() {
    with_clean_env(
        &[
            "PORT",
            "SITE_TITLE",
            "BASE_URL",
            "ALLOWED_ORIGINS",
            "BEAM_PIN",
            "ENABLE_TRANSLATION",
            "ENABLE_THEMES",
            "ENABLE_PRINT",
            "MAX_ATTEMPTS",
            "LOCKOUT_TIME_MINUTES",
            "COOKIE_MAX_AGE_HOURS",
            "TRUST_PROXY",
            "TRUSTED_PROXY_IPS",
            "BEAM_SITE_TITLE",
            "BEAM_TITLE",
        ],
        || {
            let cfg = ServerConfig::from_env("BEAM");
            assert_eq!(cfg.port, 4401);
            assert_eq!(cfg.site_title, "BEAM");
            assert_eq!(cfg.base_url, "http://localhost:4401");
            assert_eq!(cfg.allowed_origins, "*");
            assert!(cfg.pin.is_none());
            assert!(!cfg.enable_translation);
            assert!(cfg.enable_themes);
            assert!(!cfg.enable_print);
            assert!(cfg.show_version);
            assert!(cfg.show_github);
            assert_eq!(cfg.max_attempts, 5);
            assert_eq!(cfg.lockout_time_minutes, 15);
            assert_eq!(cfg.cookie_max_age_hours, 24);
        },
    );
}

#[test]
fn pin_prefix_lookup_order() {
    with_clean_env(
        &[
            "PIN",
            "BEAM_PIN",
            "SITE_TITLE",
            "BEAM_SITE_TITLE",
            "BEAM_TITLE",
        ],
        || {
            unsafe { env::set_var("PIN", "12345678") };
            assert_eq!(
                ServerConfig::from_env("BEAM").pin.as_deref(),
                Some("12345678")
            );

            unsafe { env::set_var("BEAM_PIN", "app_pin_12") };
            assert_eq!(
                ServerConfig::from_env("BEAM").pin.as_deref(),
                Some("app_pin_12"),
                "prefix wins"
            );
        },
    );
}

#[test]
fn pin_rejected_when_too_short() {
    with_clean_env(&["BEAM_PIN"], || {
        unsafe { env::set_var("BEAM_PIN", "abc") };
        assert!(ServerConfig::from_env("BEAM").pin.is_none());
    });
}

#[test]
fn site_title_prefix_lookup_order() {
    with_clean_env(&["SITE_TITLE", "BEAM_SITE_TITLE", "BEAM_TITLE"], || {
        unsafe { env::set_var("SITE_TITLE", "FromGeneric") };
        assert_eq!(ServerConfig::from_env("BEAM").site_title, "FromGeneric");

        unsafe { env::set_var("BEAM_TITLE", "FromTitle") };
        assert_eq!(
            ServerConfig::from_env("BEAM").site_title,
            "FromTitle",
            "_TITLE beats generic"
        );

        unsafe { env::set_var("BEAM_SITE_TITLE", "FromSiteTitle") };
        assert_eq!(
            ServerConfig::from_env("BEAM").site_title,
            "FromSiteTitle",
            "_SITE_TITLE beats _TITLE"
        );
    });
}

#[test]
fn booleans_truthy_values() {
    with_env(
        &[("ENABLE_TRANSLATION", "true"), ("ENABLE_THEMES", "on")],
        || {
            let cfg = ServerConfig::from_env("X");
            assert!(cfg.enable_translation);
            assert!(cfg.enable_themes);
        },
    );
}

#[test]
fn opt_out_booleans_default_true() {
    with_env(&[], || {
        let cfg = ServerConfig::from_env("X");
        assert!(cfg.show_version);
        assert!(cfg.show_github);
    });
    with_env(&[("SHOW_VERSION", "false")], || {
        assert!(!ServerConfig::from_env("X").show_version);
    });
}

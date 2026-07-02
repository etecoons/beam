use super::*;
use serial_test::serial;
use std::sync::Mutex;

static ATTEMPTS_LOCK: Mutex<()> = Mutex::new(());

fn reset_for_test() -> std::sync::MutexGuard<'static, ()> {
    if let Ok(mut map) = login_attempts().lock() {
        map.clear();
    }
    ATTEMPTS_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[serial]
#[test]
fn starts_unlocked() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    assert!(!is_locked_out("1.2.3.4", 3, lockout));
}

#[serial]
#[test]
fn locks_after_max_attempts() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    for _ in 0..3 {
        record_attempt("1.2.3.4");
    }
    assert!(is_locked_out("1.2.3.4", 3, lockout));
}

#[serial]
#[test]
fn not_locked_below_threshold() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    record_attempt("1.2.3.4");
    record_attempt("1.2.3.4");
    assert!(!is_locked_out("1.2.3.4", 3, lockout));
}

#[serial]
#[test]
fn reset_clears_lockout() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    for _ in 0..3 {
        record_attempt("1.2.3.4");
    }
    assert!(is_locked_out("1.2.3.4", 3, lockout));
    reset_attempts("1.2.3.4");
    assert!(!is_locked_out("1.2.3.4", 3, lockout));
}

#[serial]
#[test]
fn distinct_ips_are_independent() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    for _ in 0..3 {
        record_attempt("1.1.1.1");
    }
    assert!(is_locked_out("1.1.1.1", 3, lockout));
    assert!(!is_locked_out("2.2.2.2", 3, lockout));
}

#[serial]
#[test]
fn remaining_secs_zero_when_not_locked() {
    let _g = reset_for_test();
    assert_eq!(
        lockout_remaining_secs("1.2.3.4", std::time::Duration::from_secs(60)),
        0
    );
}

#[serial]
#[test]
fn remaining_secs_positive_when_locked() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    for _ in 0..3 {
        record_attempt("1.2.3.4");
    }
    let remaining = lockout_remaining_secs("1.2.3.4", lockout);
    assert!(remaining > 0 && remaining <= 60);
}

#[serial]
#[test]
fn current_attempts_zero_when_unknown() {
    let _g = reset_for_test();
    assert_eq!(current_attempts("9.9.9.9"), 0);
}

#[serial]
#[test]
fn current_attempts_reflects_recordings() {
    let _g = reset_for_test();
    record_attempt("9.9.9.9");
    record_attempt("9.9.9.9");
    assert_eq!(current_attempts("9.9.9.9"), 2);
}

#[serial]
#[test]
fn current_attempts_cleared_after_reset() {
    let _g = reset_for_test();
    record_attempt("9.9.9.9");
    reset_attempts("9.9.9.9");
    assert_eq!(current_attempts("9.9.9.9"), 0);
}

#[serial]
#[test]
fn attempts_left_max_when_no_attempts() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    assert_eq!(attempts_left("9.9.9.9", 5, lockout), 5);
}

#[serial]
#[test]
fn attempts_left_decreases_with_failures() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    record_attempt("9.9.9.9");
    record_attempt("9.9.9.9");
    assert_eq!(attempts_left("9.9.9.9", 5, lockout), 3);
}

#[serial]
#[test]
fn attempts_left_zero_when_locked() {
    let _g = reset_for_test();
    let lockout = std::time::Duration::from_secs(60);
    for _ in 0..5 {
        record_attempt("9.9.9.9");
    }
    assert_eq!(attempts_left("9.9.9.9", 5, lockout), 0);
}

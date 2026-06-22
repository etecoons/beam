use super::*;
use std::path::{Path, PathBuf};

#[test]
fn test_safe_compare() {
    assert!(security::safe_compare("1234", "1234"));
    assert!(!security::safe_compare("1234", "5678"));
    assert!(!security::safe_compare("1234", "12345"));
}

#[test]
fn test_normalize_path() {
    let path = Path::new("foo/bar/../baz");
    let norm = utils::normalize_path(path);
    assert_eq!(norm, PathBuf::from("foo/baz"));
}

#[test]
fn test_sanitize_filename_safe() {
    let clean = utils::sanitize_filename_safe("my file+name.txt");
    assert_eq!(clean, "my_file_name.txt");

    let clean_reserved = utils::sanitize_filename_safe("CON.txt");
    assert_eq!(clean_reserved, "CON_file.txt");

    let clean_empty = utils::sanitize_filename_safe("");
    assert_eq!(clean_empty, "unnamed_file.txt");
}

#[test]
fn test_sanitize_path_preserve_dirs_safe() {
    let clean = utils::sanitize_path_preserve_dirs_safe("folder/sub folder/my file.txt");
    assert_eq!(clean, "folder/sub_folder/my_file.txt");
}

#[test]
fn test_format_file_size() {
    assert_eq!(utils::format_file_size(500, None), "500.00B");
    assert_eq!(utils::format_file_size(2048, None), "2.00KB");
    assert_eq!(utils::format_file_size(2097152, Some("MB")), "2.00MB");
}

#[test]
fn test_is_valid_batch_id() {
    assert!(utils::is_valid_batch_id("1719000000-abcdef12"));
    assert!(!utils::is_valid_batch_id("1719000000-abc"));
    assert!(!utils::is_valid_batch_id("abc-abcdef12"));
}

#[test]
fn test_lockout_attempts() {
    let ip = "127.0.0.1";
    security::reset_attempts(ip);
    assert!(!security::is_locked_out(ip));

    for _ in 0..5 {
        let _ = security::record_attempt(ip);
    }
    assert!(security::is_locked_out(ip));

    security::reset_attempts(ip);
    assert!(!security::is_locked_out(ip));
}

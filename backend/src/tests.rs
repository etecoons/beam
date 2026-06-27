use super::*;
use std::path::{Path, PathBuf};
use std::net::{SocketAddr, SocketAddrV4};

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
fn test_sanitize_path_preserve_dirs_safe_rejects_parent_traversal() {
    // `..` in any position is now a hard reject (defense in depth).
    let clean = utils::sanitize_path_preserve_dirs_safe("foo/../etc/passwd");
    assert_eq!(
        clean, "unnamed_file.txt",
        "parent-traversal components must be rejected"
    );
    let clean2 = utils::sanitize_path_preserve_dirs_safe("../etc/passwd");
    assert_eq!(clean2, "unnamed_file.txt");
    // `..bar` (literal, no slash between) is technically a valid filename
    // *string*, but the sanitizer strips leading dots from each part to
    // prevent hidden files and to be conservative. The stem `..` becomes
    // empty after trimming, so the default `file` prefix kicks in, and
    // the extension `bar` is reattached. The test asserts the actual
    // behavior — leading-dot trickery always becomes a safe name.
    let clean3 = utils::sanitize_path_preserve_dirs_safe("foo/..bar/baz");
    assert_eq!(clean3, "foo/file.bar/baz", "leading dots are stripped from each part");
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

// ─────────────────────────────────────────────────────────────────────
// Path-traversal defense
// ─────────────────────────────────────────────────────────────────────

/// Test harness: create a temp upload dir with optional symlink, run the
/// check, and clean up.
fn check_under_upload(file_path: &Path, upload_dir: &Path) -> bool {
    utils::is_path_within_upload_dir(file_path, upload_dir, false)
}

#[test]
fn test_is_path_within_upload_dir_accepts_normal_paths() {
    let tmp = std::env::temp_dir().join(format!(
        "beam_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let existing_file = tmp.join("real_file.txt");
    std::fs::write(&existing_file, "ok").unwrap();

    assert!(check_under_upload(&existing_file, &tmp));
    // Inside an existing dir
    assert!(check_under_upload(&tmp.join("subdir/never_created.txt"), &tmp));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_is_path_within_upload_dir_rejects_absolute_paths() {
    let tmp = std::env::temp_dir().join("beam_test_abs");
    std::fs::create_dir_all(&tmp).unwrap();
    assert!(!check_under_upload(Path::new("/etc/passwd"), &tmp));
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_is_path_within_upload_dir_rejects_parent_traversal_in_nonexistent_suffix() {
    // Build an upload dir under the real temp dir. The target file
    // doesn't exist, and contains a `..` component.
    let tmp = std::env::temp_dir().join(format!(
        "beam_test_traversal_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let evil = tmp.join("legit_dir/../escape.txt");
    assert!(!check_under_upload(&evil, &tmp));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_is_path_within_upload_dir_rejects_when_parent_is_symlink_outside() {
    // This is the bug that motivated the fix: a symlinked subdir in the
    // upload dir, and an upload to a non-existent path under it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outer = std::env::temp_dir().join(format!(
            "beam_outer_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let upload = outer.join("uploads");
        std::fs::create_dir_all(&upload).unwrap();
        let outside = outer.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        // Create a symlink at uploads/escape -> outside
        let link = upload.join("escape");
        symlink(&outside, &link).unwrap();

        // An upload target inside the symlinked dir (doesn't yet exist).
        let evil_target = link.join("passwd");
        assert!(
            !check_under_upload(&evil_target, &upload),
            "symlinked subdir must be detected as outside the upload dir"
        );

        std::fs::remove_dir_all(&outer).ok();
    }
}

#[test]
fn test_is_path_within_upload_dir_require_exists_canonicalizes() {
    let tmp = std::env::temp_dir().join(format!(
        "beam_test_exists_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let real = tmp.join("ok.txt");
    std::fs::write(&real, "data").unwrap();
    assert!(utils::is_path_within_upload_dir(&real, &tmp, true));
    assert!(!utils::is_path_within_upload_dir(
        Path::new("/this/does/not/exist"),
        &tmp,
        true
    ));
    std::fs::remove_dir_all(&tmp).ok();
}

// ─────────────────────────────────────────────────────────────────────
// X-Forwarded-For defense
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_get_client_ip_ignores_xff_without_trusted_list() {
    use std::net::Ipv4Addr;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 4401));
    // The previous implementation honored XFF when trust_proxy=true
    // regardless of whether the allowlist was set. The new behavior
    // requires the allowlist; without it, fall back to the socket IP.
    let ip = security::get_client_ip(
        &headers,
        socket,
        true,
        None, // no trusted list
    );
    assert_eq!(ip, "10.0.0.1");
}

#[test]
fn test_get_client_ip_honors_xff_when_trusted_list_matches() {
    use std::net::Ipv4Addr;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 4401));
    let trusted = vec!["10.0.0.0/8".to_string()];
    let ip = security::get_client_ip(&headers, socket, true, Some(&trusted));
    assert_eq!(ip, "203.0.113.5");
}

#[test]
fn test_get_client_ip_ignores_xff_when_socket_not_in_trusted_list() {
    use std::net::Ipv4Addr;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 1), 4401));
    let trusted = vec!["10.0.0.0/8".to_string()];
    let ip = security::get_client_ip(&headers, socket, true, Some(&trusted));
    assert_eq!(ip, "192.168.1.1");
}

#[test]
fn test_get_client_ip_ignores_xff_when_trust_proxy_false() {
    use std::net::Ipv4Addr;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 4401));
    let ip = security::get_client_ip(&headers, socket, false, None);
    assert_eq!(ip, "10.0.0.1");
}

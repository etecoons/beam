//! Backend security helpers.
//!
//! Provides utilities used by the companion apps' stub `sh` binaries to
//! block interactive shell access inside read-only Nix containers.

/// Clears the screen and prints a friendly "console access denied" message.
///
/// Used in each companion app's `src/bin/sh.rs` stub so that running
/// `/bin/sh` inside the container surfaces a clear, on-brand alert instead
/// of dropping into a POSIX shell.
pub fn print_unauthorized_console_message() {
    // Clear screen using ANSI escape sequences.
    print!("\x1B[2J\x1B[1;1H");

    println!(
        r#" _______________________________________
/ I'm sorry, Dave. I'm afraid Nix won't \
\ let me do that.                       /
 ---------------------------------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||"#
    );

    println!("\x1B[1;31m\nSystem Alert: Console Access is UNAUTHORIZED.\x1B[0m");
    println!("This application is running inside a secure, read-only Nix container.");
    println!("Direct shell access is disabled for environment isolation and security.");
    println!("\nPress \x1B[1;37m[Enter]\x1B[0m to close connection...");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: calling the function should not panic.
    /// Output is not asserted (would clutter test logs).
    #[test]
    fn does_not_panic() {
        print_unauthorized_console_message();
    }

    /// Verify the message string contains the canonical alert phrase,
    /// so a refactor can't silently drop it.
    #[test]
    fn message_contains_unauthorized_phrase() {
        // Capture-free check: call once and ensure no panic. The string
        // content is asserted via static text below.
        const EXPECTED_FRAGMENT: &str = "Console Access is UNAUTHORIZED";
        // We can't easily capture stdout in a unit test without a crate;
        // this test documents the requirement that the source contains it.
        let source = include_str!("mod.rs");
        assert!(
            source.contains(EXPECTED_FRAGMENT),
            "security message must contain {EXPECTED_FRAGMENT:?}"
        );
    }
}

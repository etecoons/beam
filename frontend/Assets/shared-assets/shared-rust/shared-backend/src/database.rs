//! Common SQLite database utilities for companion applications.
//!
//! Provides functions to establish connection parameters, enable WAL mode,
//! and configure connection properties for optimal desktop/server runtime performance.

use rusqlite::{Connection, Result};
use std::path::Path;

/// Establishes an SQLite database connection at the specified file path,
/// enforcing foreign key constraints and enabling Write-Ahead Logging (WAL) mode
/// for concurrent read/write optimization.
pub fn establish_connection<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(path)?;

    // Enable Write-Ahead Logging (WAL) for better concurrent performance
    conn.pragma_update(None, "journal_mode", &"WAL")?;

    // Enforce relational foreign key constraints
    conn.pragma_update(None, "foreign_keys", &"ON")?;

    Ok(conn)
}

use std::fs;
use std::io::{self, Write};
use crate::status::{get_port, get_data_dir, get_pin, get_db_file_path};

pub fn run_doctor() {
    println!("=== {} Doctor Diagnostics ===", crate::APP_NAME);
    let mut failures = 0;

    let data_dir = get_data_dir();
    print!("Checking data directory {:?}... ", data_dir);
    let _ = io::stdout().flush();
    if let Err(e) = fs::create_dir_all(&data_dir) {
        println!("\x1B[1;31m[FAIL] (Cannot create directory: {})\x1B[0m", e);
        failures += 1;
    } else {
        let test_file = data_dir.join(".doctor_test");
        if fs::write(&test_file, b"test").is_err() {
            println!("\x1B[1;31m[FAIL] (Directory is not writable)\x1B[0m");
            failures += 1;
        } else {
            let _ = fs::remove_file(&test_file);
            println!("\x1B[1;32m[PASS]\x1B[0m");
        }
    }

    if let Some(path) = get_db_file_path() {
        print!("Checking database file {:?}... ", path);
        let _ = io::stdout().flush();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                        println!("\x1B[1;32m[PASS]\x1B[0m");
                    } else {
                        println!("\x1B[1;31m[FAIL] (Corrupted JSON content)\x1B[0m");
                        failures += 1;
                    }
                }
                Err(e) => {
                    println!("\x1B[1;31m[FAIL] (Cannot read file: {})\x1B[0m", e);
                    failures += 1;
                }
            }
        } else {
            println!("\x1B[1;33m[WARN] (File does not exist yet; will be created on start)\x1B[0m");
        }
    }

    let port = get_port();
    print!("Checking web port {} availability... ", port);
    let _ = io::stdout().flush();
    if std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
        println!("\x1B[1;32m[PASS] (Port is free)\x1B[0m");
    } else {
        println!("\x1B[1;33m[WARN] (Port is already bound; server is likely running)\x1B[0m");
    }

    print!("Checking Security PIN setup... ");
    let _ = io::stdout().flush();
    if get_pin().is_some() {
        println!("\x1B[1;32m[SET] (PIN is configured)\x1B[0m");
    } else {
        println!("\x1B[1;33m[WARN] (No PIN configured; authentication is disabled)\x1B[0m");
    }

    println!();
    if failures == 0 {
        println!("\x1B[1;32mDoctor report: System is healthy and configured correctly.\x1B[0m");
    } else {
        println!("\x1B[1;31mDoctor report: Found {} error(s). Please check logs or configurations.\x1B[0m", failures);
    }
}

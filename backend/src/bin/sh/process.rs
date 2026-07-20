use std::path::Path;
use std::process::Command;
use crate::status::get_port;

pub fn run_start() {
    println!("Starting {} server...", crate::APP_NAME);
    let port = get_port();
    
    if std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_err() {
        println!("\x1B[1;33mWarning: Port {} is already in use. Server is likely already running.\x1B[0m", port);
        println!("If you are inside a container, the server runs automatically.");
        return;
    }

    let server_path = if Path::new("/app/server").exists() {
        "/app/server"
    } else if Path::new("./target/release/server").exists() {
        "./target/release/server"
    } else if Path::new("./server").exists() {
        "./server"
    } else {
        "server"
    };

    println!("Spawning server process: {}", server_path);
    let child = match Command::new(server_path).spawn() {
        Ok(c) => c,
        Err(e) => {
            println!("\x1B[1;31mError: Failed to spawn server binary: {}\x1B[0m", e);
            return;
        }
    };

    println!("Server process started with PID {}.", child.id());
    println!("Console will now exit.");
}

pub fn run_end() {
    println!("Stopping {} server process...", crate::APP_NAME);
    let mut stopped = false;
    
    if Command::new("pkill").arg("-15").arg("server").status().is_ok() {
        println!("Sent SIGTERM shutdown signal to server processes.");
        stopped = true;
    } else if Command::new("kill").arg("-15").arg("1").status().is_ok() {
        println!("Sent SIGTERM to container init process (PID 1).");
        stopped = true;
    }
    
    if !stopped {
        println!("\x1B[1;31mError: Could not stop server. Process command 'pkill' / 'kill' failed.\x1B[0m");
    }
}

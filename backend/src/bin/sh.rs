use std::io::{self, BufRead};

fn main() {
    shared_backend::security::print_unauthorized_console_message();

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let _ = handle.read_line(&mut buffer);
    std::process::exit(0);
}

use crate::status::{print_env, print_status};
use crate::doctor::run_doctor;
use crate::data::{print_data_stats, list_data_contents, clear_data};
use crate::process::{run_start, run_end};

pub fn print_help() {
    println!("Usage: sh [command]");
    println!();
    println!("Commands:");
    println!("  version                   Show application version");
    println!("  status / info             Show service status and configurations");
    println!("  env                       Print loaded configuration environment variables");
    println!("  doctor / check / diagnose Perform system health and permission diagnostics");
    println!("  start / up / run          Launch backend server process if stopped");
    println!("  stop / down / end / close Gracefully terminate the backend server process");
    println!("  restart / reload          Restart the backend server process");
    println!("  data stats / size         Show database/storage file statistics");
    println!("  data list / show          List items stored in the database");
    println!("  data clear / reset        Delete the database to reset application state");
    println!("  help                      Show this help menu");
    println!();
    println!("Run without arguments to launch the interactive TUI console.");
}

pub fn handle_cli_args(args: &[String]) {
    let cmd = args[1].to_lowercase();
    match cmd.as_str() {
        "version" | "--version" | "-v" => {
            println!("{} version: v{}", crate::APP_NAME, env!("CARGO_PKG_VERSION"));
        }
        "status" | "--status" | "info" => {
            print_status();
        }
        "env" | "--env" => {
            print_env();
        }
        "doctor" | "check" | "diagnose" => {
            run_doctor();
        }
        "start" | "up" | "run" => {
            run_start();
        }
        "stop" | "down" | "end" | "close" => {
            run_end();
        }
        "restart" | "reload" => {
            run_end();
            run_start();
        }
        "data" => {
            if args.len() > 2 {
                let sub = args[2].to_lowercase();
                match sub.as_str() {
                    "stats" | "info" | "size" => print_data_stats(),
                    "list" | "show" | "view" => list_data_contents(),
                    "clear" | "reset" | "prune" => clear_data(),
                    _ => print_help(),
                }
            } else {
                print_help();
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            println!("Unknown command: '{}'. Use 'help' for usage.", args[1]);
            std::process::exit(1);
        }
    }
}

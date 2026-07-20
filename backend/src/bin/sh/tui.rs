use std::io::{self, Write, Read};
use std::process::Command;
use crate::status::{get_port, get_data_dir, get_pin, print_status, print_env};
use crate::doctor::run_doctor;
use crate::data::{print_data_stats, list_data_contents, clear_data};

pub struct RawMode;

impl RawMode {
    pub fn enable() -> Self {
        let mut cmd = Command::new("stty");
        cmd.arg("raw").arg("-echo");
        let _ = cmd.status();
        print!("\x1B[?25l");
        let _ = io::stdout().flush();
        RawMode
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let mut cmd = Command::new("stty");
        cmd.arg("-raw").arg("echo");
        let _ = cmd.status();
        print!("\x1B[?25h\x1B[0m");
        let _ = io::stdout().flush();
    }
}

pub fn run_tui() {
    let _raw = RawMode::enable();
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut menu_selection = 0;
    loop {
        print!("\x1B[2J\x1B[1;1H");

        let clr_border = "\x1B[38;5;171m";
        let clr_title = "\x1B[38;5;51m\x1B[1m";
        let clr_label = "\x1B[38;5;45m\x1B[1m";
        let clr_reset = "\x1B[0m";
        let clr_running = "\x1B[32;1m";
        let clr_disabled = "\x1B[33m";
        let clr_enabled = "\x1B[32m";

        println!("{}╔══════════════════════════════════════════════════════════╗{}", clr_border, clr_reset);
        let title = format!("{} Administration Console", crate::APP_NAME);
        println!("{}║{}  {:^52}  {}║{}", clr_border, clr_title, title, clr_border, clr_reset);
        println!("{}╠══════════════════════════════════════════════════════════╣{}", clr_border, clr_reset);

        let status_str = format!("{}● Running{}", clr_running, clr_reset);
        println!("{}║{}  {}Status:{}          {:<38}{}║{}", clr_border, clr_reset, clr_label, clr_reset, status_str, clr_border, clr_reset);
        println!("{}║{}  {}Web Port:{}        {:<28}{}║{}", clr_border, clr_reset, clr_label, clr_reset, get_port(), clr_border, clr_reset);
        
        let data_dir_str = format!("{:?}", get_data_dir());
        let data_dir_truncated = if data_dir_str.len() > 38 {
            format!("{}...", &data_dir_str[..35])
        } else {
            data_dir_str
        };
        println!("{}║{}  {}Data Directory:{}  {:<38}{}║{}", clr_border, clr_reset, clr_label, clr_reset, data_dir_truncated, clr_border, clr_reset);

        let pin_str = if get_pin().is_some() {
            format!("{}🔒 Enabled (PIN Auth Active){}", clr_enabled, clr_reset)
        } else {
            format!("{}🔓 Disabled (No Auth Active){}", clr_disabled, clr_reset)
        };
        println!("{}║{}  {}Security PIN:{}    {:<38}{}║{}", clr_border, clr_reset, clr_label, clr_reset, pin_str, clr_border, clr_reset);

        println!("{}╠══════════════════════════════════════════════════════════╣{}", clr_border, clr_reset);
        println!("{}║{}  Select an option:                                      {}║{}", clr_border, clr_reset, clr_border, clr_reset);
        println!("{}║{}                                                        {}║{}", clr_border, clr_reset, clr_border, clr_reset);

        let options = [
            ("⚙ ", "Show Full Configuration Settings"),
            ("🩺", "Run System Diagnostics (Doctor)"),
            ("📊", "View Database Statistics"),
            ("📂", "List Database/Files Content"),
            ("🗑 ", "Reset / Clear Application State"),
            ("❌", "Exit Console")
        ];

        for (i, (icon, opt)) in options.iter().enumerate() {
            if i == menu_selection {
                let opt_line = format!("  ➔  [ {} ]  {}", icon, opt);
                let padding_spaces = 50 - (opt_line.chars().count() + 1);
                let padded_opt_line = format!("{}{}", opt_line, " ".repeat(padding_spaces));
                println!("{}║{}  \x1B[48;5;93m\x1B[37;1m{}\x1B[0m  {}║{}", clr_border, clr_reset, padded_opt_line, clr_border, clr_reset);
            } else {
                let opt_line = format!("     [ {} ]  {}", icon, opt);
                let padding_spaces = 50 - (opt_line.chars().count() + 1);
                println!("{}║{}  {}{}  {}║{}", clr_border, clr_reset, opt_line, " ".repeat(padding_spaces), clr_border, clr_reset);
            }
        }

        println!("{}║{}                                                        {}║{}", clr_border, clr_reset, clr_border, clr_reset);
        println!("{}╚══════════════════════════════════════════════════════════╝{}", clr_border, clr_reset);

        println!();
        println!("  ⌨  Use \x1B[1m[Up/Down Arrow]\x1B[0m to navigate • \x1B[1m[Enter]\x1B[0m to select • \x1B[1m[Q]\x1B[0m to quit");
        let _ = stdout.flush();

        let mut key_buf = [0u8; 3];
        let bytes_read = match stdin.read(&mut key_buf) {
            Ok(n) => n,
            Err(_) => break,
        };

        if bytes_read == 1 {
            match key_buf[0] {
                13 | 10 => {
                    if menu_selection == 5 {
                        break;
                    }
                    execute_tui_option(menu_selection);
                }
                b'1' => execute_tui_option(0),
                b'2' => execute_tui_option(1),
                b'3' => execute_tui_option(2),
                b'4' => execute_tui_option(3),
                b'5' => execute_tui_option(4),
                b'6' => break,
                b'q' | b'Q' => break,
                _ => {}
            }
        } else if bytes_read == 3 && key_buf[0] == 27 && key_buf[1] == 91 {
            match key_buf[2] {
                65 => {
                    if menu_selection > 0 {
                        menu_selection -= 1;
                    } else {
                        menu_selection = 5;
                    }
                }
                66 => {
                    if menu_selection < 5 {
                        menu_selection += 1;
                    } else {
                        menu_selection = 0;
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn execute_tui_option(index: usize) {
    let mut cmd = Command::new("stty");
    cmd.arg("-raw").arg("echo");
    let _ = cmd.status();
    print!("\x1B[?25h\x1B[0m");
    let _ = io::stdout().flush();

    print!("\x1B[2J\x1B[1;1H");

    let clr_border = "\x1B[38;5;171m";
    let clr_title = "\x1B[38;5;51m\x1B[1m";
    let clr_reset = "\x1B[0m";

    let headers = [
        "CONFIGURATION SETTINGS REPORT",
        "SYSTEM DIAGNOSTICS REPORT (DOCTOR)",
        "DATABASE STATISTICS REPORT",
        "DATABASE FILE CONTENT LIST",
        "RESET APPLICATION STATE"
    ];

    if index < 5 {
        println!("{}┌────────────────────────────────────────────────────────┐{}", clr_border, clr_reset);
        println!("{}│{}  {:^52}  {}│{}", clr_border, clr_title, headers[index], clr_border, clr_reset);
        println!("{}└────────────────────────────────────────────────────────┘{}", clr_border, clr_reset);
        println!();
    }

    match index {
        0 => {
            print_status();
            println!();
            print_env();
        }
        1 => {
            run_doctor();
        }
        2 => {
            print_data_stats();
        }
        3 => {
            list_data_contents();
        }
        4 => {
            clear_data();
        }
        _ => {}
    }

    println!("\nPress \x1B[1;36m[Enter]\x1B[0m to return to menu...");
    let mut discard = String::new();
    let _ = io::stdin().read_line(&mut discard);

    let mut cmd = Command::new("stty");
    cmd.arg("raw").arg("-echo");
    let _ = cmd.status();
    print!("\x1B[?25l");
    let _ = io::stdout().flush();
}

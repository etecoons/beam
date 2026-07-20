use std::fs;
use std::io::{self, Write};
use crate::status::{get_data_dir, get_db_file_path};

pub fn print_data_stats() {
    println!("=== {} Data Statistics ===", crate::APP_NAME);
    let data_dir = get_data_dir();
    println!("Database Directory: {:?}", data_dir);

    if crate::APP_NAME == "Beam" {
        let uploads_dir = data_dir.parent().unwrap_or(&data_dir).join("uploads");
        println!("Uploads Directory: {:?}", uploads_dir);
        if let Ok(entries) = fs::read_dir(&uploads_dir) {
            let mut count = 0;
            let mut total_size = 0;
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        count += 1;
                        total_size += meta.len();
                    }
                }
            }
            println!("Uploaded Files: {}", count);
            println!("Total Space Used: {} bytes", total_size);
        } else {
            println!("Uploads Directory: not found or unreadable");
        }
    } else if let Some(path) = get_db_file_path() {
        if path.exists() {
            if let Ok(meta) = fs::metadata(&path) {
                println!("Database File: {:?}", path);
                println!("File Size: {} bytes", meta.len());
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if value.is_object() {
                            println!("Number of database keys: {}", value.as_object().unwrap().keys().len());
                        } else if value.is_array() {
                            println!("Number of database entries: {}", value.as_array().unwrap().len());
                        }
                    }
                }
            }
        } else {
            println!("Database file not found at {:?}", path);
        }
    } else {
        println!("This application is stateless and does not maintain a JSON database.");
    }
}

pub fn list_data_contents() {
    println!("=== {} Data Contents ===", crate::APP_NAME);
    let data_dir = get_data_dir();
    if crate::APP_NAME == "Beam" {
        let uploads_dir = data_dir.parent().unwrap_or(&data_dir).join("uploads");
        if let Ok(entries) = fs::read_dir(&uploads_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        println!(" - {} ({} bytes)", name, meta.len());
                    }
                }
            }
        } else {
            println!("No uploads found.");
        }
        return;
    }

    let path = match get_db_file_path() {
        Some(p) => p,
        None => {
            println!("This application is stateless and has no database contents to list.");
            return;
        }
    };

    if !path.exists() {
        println!("Database file not found.");
        return;
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            println!("Failed to read database file.");
            return;
        }
    };

    match crate::APP_NAME {
        "Todo" => {
            if let Ok(lists) = serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&content) {
                for (list_name, items) in lists {
                    println!("List: {}", list_name);
                    if let Some(arr) = items.as_array() {
                        for item in arr {
                            let text = item["text"].as_str().unwrap_or("");
                            let completed = item["completed"].as_bool().unwrap_or(false);
                            let marker = if completed { "[x]" } else { "[ ]" };
                            println!("  {} {}", marker, text);
                        }
                    }
                }
            } else {
                println!("Failed to parse todos database.");
            }
        }
        "Grid" => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(boards) = value["boards"].as_object() {
                    for (board_name, board) in boards {
                        println!("Board: {}", board_name);
                        if let Some(tasks) = board["tasks"].as_object() {
                            for (task_id, task) in tasks {
                                let title = task["title"].as_str().unwrap_or("");
                                let column = task["column"].as_str().unwrap_or("unknown");
                                println!("  - [{}] {} ({})", task_id, title, column);
                            }
                        }
                    }
                }
            } else {
                println!("Failed to parse grid tasks database.");
            }
        }
        "Pad" => {
            if let Ok(pads) = serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&content) {
                for (pad_id, pad) in pads {
                    let text = pad["content"].as_str().unwrap_or("");
                    println!("Pad ID: {} ({} chars)", pad_id, text.len());
                }
            } else {
                println!("Failed to parse pads database.");
            }
        }
        "Defend" | "Snake" | "Scan" => {
            if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                println!("Top Leaderboard Scores:");
                for (idx, entry) in entries.iter().enumerate() {
                    let name = entry["name"].as_str().unwrap_or("Anonymous");
                    let score = entry["score"].as_u64().unwrap_or(0);
                    println!("  {}. {} - {} pts", idx + 1, name, score);
                }
            } else {
                println!("Failed to parse leaderboard.");
            }
        }
        _ => {
            println!("{}", content);
        }
    }
}

pub fn clear_data() {
    print!("Are you sure you want to clear/reset the database? (y/N): ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if trimmed == "y" || trimmed == "yes" {
            if crate::APP_NAME == "Beam" {
                let uploads_dir = get_data_dir().parent().unwrap_or(&get_data_dir()).join("uploads");
                let _ = fs::remove_dir_all(&uploads_dir);
                let _ = fs::create_dir_all(&uploads_dir);
                println!("Uploads directory cleared.");
            } else if let Some(path) = get_db_file_path() {
                let empty_content = match crate::APP_NAME {
                    "Defend" | "Snake" | "Scan" => "[]",
                    _ => "{}",
                };
                if fs::write(&path, empty_content).is_ok() {
                    println!("Database cleared successfully.");
                } else {
                    println!("Failed to write empty database.");
                }
            } else {
                println!("This application is stateless; no database to clear.");
            }
        } else {
            println!("Clear cancelled.");
        }
    }
}

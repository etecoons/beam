use std::env;
use std::path::PathBuf;

pub fn get_port() -> u16 {
    env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(match crate::APP_NAME {
            "Beam" => 4401,
            "Defend" => 4504,
            "Grid" => 4405,
            "Pad" => 4402,
            "Pulse" => 4406,
            "Rustle" => 4502,
            "Scan" => 4503,
            "Snake" => 4501,
            "Todo" => 4403,
            "Trace" => 4404,
            _ => 8080,
        })
}

pub fn get_data_dir() -> PathBuf {
    let key1 = format!("{}_DATA_DIR", crate::ENV_PREFIX);
    let key2 = format!("{}_DATA_PATH", crate::ENV_PREFIX);
    let dir = env::var(&key1)
        .or_else(|_| env::var(&key2))
        .or_else(|_| env::var("DATA_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    PathBuf::from(dir)
}

pub fn get_pin() -> Option<String> {
    let key = format!("{}_PIN", crate::ENV_PREFIX);
    env::var(&key).or_else(|_| env::var("PIN")).ok()
}

pub fn get_allowed_origins() -> String {
    let key = format!("{}_ALLOWED_ORIGINS", crate::ENV_PREFIX);
    env::var(&key).or_else(|_| env::var("ALLOWED_ORIGINS")).unwrap_or_else(|_| "*".to_string())
}

pub fn get_db_file_path() -> Option<PathBuf> {
    if crate::DB_FILE_NAME.is_empty() {
        None
    } else {
        Some(get_data_dir().join(crate::DB_FILE_NAME))
    }
}

pub fn print_status() {
    println!("=== {} Status ===", crate::APP_NAME);
    println!("Port: {}", get_port());
    println!("Data Directory: {:?}", get_data_dir());
    println!("PIN Enabled: {}", get_pin().is_some());
    println!("Allowed Origins: {}", get_allowed_origins());
    println!("Timezone: {}", env::var("TZ").unwrap_or_else(|_| "UTC".to_string()));
}

pub fn print_env() {
    println!("=== {} Environment Variables ===", crate::APP_NAME);
    for (key, val) in env::vars() {
        if key.starts_with(crate::ENV_PREFIX) || key == "PORT" || key == "DATA_DIR" || key == "TZ" || key == "ALLOWED_ORIGINS" {
            println!("{}={}", key, val);
        }
    }
}

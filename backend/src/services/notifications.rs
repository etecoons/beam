use std::process::Command;
use crate::config::AppConfig;
use crate::utils::{format_file_size, calculate_directory_size, sanitize_filename_safe};

pub async fn send_notification(filename: &str, file_size: u64, config: &AppConfig) {
    let Some(ref apprise_url) = config.apprise_url else {
        return;
    };

    let formatted_size = format_file_size(file_size, config.apprise_size_unit.as_deref());
    let dir_size = calculate_directory_size(&config.upload_dir);
    let total_storage = format_file_size(dir_size, None);

    let sanitized_filename = sanitize_filename_safe(filename);

    let message = config.apprise_message
        .replace("{filename}", &sanitized_filename)
        .replace("{size}", &formatted_size)
        .replace("{storage}", &total_storage);

    tracing::info!("Sending notification via Apprise for file: {}", sanitized_filename);

    let url_clone = apprise_url.clone();
    tokio::task::spawn_blocking(move || {
        match Command::new("apprise")
            .args([&url_clone, "-b", &message])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    tracing::info!("Notification sent successfully: {}", String::from_utf8_lossy(&output.stdout).trim());
                } else {
                    tracing::error!("Apprise exited with status: {}. Error: {}", output.status, String::from_utf8_lossy(&output.stderr).trim());
                }
            }
            Err(e) => {
                tracing::error!("Failed to run apprise notification: {}", e);
            }
        }
    });
}

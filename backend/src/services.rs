use crate::config::AppConfig;
use std::sync::Arc;

pub async fn send_notification(filename: &str, size_bytes: u64, config: &Arc<AppConfig>) {
    let url = match &config.apprise_url {
        Some(u) => u,
        None => return,
    };

    let formatted_size =
        crate::utils::format_file_size(size_bytes, config.apprise_size_unit.as_deref());

    // Gather storage details if limit exists
    let storage_str = if let Some(limit) = config.max_storage_limit {
        let items = crate::routes::files::helpers::get_directory_contents(&config.upload_dir, "")
            .unwrap_or_default();
        let total_size = crate::routes::files::helpers::calculate_total_size(&items);
        let used_pct = if limit == 0 {
            0.0
        } else {
            (total_size as f64 / limit as f64) * 100.0
        };
        format!(
            "{} of {} ({:.1}%)",
            crate::utils::format_file_size(total_size, None),
            crate::utils::format_file_size(limit, None),
            used_pct
        )
    } else {
        "Unrestricted".to_string()
    };

    let message = config
        .apprise_message
        .replace("{filename}", filename)
        .replace("{size}", &formatted_size)
        .replace("{storage}", &storage_str);

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "urls": url,
        "body": message,
        "title": format!("{} Notification", config.site_title),
    });

    tracing::info!("Sending notification via Apprise to URL: {}", url);
    match client
        .post("https://api.apprise.io/notify")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Notification sent successfully.");
            } else {
                tracing::error!("Apprise API returned error status: {:?}", resp.status());
            }
        }
        Err(e) => {
            tracing::error!("Failed to connect to Apprise API: {}", e);
        }
    }
}

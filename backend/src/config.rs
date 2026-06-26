use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub node_env: String,
    pub base_url: String,
    pub upload_dir: PathBuf,
    pub max_file_size: u64,
    pub auto_upload: bool,
    pub show_file_list: bool,
    pub pin: Option<String>,
    pub trust_proxy: bool,
    pub trusted_proxy_ips: Option<Vec<String>>,
    pub site_title: String,
    pub allowed_extensions: Option<Vec<String>>,
    pub client_max_retries: u32,
    pub max_storage_limit: Option<u64>,
    pub retention_period_days: Option<u64>,
    pub allowed_origins: String,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
}

impl AppConfig {
    pub fn load() -> Self {
        let _ = dotenvy::from_path("/app/data/.env");
        let _ = dotenvy::dotenv();

        let port: u16 = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4401);

        let node_env = env::var("NODE_ENV").unwrap_or_else(|_| "production".to_string());

        let mut base_url =
            env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());

        // Determine upload directory
        let upload_dir = if let Ok(dir) = env::var("UPLOAD_DIR") {
            if !dir.is_empty() {
                PathBuf::from(dir)
            } else {
                env::var("LOCAL_UPLOAD_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("./local_uploads"))
            }
        } else {
            env::var("LOCAL_UPLOAD_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./local_uploads"))
        };

        let max_file_size = env::var("MAX_FILE_SIZE")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1024)
            * 1024
            * 1024; // MB to bytes

        let auto_upload = env::var("AUTO_UPLOAD")
            .map(|val| val == "true")
            .unwrap_or(false);

        let show_file_list = env::var("SHOW_FILE_LIST")
            .map(|val| val == "true")
            .unwrap_or(false);

        // Accept BEAM_PIN or PIN
        let pin = env::var("BEAM_PIN")
            .or_else(|_| env::var("PIN"))
            .ok()
            .filter(|p| {
                !p.is_empty()
                    && p.chars().all(|c| c.is_ascii_digit())
                    && p.len() >= 4
                    && p.len() <= 10
            });

        let trust_proxy = env::var("TRUST_PROXY")
            .map(|val| val == "true")
            .unwrap_or(false);

        let trusted_proxy_ips = env::var("TRUSTED_PROXY_IPS")
            .ok()
            .map(|ips| ips.split(',').map(|ip| ip.trim().to_string()).collect());

        // Accept BEAM_TITLE, BEAM_SITE_TITLE, or SITE_TITLE
        let site_title = env::var("BEAM_TITLE")
            .or_else(|_| env::var("BEAM_SITE_TITLE"))
            .or_else(|_| env::var("SITE_TITLE"))
            .unwrap_or_else(|_| "Beam".to_string());

        let allowed_extensions = env::var("ALLOWED_EXTENSIONS")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|exts| {
                exts.split(',')
                    .map(|ext| {
                        let mut trimmed = ext.trim().to_lowercase();
                        if !trimmed.starts_with('.') {
                            trimmed.insert(0, '.');
                        }
                        trimmed
                    })
                    .collect()
            });

        let client_max_retries = env::var("CLIENT_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(5);

        let max_storage_limit = env::var("MAX_STORAGE_LIMIT_GB")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(|gb| gb * 1024 * 1024 * 1024);

        let retention_period_days = env::var("RETENTION_PERIOD_DAYS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok());

        let enable_translation = env::var("ENABLE_TRANSLATION")
            .map(|v| v == "true" || v == "on")
            .unwrap_or(false);

        let enable_themes = env::var("ENABLE_THEMES")
            .map(|v| v == "true" || v == "on").unwrap_or(false);

        let enable_print = env::var("ENABLE_PRINT")
            .map(|v| v == "true" || v == "on").unwrap_or(false);

        Self {
            port,
            node_env,
            base_url,
            upload_dir,
            max_file_size,
            auto_upload,
            show_file_list,
            pin,
            trust_proxy,
            trusted_proxy_ips,
            site_title,
            allowed_extensions,
            client_max_retries,
            max_storage_limit,
            retention_period_days,
            allowed_origins,
            enable_translation,
            enable_themes,
            enable_print,
        }
    }
}

use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub base_url: String,
    pub storage_root: String,
    pub admin_email: String,
    pub admin_bootstrap_password: Option<String>,
    pub web_origins: Vec<String>,
    pub http_max_body_bytes: usize,
    pub token_length: usize,
    pub fcm_service_account_json: Option<String>,
    pub github_token: Option<String>,
    pub max_releases_per_channel: usize,
    pub github_poll_interval_secs: u64,
    pub trusted_proxy_hops: usize,
    pub pin_service_url: String,
}

/// `.env`/compose files pass unset variables through as an empty string
/// rather than omitting them, so a plain `.ok()` would treat `VAR=` as "set
/// to the empty password" instead of "use the generated one".
fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

impl Config {
    pub fn from_env() -> Self {
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/kobup.db?mode=rwc".into());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let base_url =
            env::var("KOBUP_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
        let storage_root = env::var("STORAGE_ROOT").unwrap_or_else(|_| "data".into());
        let admin_email = env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@localhost".into());
        let admin_bootstrap_password = non_blank(env::var("ADMIN_BOOTSTRAP_PASSWORD").ok());
        let web_origins = env::var("KOBUP_WEB_ORIGINS")
            .ok()
            .map(|val| {
                val.split(',')
                    .map(|o| o.trim().to_string())
                    .filter(|o| !o.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let http_max_body_bytes = env::var("HTTP_MAX_BODY_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500 * 1024 * 1024); // 500MB for APK uploads
        let token_length = env::var("TOKEN_LENGTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(32);
        let fcm_service_account_json = env::var("FCM_SERVICE_ACCOUNT_JSON").ok();
        let github_token = env::var("KOBUP_GITHUB_TOKEN").ok();
        let max_releases_per_channel = env::var("MAX_RELEASES_PER_CHANNEL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        let github_poll_interval_secs = env::var("GITHUB_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600);
        let trusted_proxy_hops = env::var("KOBUP_TRUSTED_PROXY_HOPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let pin_service_url = non_blank(env::var("KOBUP_PIN_SERVICE_URL").ok())
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://pin.biboop.com".into());

        Self {
            database_url,
            bind_addr,
            base_url,
            storage_root,
            admin_email,
            admin_bootstrap_password,
            web_origins,
            http_max_body_bytes,
            token_length,
            fcm_service_account_json,
            github_token,
            max_releases_per_channel,
            github_poll_interval_secs,
            trusted_proxy_hops,
            pin_service_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_bootstrap_password_env_var_is_treated_as_unset() {
        unsafe {
            env::set_var("ADMIN_BOOTSTRAP_PASSWORD", "");
        }
        let config = Config::from_env();
        unsafe {
            env::remove_var("ADMIN_BOOTSTRAP_PASSWORD");
        }
        assert_eq!(config.admin_bootstrap_password, None);
    }
}

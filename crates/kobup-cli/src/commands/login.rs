use indicatif::ProgressBar;
use serde::Deserialize;
use std::time::{Duration, Instant};

use crate::config::UserConfig;
use crate::http;

const NAMESPACE: &str = "kobup";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(300);
const BACKOFF: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct PinResponse {
    pin: String,
    result: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct LoginResult {
    api_url: String,
    token: String,
}

fn pin_service() -> String {
    std::env::var("KOBUP_PIN_SERVICE_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://pin.biboop.com".into())
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = http::client()?;
    let pin_service = pin_service();

    // Create a new PIN
    let resp: PinResponse = client
        .post(format!("{pin_service}/pin/{NAMESPACE}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut current_pin = resp.pin;
    println!(
        "Enter this PIN in the Kobup web admin under 'Login CLI': {}",
        current_pin
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Waiting for login...");
    spinner.enable_steady_tick(Duration::from_millis(120));

    let start = Instant::now();

    loop {
        if start.elapsed() > TIMEOUT {
            spinner.finish_and_clear();
            return Err("Login timed out after 5 minutes".into());
        }

        tokio::time::sleep(POLL_INTERVAL).await;

        let resp = client
            .post(format!("{pin_service}/pin/{NAMESPACE}/{current_pin}"))
            .send()
            .await?;

        // The service reports an expired or already-consumed PIN as 404 and no
        // longer mints a replacement, so allocate one and re-prompt.
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            let fresh: PinResponse = client
                .post(format!("{pin_service}/pin/{NAMESPACE}"))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            current_pin = fresh.pin;
            spinner.set_message(format!("PIN expired. New PIN: {current_pin}"));
            println!(
                "PIN expired. Enter new PIN in the Kobup web admin: {}",
                current_pin
            );
            continue;
        }

        // Guess throttling on the service is shared across callers; back off
        // rather than failing the login.
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            spinner.set_message("Rate limited by PIN service; retrying...");
            tokio::time::sleep(BACKOFF).await;
            continue;
        }

        let resp: PinResponse = resp.error_for_status()?.json().await?;

        if let Some(result) = resp.result {
            spinner.finish_and_clear();
            let login: LoginResult = serde_json::from_value(result)?;

            let mut config = UserConfig::load()?;
            config.server = Some(login.api_url.clone());
            config.token = Some(login.token);
            config.save()?;

            println!("Logged in to {}", login.api_url);
            return Ok(());
        }
    }
}

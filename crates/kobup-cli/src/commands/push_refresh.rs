use crate::config::{ProjectConfig, UserConfig};
use crate::http;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let user_config = UserConfig::load()?;
    let server = user_config
        .server
        .as_deref()
        .ok_or("Not logged in. Run `kobup login` first.")?;
    let token = user_config
        .token
        .as_deref()
        .ok_or("Not logged in. Run `kobup login` first.")?;

    let project_config =
        ProjectConfig::load().map_err(|_| "No .kobup.json found. Run `kobup init` first.")?;

    let slug = &project_config.project_slug;
    let client = http::client()?;

    let resp = client
        .post(format!("{server}/api/v1/ci/projects/{slug}/push-refresh"))
        .header("authorization", http::auth_header(token))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server error ({status}): {body}").into());
    }

    println!("Push refresh sent for {slug}");
    Ok(())
}

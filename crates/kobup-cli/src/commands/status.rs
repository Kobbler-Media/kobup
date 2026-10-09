use crate::config::{ProjectConfig, UserConfig};
use crate::http;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let user_config = UserConfig::load()?;

    println!("=== User Config ===");
    match &user_config.server {
        Some(server) => println!("  Server: {server}"),
        None => println!("  Server: (not set)"),
    }
    match &user_config.token {
        Some(token) => {
            let masked = if token.len() > 8 {
                format!("{}...{}", &token[..4], &token[token.len() - 4..])
            } else {
                "****".to_string()
            };
            println!("  Token:  {masked}");
        }
        None => println!("  Token:  (not set)"),
    }

    println!();
    println!("=== Project Config ===");
    match ProjectConfig::load() {
        Ok(project) => {
            println!("  Slug:    {}", project.project_slug);
            println!("  Name:    {}", project.project_name);
            println!("  Package: {}", project.package_name);
            println!("  Channel: {}", project.channel);
            println!("  Server:  {}", project.server);
        }
        Err(_) => {
            println!("  No .kobup.json found in current directory");
        }
    }

    // If logged in and project configured, fetch catalog
    if let (Some(server), Some(token)) = (&user_config.server, &user_config.token)
        && let Ok(project) = ProjectConfig::load()
    {
        println!();
        println!("=== Latest Releases ===");
        let client = http::client()?;
        let resp = client
            .get(format!("{server}/api/v1/catalog"))
            .header("authorization", http::auth_header(token))
            .send()
            .await?;

        if resp.status().is_success() {
            let catalog: serde_json::Value = resp.json().await?;
            if let Some(apps) = catalog.as_array() {
                let matching: Vec<_> = apps
                    .iter()
                    .filter(|app| {
                        app.get("slug")
                            .and_then(|s| s.as_str())
                            .is_some_and(|s| s == project.project_slug)
                    })
                    .collect();

                if matching.is_empty() {
                    println!("  No releases found for {}", project.project_slug);
                } else {
                    for app in matching {
                        let version = app
                            .get("latestVersion")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let channel = app
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        println!("  {channel}: v{version}");
                    }
                }
            }
        } else {
            println!("  Could not fetch catalog: {}", resp.status());
        }
    }

    Ok(())
}

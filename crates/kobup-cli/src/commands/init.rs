use clap::Args;
use serde::Deserialize;

use crate::config::{ProjectConfig, UserConfig};
use crate::http;

#[derive(Args)]
pub struct InitArgs {
    /// Project slug
    #[arg(long)]
    pub project: Option<String>,
    /// Display name
    #[arg(long)]
    pub name: Option<String>,
    /// Android package name
    #[arg(long, name = "package")]
    pub package_name: Option<String>,
    /// Release channel
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Infer from Android project directory
    #[arg(long, name = "from-android-app")]
    pub from_android_app: Option<String>,
}

#[derive(Deserialize)]
struct ProjectResponse {
    slug: String,
}

fn try_parse_gradle_package(dir: &str) -> Option<String> {
    let candidates = [
        format!("{dir}/app/build.gradle.kts"),
        format!("{dir}/app/build.gradle"),
        format!("{dir}/build.gradle.kts"),
        format!("{dir}/build.gradle"),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            // Try applicationId first, then namespace
            for pattern in &["applicationId", "namespace"] {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains(pattern) {
                        // Extract quoted value: applicationId "com.example.app" or applicationId = "com.example.app"
                        if let Some(start) = trimmed.find('"')
                            && let Some(end) = trimmed[start + 1..].find('"')
                        {
                            return Some(trimmed[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn prompt_string(
    prompt: &str,
    default: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut builder = dialoguer::Input::<String>::new().with_prompt(prompt);
    if let Some(d) = default {
        builder = builder.default(d.to_string());
    }
    Ok(builder.interact_text()?)
}

pub async fn run(args: InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let user_config = UserConfig::load()?;
    let server = user_config
        .server
        .as_deref()
        .ok_or("Not logged in. Run `kobup login` first.")?;
    let token = user_config
        .token
        .as_deref()
        .ok_or("Not logged in. Run `kobup login` first.")?;

    // Try to infer package name from gradle
    let gradle_package = args
        .from_android_app
        .as_deref()
        .and_then(try_parse_gradle_package);

    let slug = match args.project {
        Some(v) => v,
        None => prompt_string("Project slug", None)?,
    };
    let name = match args.name {
        Some(v) => v,
        None => prompt_string("Display name", Some(&slug))?,
    };
    let package_name = match args.package_name {
        Some(v) => v,
        None => prompt_string("Package name", gradle_package.as_deref())?,
    };

    let client = http::client()?;
    let resp = client
        .post(format!("{server}/api/v1/admin/projects"))
        .header("authorization", http::auth_header(token))
        .json(&serde_json::json!({
            "slug": slug,
            "name": name,
            "packageName": package_name,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server error ({status}): {body}").into());
    }

    let project: ProjectResponse = resp.json().await?;

    let config = ProjectConfig {
        project_slug: project.slug.clone(),
        project_name: name,
        package_name,
        channel: args.channel,
        server: server.to_string(),
    };
    config.save()?;

    println!("Project initialized: {}", project.slug);
    Ok(())
}

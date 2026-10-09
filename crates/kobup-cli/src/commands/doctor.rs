use crate::config::{ProjectConfig, UserConfig};
use crate::http;

fn check(ok: bool, label: &str) {
    if ok {
        println!("  \u{2713} {label}");
    } else {
        println!("  \u{2717} {label}");
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kobup Doctor");
    println!();

    let user_config = UserConfig::load()?;
    let has_server = user_config.server.is_some();
    let has_token = user_config.token.is_some();

    check(has_server && has_token, "User config has server and token");

    // Check token validity
    if let (Some(server), Some(token)) = (&user_config.server, &user_config.token) {
        let client = http::client()?;
        let resp = client
            .get(format!("{server}/health"))
            .header("authorization", http::auth_header(token))
            .send()
            .await;

        match resp {
            Ok(r) => check(r.status().is_success(), "Server is reachable"),
            Err(e) => check(false, &format!("Server is reachable ({e})")),
        }
    } else {
        check(false, "Server is reachable (not logged in)");
    }

    let has_project = ProjectConfig::load().is_ok();
    check(has_project, ".kobup.json exists");

    if let Ok(project) = ProjectConfig::load()
        && let (Some(server), Some(token)) = (&user_config.server, &user_config.token)
    {
        let client = http::client()?;
        let resp = client
            .get(format!(
                "{server}/api/v1/admin/projects/{}",
                project.project_slug
            ))
            .header("authorization", http::auth_header(token))
            .send()
            .await;

        match resp {
            Ok(r) => check(
                r.status().is_success(),
                &format!("Project '{}' exists on server", project.project_slug),
            ),
            Err(e) => check(
                false,
                &format!("Project '{}' exists on server ({e})", project.project_slug),
            ),
        }
    }

    Ok(())
}

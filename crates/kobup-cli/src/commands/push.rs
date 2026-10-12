use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::config::{ProjectConfig, UserConfig};
use crate::{dex_metadata, http};

#[derive(Args)]
pub struct PushArgs {
    /// APK file path(s)
    #[arg(long = "apk")]
    pub apk_paths: Vec<String>,
    /// Release channel
    #[arg(long)]
    pub channel: Option<String>,
    /// Release notes
    #[arg(long)]
    pub notes: Option<String>,
    /// Publish immediately
    #[arg(long)]
    pub publish: bool,
    /// Trigger push refresh after upload
    #[arg(long)]
    pub push_refresh: bool,
    /// Build via Gradle before pushing
    #[arg(long)]
    pub build_gradle: bool,
    /// Gradle build variant
    #[arg(long, default_value = "release")]
    pub variant: String,
}

fn find_gradle_apks(variant: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let search_dir = format!("app/build/outputs/apk/{variant}");
    let pattern = format!("{search_dir}/*.apk");

    let mut apks: Vec<PathBuf> = glob::glob(&pattern)?.filter_map(Result::ok).collect();

    if apks.is_empty() {
        // Also try without variant subdirectory
        let pattern = "app/build/outputs/apk/**/*.apk".to_string();
        apks = glob::glob(&pattern)?.filter_map(Result::ok).collect();
    }

    Ok(apks)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

struct UploadParams<'a> {
    client: &'a reqwest::Client,
    server: &'a str,
    token: &'a str,
    slug: &'a str,
    channel: &'a str,
    publish: bool,
    notes: Option<&'a str>,
}

/// One request for every APK, so the server files ABI splits of one build under a
/// single release instead of rejecting the second as a duplicate version code.
async fn upload_release(
    params: &UploadParams<'_>,
    apk_paths: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut form = reqwest::multipart::Form::new()
        .text("channel", params.channel.to_string())
        .text("publish", if params.publish { "true" } else { "false" });
    let mut total_bytes = 0u64;
    for apk_path in apk_paths {
        let apk_bytes = std::fs::read(apk_path)?;
        total_bytes += apk_bytes.len() as u64;
        let filename = apk_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        form = form.part(
            "apk",
            reqwest::multipart::Part::bytes(apk_bytes).file_name(filename),
        );
    }

    if let Some(notes) = params.notes {
        form = form.text("release_notes", notes.to_string());
    }

    if let Some(first) = apk_paths.first() {
        for (min_api, path) in dex_metadata::profiles_beside(first) {
            form = form.part(
                format!("dm_{min_api}"),
                reqwest::multipart::Part::bytes(std::fs::read(&path)?)
                    .file_name(format!("{min_api}.dm")),
            );
        }
    }

    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {bytes}/{total_bytes}")
            .expect("valid template")
            .progress_chars("=> "),
    );
    pb.set_message(format!("Uploading {} APK(s)", apk_paths.len()));

    let resp = params
        .client
        .post(format!(
            "{}/api/v1/ci/projects/{}/release",
            params.server, params.slug
        ))
        .header("authorization", http::auth_header(params.token))
        .multipart(form)
        .send()
        .await?;

    pb.finish_with_message(format!("Uploaded {} APK(s)", apk_paths.len()));

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Upload failed ({status}): {body}").into());
    }

    Ok(())
}

pub async fn run(args: PushArgs) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut apk_paths: Vec<PathBuf> = args.apk_paths.iter().map(PathBuf::from).collect();

    if args.build_gradle {
        let variant_cap = capitalize_first(&args.variant);
        println!("Running ./gradlew assemble{variant_cap}...");

        let status = tokio::process::Command::new("./gradlew")
            .arg(format!("assemble{variant_cap}"))
            .status()
            .await?;

        if !status.success() {
            return Err("Gradle build failed".into());
        }

        let found = find_gradle_apks(&args.variant)?;
        if found.is_empty() {
            return Err(format!(
                "No APK files found in app/build/outputs/apk/{}",
                args.variant
            )
            .into());
        }
        apk_paths.extend(found);
    }

    if apk_paths.is_empty() {
        return Err("Specify --apk or --build-gradle".into());
    }

    let channel = args.channel.as_deref().unwrap_or(&project_config.channel);
    let client = http::client()?;
    let slug = &project_config.project_slug;

    let params = UploadParams {
        client: &client,
        server,
        token,
        slug,
        channel,
        publish: args.publish,
        notes: args.notes.as_deref(),
    };

    if let Some(missing) = apk_paths.iter().find(|p| !p.exists()) {
        return Err(format!("APK file not found: {}", missing.display()).into());
    }
    upload_release(&params, &apk_paths).await?;

    if args.push_refresh {
        client
            .post(format!("{server}/api/v1/ci/projects/{slug}/push-refresh"))
            .header("authorization", http::auth_header(token))
            .send()
            .await?
            .error_for_status()?;
        println!("Push refresh sent for {slug}");
    }

    println!(
        "Done! Pushed {} APK(s) to {slug} (channel: {channel})",
        apk_paths.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{UploadParams, upload_release};
    use axum::extract::Multipart;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn splits_go_up_as_one_release() {
        let dir = std::env::temp_dir().join(format!("kobup-cli-push-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("baselineProfiles/0")).unwrap();
        std::fs::write(
            dir.join("output-metadata.json"),
            r#"{"baselineProfiles":[{"minApi":31,"baselineProfiles":["baselineProfiles/0/app.dm"]}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("baselineProfiles/0/app.dm"), b"dm").unwrap();
        let apks: Vec<_> = ["app-arm64-v8a-release.apk", "app-x86_64-release.apk"]
            .iter()
            .map(|name| {
                std::fs::write(dir.join(name), name.as_bytes()).unwrap();
                dir.join(name)
            })
            .collect();

        let requests: Arc<Mutex<Vec<Vec<String>>>> = Default::default();
        let sink = requests.clone();
        let app = axum::Router::new().route(
            "/api/v1/ci/projects/{slug}/release",
            axum::routing::post(move |mut form: Multipart| {
                let sink = sink.clone();
                async move {
                    let mut fields = Vec::new();
                    while let Some(field) = form.next_field().await.unwrap() {
                        let name = field.name().unwrap_or_default().to_string();
                        let file = field.file_name().unwrap_or_default().to_string();
                        fields.push(format!("{name}:{file}"));
                    }
                    sink.lock().unwrap().push(fields);
                    axum::http::StatusCode::CREATED
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });

        let client = reqwest::Client::new();
        let params = UploadParams {
            client: &client,
            server: &server,
            token: "t",
            slug: "app",
            channel: "stable",
            publish: true,
            notes: None,
        };
        upload_release(&params, &apks).await.expect("upload");

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{requests:?}");
        let apk_fields: Vec<_> = requests[0]
            .iter()
            .filter(|f| f.starts_with("apk:"))
            .collect();
        assert_eq!(
            apk_fields,
            [
                "apk:app-arm64-v8a-release.apk",
                "apk:app-x86_64-release.apk"
            ]
        );
        assert_eq!(
            requests[0]
                .iter()
                .filter(|f| f.starts_with("dm_31:"))
                .count(),
            1
        );
    }
}

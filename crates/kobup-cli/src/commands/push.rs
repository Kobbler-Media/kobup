use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

use crate::config::{ProjectConfig, UserConfig};
use crate::http;

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

async fn upload_apk(
    params: &UploadParams<'_>,
    apk_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let apk_bytes = std::fs::read(apk_path)?;
    let file_size = apk_bytes.len() as u64;
    let filename = apk_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {bytes}/{total_bytes}")
            .expect("valid template")
            .progress_chars("=> "),
    );
    pb.set_message(format!("Uploading {filename}"));

    let mut form = reqwest::multipart::Form::new()
        .part(
            "apk",
            reqwest::multipart::Part::bytes(apk_bytes).file_name(filename.clone()),
        )
        .text("channel", params.channel.to_string())
        .text("publish", if params.publish { "true" } else { "false" });

    if let Some(notes) = params.notes {
        form = form.text("release_notes", notes.to_string());
    }

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

    pb.finish_with_message(format!("Uploaded {filename}"));

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

    for apk_path in &apk_paths {
        if !apk_path.exists() {
            return Err(format!("APK file not found: {}", apk_path.display()).into());
        }
        upload_apk(&params, apk_path).await?;
    }

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

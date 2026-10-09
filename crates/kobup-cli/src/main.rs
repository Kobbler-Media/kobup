mod commands;
mod config;
mod dex_metadata;
mod http;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kobup",
    about = "Kobup CLI — publish and manage Android app updates"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a Kobup server via PIN flow
    Login,
    /// Initialize a project in the current directory
    Init(commands::init::InitArgs),
    /// Push APK(s) to Kobup
    Push(commands::push::PushArgs),
    /// Trigger push refresh notification
    PushRefresh,
    /// Show current project status
    Status,
    /// Check local setup and connectivity
    Doctor,
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Login => commands::login::run().await,
        Commands::Init(args) => commands::init::run(args).await,
        Commands::Push(args) => commands::push::run(args).await,
        Commands::PushRefresh => commands::push_refresh::run().await,
        Commands::Status => commands::status::run().await,
        Commands::Doctor => commands::doctor::run().await,
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

use kobup_service::{
    AppState, Config, auth,
    db::{connect, migrate},
    fcm::FcmClient,
    github,
    rate_limit::RateLimiter,
    retention,
    storage::LocalStorage,
};
use rand::distr::{Alphanumeric, SampleString};
use std::sync::Arc;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();

    // Ensure database parent directory exists before connecting
    if let Some(db_path) = config.database_url.strip_prefix("sqlite:")
        && let Some(db_path) = db_path.split('?').next()
        && let Some(parent) = std::path::Path::new(db_path).parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = tokio::fs::create_dir_all(parent).await
    {
        error!(
            "Failed to create database directory {}: {e}",
            parent.display()
        );
        return;
    }

    let pool = match connect(&config.database_url).await {
        Ok(pool) => pool,
        Err(err) => {
            error!("Failed to connect to database: {err}");
            return;
        }
    };

    if let Err(err) = migrate(&pool).await {
        error!("Database migration failed: {err}");
        return;
    }

    bootstrap_admin(&pool, &config).await;

    let storage = LocalStorage::new(&config.storage_root);
    if let Err(e) = storage.ensure_dirs().await {
        error!("Failed to create storage directories: {e}");
        return;
    }

    // Initialize FCM client if configured
    let fcm = config
        .fcm_service_account_json
        .as_deref()
        .filter(|p| !p.is_empty())
        .and_then(|path| match FcmClient::from_file(path) {
            Ok(client) => {
                info!("FCM client initialized from {path}");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!("Failed to initialize FCM client: {e}");
                None
            }
        });

    let config = Arc::new(config);
    let storage = Arc::new(storage);

    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
        storage: storage.clone(),
        fcm,
        login_limiter: Arc::new(RateLimiter::new(5, std::time::Duration::from_secs(60))),
    };

    // Start GitHub release polling background task
    github::start_polling(pool.clone(), storage.clone(), config.clone());

    // Start artifact retention background task
    retention::start_retention_loop(pool, storage, config.clone());

    let app = kobup_service::create_router(state);

    let listener = match tokio::net::TcpListener::bind(config.bind_addr.as_str()).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {e}", config.bind_addr);
            return;
        }
    };

    info!("kobup-service listening on {}", config.bind_addr);
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    {
        error!("Server error: {e}");
    }
}

async fn bootstrap_admin(pool: &kobup_service::DbPool, config: &Config) {
    let has_users: bool = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users)")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
        != 0;

    if has_users {
        return;
    }

    let password = config.admin_bootstrap_password.clone().unwrap_or_else(|| {
        let p = Alphanumeric.sample_string(&mut rand::rng(), 24);
        info!("Generated admin password: {p}");
        p
    });

    let password_hash = match auth::hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to hash bootstrap password: {e}");
            return;
        }
    };

    let id = uuid::Uuid::now_v7().to_string();
    if let Err(e) = sqlx::query!(
        "INSERT INTO users (id, email, password_hash, role) VALUES (?, ?, ?, 'admin')",
        id,
        config.admin_email,
        password_hash,
    )
    .execute(pool)
    .await
    {
        error!("Failed to bootstrap admin user: {e}");
        return;
    }

    info!("Bootstrapped admin user: {}", config.admin_email);

    // Bootstrap the "kobup" self-update project
    bootstrap_kobup_project(pool).await;
}

async fn bootstrap_kobup_project(pool: &kobup_service::DbPool) {
    use kobup_service::repositories;

    // Check if project already exists
    if let Ok(Some(_)) = repositories::projects::get_by_slug(pool, "kobup").await {
        return;
    }

    let input = repositories::projects::CreateProjectInput {
        slug: "kobup".to_string(),
        name: "Kobup Updater".to_string(),
        package_name: "com.kobup.app".to_string(),
        description:
            "The Kobup updater app itself. Upload new versions here to self-update all devices."
                .to_string(),
        max_releases_per_channel: None,
    };

    match repositories::projects::create(pool, &input).await {
        Ok(project) => {
            // Create default stable channel
            if let Err(e) = repositories::channels::create(pool, &project.id, "stable", true).await
            {
                error!("Failed to create kobup stable channel: {e}");
            }
            info!("Bootstrapped 'kobup' self-update project");
        }
        Err(e) => {
            error!("Failed to bootstrap kobup project: {e}");
        }
    }
}

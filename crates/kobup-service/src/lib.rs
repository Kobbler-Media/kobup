pub mod apk;
pub mod auth;
pub mod config;
pub mod db;
pub mod fcm;
pub mod github;
pub mod http;
pub mod rate_limit;
pub mod repositories;
pub mod retention;
pub mod routes;
pub mod storage;

pub use config::Config;
pub use db::DbPool;

use axum::http::Method;
use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use axum::routing::{delete, get, post, put};
use axum::{Router, extract::DefaultBodyLimit};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use routes::{admin, ci, health, public};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<Config>,
    pub storage: Arc<storage::LocalStorage>,
    pub fcm: Option<Arc<fcm::FcmClient>>,
    pub login_limiter: Arc<rate_limit::RateLimiter>,
}

pub fn create_router(state: AppState) -> Router {
    let cors = if state.config.web_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
    } else {
        let origins: Vec<_> = state
            .config
            .web_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
    };

    Router::new()
        .route("/health", get(health::health))
        // Admin auth
        .route("/api/v1/admin/login", post(admin::auth::login))
        .route("/api/v1/admin/me", get(admin::auth::me))
        .route("/api/v1/admin/logout", post(admin::auth::logout))
        .route(
            "/api/v1/admin/me/password",
            put(admin::auth::change_password),
        )
        .route("/api/v1/admin/me/email", put(admin::auth::change_email))
        // Admin projects
        .route(
            "/api/v1/admin/projects",
            get(admin::projects::list).post(admin::projects::create),
        )
        .route(
            "/api/v1/admin/projects/{id}",
            get(admin::projects::get)
                .patch(admin::projects::update)
                .delete(admin::projects::delete_project),
        )
        // Admin channels
        .route(
            "/api/v1/admin/projects/{project_id}/channels",
            post(admin::channels::create),
        )
        .route(
            "/api/v1/admin/projects/{project_id}/channels/{id}",
            delete(admin::channels::delete_channel),
        )
        // Admin upload
        .route(
            "/api/v1/admin/projects/{project_id}/upload",
            post(admin::upload::upload),
        )
        // Admin project icon
        .route(
            "/api/v1/admin/projects/{id}/icon",
            post(admin::projects::upload_icon),
        )
        // Admin releases
        .route(
            "/api/v1/admin/projects/{project_id}/releases",
            get(admin::releases::list),
        )
        .route(
            "/api/v1/admin/releases/{id}/publish",
            post(admin::releases::publish),
        )
        .route(
            "/api/v1/admin/releases/{id}",
            delete(admin::releases::delete_release),
        )
        // Admin devices
        .route("/api/v1/admin/devices", get(admin::devices::list))
        // Admin tokens
        .route(
            "/api/v1/admin/tokens",
            get(admin::tokens::list).post(admin::tokens::create),
        )
        .route("/api/v1/admin/tokens/{id}", delete(admin::tokens::revoke))
        // Admin users
        .route(
            "/api/v1/admin/users",
            get(admin::users::list).post(admin::users::create),
        )
        .route("/api/v1/admin/users/{id}", delete(admin::users::delete))
        .route(
            "/api/v1/admin/users/{id}/password",
            put(admin::users::reset_password),
        )
        // Admin PIN login
        .route(
            "/api/v1/admin/pin-login/device",
            post(admin::pin_login::pin_login_device),
        )
        .route(
            "/api/v1/admin/pin-login/cli",
            post(admin::pin_login::pin_login_cli),
        )
        // Admin push refresh
        .route(
            "/api/v1/admin/push-refresh",
            post(admin::push_refresh::push_refresh),
        )
        // Admin github ingest
        .route(
            "/api/v1/admin/projects/{project_id}/ingest/github",
            post(admin::github_ingest::configure),
        )
        // Public catalog
        .route("/api/v1/catalog", get(public::catalog::catalog))
        .route("/api/v1/projects/{id}/icon", get(public::icons::get_icon))
        .route(
            "/api/v1/projects/{slug}/latest",
            get(public::catalog::latest_by_slug),
        )
        .route(
            "/api/v1/packages/{package_name}/latest",
            get(public::packages::latest_by_package),
        )
        .route(
            "/api/v1/artifacts/{artifact_id}/download",
            get(public::download::download),
        )
        // Public devices
        .route("/api/v1/devices/register", post(public::devices::register))
        .route(
            "/api/v1/devices/{device_id}/push-token",
            post(public::devices::push_token),
        )
        .route(
            "/api/v1/devices/{device_id}/heartbeat",
            post(public::devices::heartbeat),
        )
        .route(
            "/api/v1/devices/{device_id}/install-event",
            post(public::devices::install_event),
        )
        // CI
        .route(
            "/api/v1/ci/projects/{slug}/release",
            post(ci::release::upload),
        )
        .route(
            "/api/v1/ci/projects/{slug}/push-refresh",
            post(ci::push_refresh::push_refresh),
        )
        .layer(DefaultBodyLimit::max(state.config.http_max_body_bytes))
        .layer(cors)
        .with_state(state)
}

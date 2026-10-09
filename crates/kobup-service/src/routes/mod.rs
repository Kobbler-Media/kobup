pub mod admin;
pub mod ci;
pub mod health;
pub mod public;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::repositories::{api_tokens, auth_tokens};

// ---------------------------------------------------------------------------
// Auth extractors
// ---------------------------------------------------------------------------

/// Admin user (web login, scope=admin).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

#[derive(Debug)]
pub struct AuthRejection;

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer(&parts.headers)
            .or_else(|| extract_session_cookie(&parts.headers))
            .unwrap_or_default();

        let user_id = auth_tokens::validate_token(&app_state.pool, &token, "admin")
            .await
            .map_err(|_| AuthRejection)?
            .ok_or(AuthRejection)?;

        Ok(AuthUser { user_id })
    }
}

/// Device auth (scope=device, long-lived read-only token).
#[derive(Debug, Clone)]
pub struct DeviceAuth {
    pub user_id: String,
}

impl<S> FromRequestParts<S> for DeviceAuth
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer(&parts.headers).unwrap_or_default();

        let user_id = auth_tokens::validate_token(&app_state.pool, &token, "device")
            .await
            .map_err(|_| AuthRejection)?
            .ok_or(AuthRejection)?;

        Ok(DeviceAuth { user_id })
    }
}

/// CI auth (API token with project scoping).
#[derive(Debug, Clone)]
pub struct CiAuth {
    pub project_id: Option<String>,
}

impl<S> FromRequestParts<S> for CiAuth
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer(&parts.headers).unwrap_or_default();

        if let Ok(Some(project_id)) = api_tokens::validate(&app_state.pool, &token).await {
            return Ok(CiAuth { project_id });
        }

        // A CLI PIN-login token lives in auth_tokens (scope="ci"), not
        // api_tokens, and isn't scoped to a single project.
        auth_tokens::validate_token(&app_state.pool, &token, "ci")
            .await
            .map_err(|_| AuthRejection)?
            .ok_or(AuthRejection)?;

        Ok(CiAuth { project_id: None })
    }
}

/// Any valid auth (admin, device, or CI).
#[derive(Debug, Clone)]
pub enum AnyAuth {
    Admin(AuthUser),
    Device(DeviceAuth),
    Ci(CiAuth),
}

impl<S> FromRequestParts<S> for AnyAuth
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_bearer(&parts.headers)
            .or_else(|| extract_session_cookie(&parts.headers))
            .unwrap_or_default();

        if token.is_empty() {
            return Err(AuthRejection);
        }

        // Try admin scope first
        if let Ok(Some(user_id)) =
            auth_tokens::validate_token(&app_state.pool, &token, "admin").await
        {
            return Ok(AnyAuth::Admin(AuthUser { user_id }));
        }

        // Try device scope
        if let Ok(Some(user_id)) =
            auth_tokens::validate_token(&app_state.pool, &token, "device").await
        {
            return Ok(AnyAuth::Device(DeviceAuth { user_id }));
        }

        // Try CI token
        if let Ok(Some(project_id)) = api_tokens::validate(&app_state.pool, &token).await {
            return Ok(AnyAuth::Ci(CiAuth { project_id }));
        }

        Err(AuthRejection)
    }
}

// ---------------------------------------------------------------------------
// Token extraction helpers
// ---------------------------------------------------------------------------

pub fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let auth = headers
        .get("authorization")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("").trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("kobup_session=") {
            let token = value.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

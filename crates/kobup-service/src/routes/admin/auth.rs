use axum::{
    Json as AxumJson,
    extract::{ConnectInfo, Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{AppState, auth, rate_limit::resolve_client_ip, repositories};

fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.eq_ignore_ascii_case("https"))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub role: String,
    pub email: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub user_id: String,
    pub email: String,
    pub role: String,
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let forwarded_for = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok());
    let ip = resolve_client_ip(forwarded_for, state.config.trusted_proxy_hops, peer.ip());

    if !state.login_limiter.check(ip) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    let (user_id, password_hash, role) =
        match repositories::users::get_by_email_for_login(&state.pool, &payload.email).await {
            Ok(Some(data)) => data,
            _ => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        };

    if !auth::verify_password(&password_hash, &payload.password) {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    let token = Alphanumeric.sample_string(&mut rand::rng(), state.config.token_length);
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(30))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    if let Err(e) =
        repositories::auth_tokens::create(&state.pool, &user_id, &token, "admin", Some(&expires_at))
            .await
    {
        tracing::error!(error = %e, "failed to insert auth token during login");
        return (StatusCode::INTERNAL_SERVER_ERROR, "login failed").into_response();
    }

    let secure_suffix = if is_https(&headers) { "; Secure" } else { "" };
    let session_cookie = format!(
        "kobup_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000{secure_suffix}",
    );
    let mut resp_headers = HeaderMap::new();
    let cookie_value = match HeaderValue::from_str(&session_cookie) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to create session cookie header");
            return (StatusCode::INTERNAL_SERVER_ERROR, "login failed").into_response();
        }
    };
    resp_headers.append("Set-Cookie", cookie_value);
    (
        resp_headers,
        AxumJson(LoginResponse {
            token,
            role,
            email: payload.email,
        }),
    )
        .into_response()
}

pub async fn me(State(state): State<AppState>, auth: crate::routes::AuthUser) -> impl IntoResponse {
    match repositories::users::get_by_id(&state.pool, &auth.user_id).await {
        Ok(Some((id, email, role))) => AxumJson(MeResponse {
            user_id: id,
            email,
            role,
        })
        .into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch user").into_response(),
    }
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let token = crate::routes::extract_bearer(&headers)
        .or_else(|| crate::routes::extract_session_cookie(&headers));

    if let Some(token) = token {
        let _ = repositories::auth_tokens::revoke_by_token(&state.pool, &token).await;
    }

    let secure_suffix = if is_https(&headers) { "; Secure" } else { "" };
    let clear_cookie =
        format!("kobup_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_suffix}");
    let mut resp_headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&clear_cookie) {
        resp_headers.insert("Set-Cookie", v);
    }
    (resp_headers, StatusCode::NO_CONTENT).into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEmailRequest {
    pub email: String,
}

pub async fn change_email(
    State(state): State<AppState>,
    auth: crate::routes::AuthUser,
    Json(payload): Json<ChangeEmailRequest>,
) -> impl IntoResponse {
    let email = payload.email.trim();
    if email.is_empty() || !email.contains('@') {
        return (StatusCode::BAD_REQUEST, "invalid email").into_response();
    }

    match repositories::users::update_email(&state.pool, &auth.user_id, email).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "user not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update email");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to update email").into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: crate::routes::AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    if payload.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters",
        )
            .into_response();
    }

    let password_hash = match repositories::users::get_password_hash(&state.pool, &auth.user_id)
        .await
    {
        Ok(Some(h)) => h,
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch user").into_response(),
    };

    if !auth::verify_password(&password_hash, &payload.current_password) {
        return (StatusCode::UNAUTHORIZED, "current password is incorrect").into_response();
    }

    let new_hash = match auth::hash_password(&payload.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "failed to hash password");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update password",
            )
                .into_response();
        }
    };
    match repositories::users::update_password(&state.pool, &auth.user_id, &new_hash).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update password",
        )
            .into_response(),
    }
}

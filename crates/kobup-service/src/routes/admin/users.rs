use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{AppState, auth, repositories};

pub async fn list(
    State(state): State<AppState>,
    _auth: crate::routes::AuthUser,
) -> impl IntoResponse {
    match repositories::users::list(&state.pool).await {
        Ok(users) => Json(users).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to list users");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list users").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub role: String,
}

pub async fn create(
    State(state): State<AppState>,
    _auth: crate::routes::AuthUser,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    if payload.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters",
        )
            .into_response();
    }

    if auth::Role::parse(&payload.role).is_none() {
        return (StatusCode::BAD_REQUEST, "role must be admin or viewer").into_response();
    }

    let password_hash = match auth::hash_password(&payload.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "failed to hash password");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to create user").into_response();
        }
    };

    let id = uuid::Uuid::now_v7().to_string();

    match repositories::users::create(
        &state.pool,
        &id,
        &payload.email,
        &password_hash,
        &payload.role,
    )
    .await
    {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create user");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to create user").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

/// Admin resets another user's password.
pub async fn reset_password(
    State(state): State<AppState>,
    _auth: crate::routes::AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    if payload.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters",
        )
            .into_response();
    }

    let password_hash = match auth::hash_password(&payload.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "failed to hash password");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to reset password",
            )
                .into_response();
        }
    };

    match repositories::users::update_password(&state.pool, &id, &password_hash).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "user not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to reset password");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to reset password",
            )
                .into_response()
        }
    }
}

pub async fn delete(
    State(state): State<AppState>,
    auth: crate::routes::AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if auth.user_id == id {
        return (StatusCode::BAD_REQUEST, "cannot delete yourself").into_response();
    }

    match repositories::users::delete(&state.pool, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "user not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to delete user");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to delete user").into_response()
        }
    }
}

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};

use crate::routes::AuthUser;
use crate::{AppState, repositories};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    id: String,
    project_id: Option<String>,
    name: String,
    created_at: String,
    revoked_at: Option<String>,
}

pub async fn list(State(state): State<AppState>, _auth: AuthUser) -> impl IntoResponse {
    match repositories::api_tokens::list(&state.pool).await {
        Ok(rows) => {
            let resp: Vec<TokenResponse> = rows
                .into_iter()
                .map(|r| TokenResponse {
                    id: r.id,
                    project_id: r.project_id,
                    name: r.name,
                    created_at: r.created_at,
                    revoked_at: r.revoked_at,
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to list tokens");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list tokens").into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenRequest {
    pub name: String,
    pub project_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateTokenResponse {
    id: String,
    token: String,
    name: String,
    project_id: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    if payload.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "name is required").into_response();
    }

    let raw_token = Alphanumeric.sample_string(&mut rand::rng(), state.config.token_length);

    match repositories::api_tokens::create(
        &state.pool,
        payload.project_id.as_deref(),
        payload.name.trim(),
        &raw_token,
    )
    .await
    {
        Ok(id) => (
            StatusCode::CREATED,
            Json(CreateTokenResponse {
                id,
                token: raw_token,
                name: payload.name,
                project_id: payload.project_id,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create token");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to create token").into_response()
        }
    }
}

pub async fn revoke(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match repositories::api_tokens::revoke(&state.pool, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "token not found or already revoked").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to revoke token");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to revoke token").into_response()
        }
    }
}

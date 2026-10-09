use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, repositories};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub device_name: String,
    pub sdk_int: Option<i64>,
    pub abis: Option<Vec<String>>,
    pub is_tv: Option<bool>,
    pub kobup_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterResponse {
    device_id: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let abis_json = serde_json::to_string(&payload.abis.unwrap_or_default())
        .unwrap_or_else(|_| "[]".to_string());

    let input = repositories::devices::CreateDeviceInput {
        device_name: payload.device_name,
        sdk_int: payload.sdk_int,
        abis_json,
        is_tv: payload.is_tv.unwrap_or(false),
        kobup_version: payload.kobup_version,
    };

    match repositories::devices::create(&state.pool, &input).await {
        Ok(device) => (
            StatusCode::CREATED,
            Json(RegisterResponse {
                device_id: device.id,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to register device");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to register device",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushTokenRequest {
    pub provider: Option<String>,
    pub token: String,
}

pub async fn push_token(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(payload): Json<PushTokenRequest>,
) -> impl IntoResponse {
    let provider = payload.provider.as_deref().unwrap_or("fcm");

    match repositories::device_push_tokens::upsert(
        &state.pool,
        &device_id,
        provider,
        &payload.token,
    )
    .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to upsert push token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to save push token",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub kobup_version: Option<String>,
    pub app_version_code: Option<i64>,
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(payload): Json<HeartbeatRequest>,
) -> impl IntoResponse {
    match repositories::devices::update_heartbeat(
        &state.pool,
        &device_id,
        payload.kobup_version.as_deref(),
        payload.app_version_code,
    )
    .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "device not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update heartbeat");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update heartbeat",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallEventRequest {
    pub package_name: String,
    pub from_version_code: Option<i64>,
    pub to_version_code: Option<i64>,
    pub status: String,
    pub message: Option<String>,
}

pub async fn install_event(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(payload): Json<InstallEventRequest>,
) -> impl IntoResponse {
    match repositories::install_events::create(
        &state.pool,
        &device_id,
        &payload.package_name,
        payload.from_version_code,
        payload.to_version_code,
        &payload.status,
        payload.message.as_deref(),
    )
    .await
    {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create install event");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to record install event",
            )
                .into_response()
        }
    }
}

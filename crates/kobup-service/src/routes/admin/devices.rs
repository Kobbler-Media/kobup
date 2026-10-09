use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::routes::AuthUser;
use crate::{AppState, repositories};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceResponse {
    id: String,
    device_name: String,
    platform: String,
    sdk_int: Option<i64>,
    abis_json: String,
    is_tv: bool,
    kobup_version: Option<String>,
    app_version_code: Option<i64>,
    last_seen_at: String,
    created_at: String,
}

pub async fn list(State(state): State<AppState>, _auth: AuthUser) -> impl IntoResponse {
    match repositories::devices::list(&state.pool).await {
        Ok(devices) => {
            let resp: Vec<DeviceResponse> = devices
                .into_iter()
                .map(|d| DeviceResponse {
                    id: d.id,
                    device_name: d.device_name,
                    platform: d.platform,
                    sdk_int: d.sdk_int,
                    abis_json: d.abis_json,
                    is_tv: d.is_tv,
                    kobup_version: d.kobup_version,
                    app_version_code: d.app_version_code,
                    last_seen_at: d.last_seen_at,
                    created_at: d.created_at,
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to list devices");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list devices").into_response()
        }
    }
}

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use xconnect_protocol::{DeviceInfo, DevicePlatform};

use crate::{auth::extract_access_subject, errors::ApiError, AppState};

#[derive(Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_name: String,
    pub platform: DevicePlatform,
    pub trusted: Option<bool>,
    pub unattended_enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct TrustRequest {
    pub trusted: bool,
}

#[derive(Serialize)]
pub struct DeviceListResponse {
    pub items: Vec<DeviceInfo>,
}

pub async fn list_devices(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<DeviceListResponse>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;

    let items = state
        .db
        .list_devices_by_account(account_id)
        .into_iter()
        .map(|row| DeviceInfo {
            device_id: row.device_id,
            account_id: row.account_id,
            device_name: row.device_name,
            platform: row.platform,
            trusted: row.trusted,
            unattended_enabled: row.unattended_enabled,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(DeviceListResponse { items }))
}

pub async fn register_device(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<DeviceInfo>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;

    if payload.device_name.trim().is_empty() {
        return Err(ApiError::BadRequest("device_name_required".to_string()));
    }

    let row = state.db.register_device(
        account_id,
        payload.device_name,
        payload.platform,
        payload.trusted.unwrap_or(false),
        payload.unattended_enabled.unwrap_or(false),
    );

    Ok(Json(DeviceInfo {
        device_id: row.device_id,
        account_id: row.account_id,
        device_name: row.device_name,
        platform: row.platform,
        trusted: row.trusted,
        unattended_enabled: row.unattended_enabled,
        created_at: row.created_at,
    }))
}

pub async fn set_trust(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<TrustRequest>,
) -> Result<Json<DeviceInfo>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;

    let row = state
        .db
        .set_device_trust(account_id, id, payload.trusted)
        .ok_or(ApiError::NotFound)?;

    Ok(Json(DeviceInfo {
        device_id: row.device_id,
        account_id: row.account_id,
        device_name: row.device_name,
        platform: row.platform,
        trusted: row.trusted,
        unattended_enabled: row.unattended_enabled,
        created_at: row.created_at,
    }))
}

pub async fn delete_device(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;

    if !state.db.delete_device(account_id, id) {
        return Err(ApiError::NotFound);
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

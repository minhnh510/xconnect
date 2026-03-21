use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use xconnect_protocol::{SessionDescriptor, SessionState};

use crate::{auth::extract_access_subject, errors::ApiError, AppState};

#[derive(Deserialize)]
pub struct RequestSessionBody {
    pub caller_device_id: Uuid,
    pub target_device_id: Uuid,
    pub unattended: bool,
}

pub async fn request_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RequestSessionBody>,
) -> Result<Json<SessionDescriptor>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;

    let caller = state
        .db
        .get_device(payload.caller_device_id)
        .ok_or_else(|| ApiError::BadRequest("caller_device_not_found".to_string()))?;
    let target = state
        .db
        .get_device(payload.target_device_id)
        .ok_or_else(|| ApiError::BadRequest("target_device_not_found".to_string()))?;

    if caller.account_id != account_id || target.account_id != account_id {
        return Err(ApiError::Forbidden);
    }

    if payload.unattended && (!caller.trusted || !target.unattended_enabled) {
        return Err(ApiError::Forbidden);
    }

    let session = state.db.create_session(
        account_id,
        payload.caller_device_id,
        payload.target_device_id,
        payload.unattended,
    );
    state.metrics.inc_session_created();

    Ok(Json(SessionDescriptor {
        session_id: session.session_id,
        account_id: session.account_id,
        caller_device_id: session.caller_device_id,
        target_device_id: session.target_device_id,
        unattended: session.unattended,
        state: session.state,
        created_at: session.created_at,
    }))
}

pub async fn cancel_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;
    state
        .db
        .set_session_state(account_id, id, SessionState::Rejected)
        .ok_or(ApiError::NotFound)?;

    Ok(Json(serde_json::json!({"status": "rejected"})))
}

pub async fn end_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;
    state
        .db
        .set_session_state(account_id, id, SessionState::Ended)
        .ok_or(ApiError::NotFound)?;

    Ok(Json(serde_json::json!({"status": "ended"})))
}

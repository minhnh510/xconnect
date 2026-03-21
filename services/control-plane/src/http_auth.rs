use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{hash_password, verify_password},
    errors::ApiError,
    AppState,
};

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub account_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if payload.email.trim().is_empty() || payload.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "email required and password must be >= 8 chars".to_string(),
        ));
    }

    let password_hash = hash_password(&payload.password)?;
    let user = state
        .db
        .create_user(payload.email.to_lowercase(), password_hash)
        .ok_or_else(|| ApiError::Conflict("email_already_exists".to_string()))?;

    let access_token = state
        .token_service
        .issue_access_token(user.user_id, state.config.access_token_ttl_seconds)?;
    let refresh_token = state
        .token_service
        .issue_refresh_token(user.user_id, state.config.refresh_token_ttl_seconds)?;

    Ok(Json(AuthResponse {
        account_id: user.user_id,
        access_token,
        refresh_token,
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user = state
        .db
        .find_user_by_email(&payload.email.to_lowercase())
        .ok_or(ApiError::Unauthorized)?;

    let ok = verify_password(&payload.password, &user.password_hash)?;
    if !ok {
        state.metrics.inc_login_fail();
        return Err(ApiError::Unauthorized);
    }
    state.metrics.inc_login_ok();

    let access_token = state
        .token_service
        .issue_access_token(user.user_id, state.config.access_token_ttl_seconds)?;
    let refresh_token = state
        .token_service
        .issue_refresh_token(user.user_id, state.config.refresh_token_ttl_seconds)?;

    Ok(Json(AuthResponse {
        account_id: user.user_id,
        access_token,
        refresh_token,
    }))
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = state.token_service.verify(&payload.refresh_token)?;
    if claims.scope != "refresh" {
        return Err(ApiError::Unauthorized);
    }

    let access_token = state
        .token_service
        .issue_access_token(claims.sub, state.config.access_token_ttl_seconds)?;
    let refresh_token = state
        .token_service
        .issue_refresh_token(claims.sub, state.config.refresh_token_ttl_seconds)?;

    Ok(Json(AuthResponse {
        account_id: claims.sub,
        access_token,
        refresh_token,
    }))
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = payload.refresh_token;
    let _ = state.token_service.verify(&token)?;
    state.token_service.revoke_refresh(token);

    Ok(Json(serde_json::json!({ "ok": true })))
}

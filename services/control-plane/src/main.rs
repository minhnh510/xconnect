#[path = "tracing.rs"]
mod app_tracing;
mod auth;
mod config;
mod db;
mod errors;
mod http_auth;
mod http_devices;
mod metrics;
mod session_service;
mod ws_signal;

use std::sync::Arc;

use axum::{routing::get, Router};
use config::Config;
use db::InMemoryDb;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::auth::TokenService;
use crate::metrics::Metrics;
use crate::ws_signal::WsHub;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: InMemoryDb,
    pub token_service: TokenService,
    pub ws_hub: WsHub,
    pub metrics: Metrics,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app_tracing::init();

    let config = Config::from_env()?;
    let _ = (&config.turn_secret, &config.turn_uris, &config.tls_pin_mode);
    let token_service = TokenService::new(config.jwt_secret.as_bytes())?;

    let state = Arc::new(AppState {
        config: config.clone(),
        db: InMemoryDb::default(),
        token_service,
        ws_hub: WsHub::default(),
        metrics: Metrics::default(),
    });

    let app = Router::new()
        .route("/v1/health", get(http_auth::health))
        .route(
            "/v1/auth/register",
            axum::routing::post(http_auth::register),
        )
        .route("/v1/auth/login", axum::routing::post(http_auth::login))
        .route("/v1/auth/refresh", axum::routing::post(http_auth::refresh))
        .route("/v1/auth/logout", axum::routing::post(http_auth::logout))
        .route(
            "/v1/devices",
            axum::routing::get(http_devices::list_devices),
        )
        .route(
            "/v1/devices/register",
            axum::routing::post(http_devices::register_device),
        )
        .route(
            "/v1/devices/:id/trust",
            axum::routing::patch(http_devices::set_trust),
        )
        .route(
            "/v1/devices/:id",
            axum::routing::delete(http_devices::delete_device),
        )
        .route(
            "/v1/sessions/request",
            axum::routing::post(session_service::request_session),
        )
        .route(
            "/v1/sessions/:id/cancel",
            axum::routing::post(session_service::cancel_session),
        )
        .route(
            "/v1/sessions/:id/end",
            axum::routing::post(session_service::end_session),
        )
        .route("/v1/signal", get(ws_signal::ws_upgrade))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = config.listen_addr;
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "control-plane listening");

    axum::serve(listener, app).await?;
    Ok(())
}

use std::net::SocketAddr;

use api_types::HealthResponse;
use axum::{Json, Router, routing::get};
use eyre::Result;
use tracing::info;

use crate::shutdown::ShutdownSignal;

/// Health check handler returning `{ "status": "ok" }`.
pub async fn handler() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok".to_owned() })
}

/// Create a router exposing the `/health` endpoint.
pub fn router() -> Router {
    Router::new().route("/health", get(handler))
}

/// Start a simple health check server.
///
/// The server exposes a `/health` endpoint that returns `{ "status": "ok" }`.
pub async fn serve(addr: SocketAddr, shutdown: ShutdownSignal) -> Result<()> {
    let app = router();

    info!("Starting health server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).with_graceful_shutdown(shutdown).await?;
    Ok(())
}

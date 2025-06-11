use std::net::SocketAddr;

use api_types::HealthResponse;
use axum::{Json, Router, routing::get};
use eyre::Result;
use tracing::info;

/// Start a simple health check server.
///
/// The server listens on the provided address and exposes a `/health` endpoint
/// returning `{ "status": "ok" }`.
pub async fn serve(addr: SocketAddr) -> Result<()> {
    let app = Router::new()
        .route("/health", get(|| async { Json(HealthResponse { status: "ok".to_owned() }) }));

    info!("Starting health server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

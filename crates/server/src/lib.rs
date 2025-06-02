//! Helper utilities to launch the Taikoscope API server.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use api::{self, ApiState};
use axum::{
    Router,
    http::{HeaderValue, Method},
};
use clickhouse_lib::ClickhouseReader;
use eyre::Result;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

/// Allowed CORS origins for dashboard requests.
const ALLOWED_ORIGINS: &[&str] = &["https://taikoscope.xyz", "https://www.taikoscope.xyz"];

/// Version prefix for all API routes.
pub const API_VERSION: &str = "v1";

/// Build the API router with CORS and tracing layers.
pub fn router(state: ApiState, extra_origins: Vec<String>) -> Router {
    let extra = Arc::new(extra_origins);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate({
            let extra = Arc::clone(&extra);
            move |origin: &HeaderValue, _| match origin.to_str() {
                Ok(origin) => {
                    ALLOWED_ORIGINS.contains(&origin) ||
                        origin.ends_with(".vercel.app") ||
                        extra.iter().any(|o| o == origin)
                }
                Err(_) => false,
            }
        }))
        .allow_methods([Method::GET])
        .allow_headers(Any);
    let trace = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Router::new().nest(&format!("/{API_VERSION}"), api::router(state)).layer(cors).layer(trace)
}

/// Run the API server on the given address.
pub async fn run(
    addr: SocketAddr,
    client: ClickhouseReader,
    extra_origins: Vec<String>,
    max_requests: u64,
    rate_period: Duration,
) -> Result<()> {
    let state = ApiState::new(client, max_requests, rate_period);
    let app = router(state, extra_origins);

    info!("Starting API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

//! Thin HTTP API for accessing `ClickHouse` data

use std::net::SocketAddr;

use axum::{Json, Router, extract::State, routing::get};
use chrono::{Duration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use tracing::info;

#[derive(Clone, Debug)]
struct ApiState {
    client: ClickhouseClient,
}

impl ApiState {
    const fn new(client: ClickhouseClient) -> Self {
        Self { client }
    }
}

#[derive(Serialize)]
struct L2HeadResponse {
    last_l2_head_time: Option<String>,
}

#[derive(Serialize)]
struct L1HeadResponse {
    last_l1_head_time: Option<String>,
}

#[derive(Serialize)]
struct SlashingEventsResponse {
    events: Vec<clickhouse::SlashingEventRow>,
}

async fn l2_head(State(state): State<ApiState>) -> Json<L2HeadResponse> {
    let ts = match state.client.get_last_l2_head_time().await {
        Ok(time) => time,
        Err(e) => {
            tracing::error!("Failed to get L2 head time: {}", e);
            None
        }
    };

    let resp = L2HeadResponse { last_l2_head_time: ts.map(|t| t.to_rfc3339()) };
    Json(resp)
}

async fn l1_head(State(state): State<ApiState>) -> Json<L1HeadResponse> {
    let ts = match state.client.get_last_l1_head_time().await {
        Ok(time) => time,
        Err(e) => {
            tracing::error!("Failed to get L1 head time: {}", e);
            None
        }
    };

    let resp = L1HeadResponse { last_l1_head_time: ts.map(|t| t.to_rfc3339()) };
    Json(resp)
}

async fn slashing_last_hour(State(state): State<ApiState>) -> Json<SlashingEventsResponse> {
    let since = Utc::now() - Duration::hours(1);
    let events = match state.client.get_slashing_events_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get slashing events: {}", e);
            Vec::new()
        }
    };
    Json(SlashingEventsResponse { events })
}

/// Run the API server on the given address
pub async fn run(addr: SocketAddr, client: ClickhouseClient) -> Result<()> {
    let state = ApiState::new(client);
    let app = Router::new()
        .route("/l2-head", get(l2_head))
        .route("/l1-head", get(l1_head))
        .route("/slashings/last-hour", get(slashing_last_hour))
        .with_state(state);

    info!("Starting API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

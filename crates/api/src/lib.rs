//! Thin HTTP API for accessing ClickHouse data

use std::net::SocketAddr;

use axum::{Json, Router, extract::State, routing::get};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use tracing::info;

#[derive(Clone, Debug)]
struct ApiState {
    client: ClickhouseClient,
}

impl ApiState {
    fn new(client: ClickhouseClient) -> Self {
        Self { client }
    }
}

#[derive(Serialize)]
struct L2HeadResponse {
    last_l2_head_time: Option<String>,
}

async fn l2_head(State(state): State<ApiState>) -> Result<Json<L2HeadResponse>> {
    let ts = state.client.get_last_l2_head_time().await?;
    let resp = L2HeadResponse { last_l2_head_time: ts.map(|t| t.to_rfc3339()) };
    Ok(Json(resp))
}

/// Run the API server on the given address
pub async fn run(addr: SocketAddr, client: ClickhouseClient) -> Result<()> {
    let state = ApiState::new(client);
    let app = Router::new().route("/l2-head", get(l2_head)).with_state(state);

    info!("Starting API server", %addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

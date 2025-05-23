//! Thin HTTP API for accessing `ClickHouse` data

use std::net::SocketAddr;

use axum::{Json, Router, extract::State, middleware, response::IntoResponse, routing::get};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse::ClickhouseClient;
use eyre::Result;
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration as StdDuration, Instant},
};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Maximum number of requests allowed during the [`RATE_PERIOD`].
const MAX_REQUESTS: u64 = 60;
/// Duration for the rate limiting window.
const RATE_PERIOD: StdDuration = StdDuration::from_secs(60);

#[derive(Clone, Debug)]
struct RateLimiter {
    state: Arc<Mutex<LimiterState>>,
    capacity: u64,
    period: StdDuration,
}

#[derive(Debug)]
struct LimiterState {
    count: u64,
    reset_at: Instant,
}

impl RateLimiter {
    fn new(capacity: u64, period: StdDuration) -> Self {
        Self {
            state: Arc::new(Mutex::new(LimiterState {
                count: 0,
                reset_at: Instant::now() + period,
            })),
            capacity,
            period,
        }
    }


fn try_acquire(&self) -> bool {
    let mut state = self.state.lock().expect("lock poisoned");
    let now = Instant::now();
    if now >= state.reset_at {
        state.reset_at = now + self.period;
        state.count = 1;  // Start at 1 for this request
        return true;
    }
    if state.count < self.capacity {
        state.count += 1;
        true
    } else {
        false
    }
}

}

#[derive(Clone, Debug)]
struct ApiState {
    client: ClickhouseClient,
    limiter: RateLimiter,
}

impl ApiState {
    fn new(client: ClickhouseClient) -> Self {
        Self { client, limiter: RateLimiter::new(MAX_REQUESTS, RATE_PERIOD) }
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

#[derive(Serialize)]
struct AvgProveTimeResponse {
    avg_prove_time_ms: Option<u64>,
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
    let since = Utc::now() - ChronoDuration::hours(1);
    let events = match state.client.get_slashing_events_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get slashing events: {}", e);
            Vec::new()
        }
    };
    Json(SlashingEventsResponse { events })
}

async fn avg_prove_time(State(state): State<ApiState>) -> Json<AvgProveTimeResponse> {
    let avg = match state.client.get_avg_prove_time_last_hour().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg prove time: {}", e);
            None
        }
    };
    Json(AvgProveTimeResponse { avg_prove_time_ms: avg })
}

async fn rate_limit(
    State(state): State<ApiState>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    if state.limiter.try_acquire() {
        next.run(req).await
    } else {
        axum::http::StatusCode::TOO_MANY_REQUESTS.into_response()
    }
}

/// Run the API server on the given address
pub async fn run(addr: SocketAddr, client: ClickhouseClient) -> Result<()> {
    let state = ApiState::new(client);
    let app = Router::new()
        .route("/l2-head", get(l2_head))
        .route("/l1-head", get(l1_head))
        .route("/slashings/last-hour", get(slashing_last_hour))
        .route("/avg-prove-time", get(avg_prove_time))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
        .layer(CorsLayer::permissive());

    info!("Starting API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

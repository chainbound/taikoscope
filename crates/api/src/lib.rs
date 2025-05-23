//! Thin HTTP API for accessing `ClickHouse` data

use std::net::SocketAddr;

use axum::{Json, Router, extract::State, middleware, response::IntoResponse, routing::get};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse_lib::ClickhouseClient;
use eyre::Result;
use hex::encode;
use primitives::rate_limiter::RateLimiter;
use serde::Serialize;
use std::time::Duration as StdDuration;
use tower_http::cors::CorsLayer;
use tracing::info;

/// Maximum number of requests allowed during the [`RATE_PERIOD`].
const MAX_REQUESTS: u64 = 60;
/// Duration for the rate limiting window.
const RATE_PERIOD: StdDuration = StdDuration::from_secs(60);

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
    events: Vec<clickhouse_lib::SlashingEventRow>,
}

#[derive(Serialize)]
struct ForcedInclusionEventsResponse {
    events: Vec<clickhouse_lib::ForcedInclusionProcessedRow>,
}

#[derive(Serialize)]
struct ReorgEventsResponse {
    events: Vec<clickhouse::L2ReorgRow>,
}

#[derive(Serialize)]
struct ActiveGatewaysResponse {
    gateways: Vec<String>,
}

#[derive(Serialize)]
struct AvgProveTimeResponse {
    avg_prove_time_ms: Option<u64>,
}

#[derive(Serialize)]
struct AvgVerifyTimeResponse {
    avg_verify_time_ms: Option<u64>,
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

async fn forced_inclusions_last_hour(
    State(state): State<ApiState>,
) -> Json<ForcedInclusionEventsResponse> {
    let since = Utc::now() - ChronoDuration::hours(1);
    let events = match state.client.get_forced_inclusions_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get forced inclusion events: {}", e);
            Vec::new()
        }
    };
    Json(ForcedInclusionEventsResponse { events })
}

async fn reorgs_last_hour(State(state): State<ApiState>) -> Json<ReorgEventsResponse> {
    let since = Utc::now() - ChronoDuration::hours(1);
    let events = match state.client.get_l2_reorgs_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get reorg events: {}", e);
            Vec::new()
        }
    };
    Json(ReorgEventsResponse { events })
}

async fn active_gateways_last_hour(State(state): State<ApiState>) -> Json<ActiveGatewaysResponse> {
    let since = Utc::now() - ChronoDuration::hours(1);
    let gateways = match state.client.get_active_gateways_since(since).await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to get active gateways: {}", e);
            Vec::new()
        }
    };
    let gateways = gateways.into_iter().map(|a| format!("0x{}", encode(a))).collect();
    Json(ActiveGatewaysResponse { gateways })
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

async fn avg_verify_time(State(state): State<ApiState>) -> Json<AvgVerifyTimeResponse> {
    let avg = match state.client.get_avg_verify_time_last_hour().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg verify time: {}", e);
            None
        }
    };
    Json(AvgVerifyTimeResponse { avg_verify_time_ms: avg })
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
        .route("/forced-inclusions/last-hour", get(forced_inclusions_last_hour))
        .route("/reorgs/last-hour", get(reorgs_last_hour))
        .route("/active-gateways/last-hour", get(active_gateways_last_hour))
        .route("/avg-prove-time", get(avg_prove_time))
        .route("/avg-verify-time", get(avg_verify_time))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
        .layer(CorsLayer::permissive());

    info!("Starting API server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{self, Body},
        http::Request,
    };
    use chrono::{TimeZone, Utc};
    use clickhouse::{
        Row,
        test::{Mock, handlers},
    };
    use serde::Serialize;
    use serde_json::{Value, json};
    use tower::util::ServiceExt;
    use url::Url;

    #[derive(Serialize, Row)]
    struct MaxRow {
        block_ts: u64,
    }

    #[derive(Serialize, Row)]
    struct AvgRowTest {
        avg_ms: Option<f64>,
    }

    fn build_app(mock_url: &str) -> Router {
        let url = Url::parse(mock_url).unwrap();
        let client =
            ClickhouseClient::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client);
        Router::new()
            .route("/l2-head", get(l2_head))
            .route("/l1-head", get(l1_head))
            .route("/slashings/last-hour", get(slashing_last_hour))
            .route("/forced-inclusions/last-hour", get(forced_inclusions_last_hour))
            .route("/avg-prove-time", get(avg_prove_time))
            .route("/avg-verify-time", get(avg_verify_time))
            .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
            .with_state(state)
    }

    async fn send_request(app: Router, uri: &str) -> Value {
        let response =
            app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
        assert!(response.status().is_success());
        let bytes = body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn l2_head_endpoint() {
        let mock = Mock::new();
        let ts = 42u64;
        mock.add(handlers::provide(vec![MaxRow { block_ts: ts }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-head").await;
        let expected = Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
        assert_eq!(body, json!({ "last_l2_head_time": expected }));
    }

    #[tokio::test]
    async fn l1_head_endpoint() {
        let mock = Mock::new();
        let ts = 24u64;
        mock.add(handlers::provide(vec![MaxRow { block_ts: ts }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l1-head").await;
        let expected = Utc.timestamp_opt(ts as i64, 0).single().unwrap().to_rfc3339();
        assert_eq!(body, json!({ "last_l1_head_time": expected }));
    }

    #[tokio::test]
    async fn slashing_events_endpoint() {
        let mock = Mock::new();
        let event =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        mock.add(handlers::provide(vec![event]));
        let expected =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        let app = build_app(mock.url());
        let body = send_request(app, "/slashings/last-hour").await;
        assert_eq!(body, json!({ "events": [expected] }));
    }

    #[tokio::test]
    async fn forced_inclusions_endpoint() {
        let mock = Mock::new();
        let event = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        mock.add(handlers::provide(vec![event]));
        let expected = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        let app = build_app(mock.url());
        let body = send_request(app, "/forced-inclusions/last-hour").await;
        assert_eq!(body, json!({ "events": [expected] }));
    }

    #[tokio::test]
    async fn avg_prove_time_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: Some(1500.0) }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-prove-time").await;
        assert_eq!(body, json!({ "avg_prove_time_ms": 1500 }));
    }

    #[tokio::test]
    async fn avg_verify_time_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: Some(2500.0) }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-verify-time").await;
        assert_eq!(body, json!({ "avg_verify_time_ms": 2500 }));
    }
}

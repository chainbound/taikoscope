//! Thin HTTP API for accessing `ClickHouse` data

use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method},
    middleware,
    response::IntoResponse,
    routing::get,
};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse_lib::ClickhouseClient;
use eyre::Result;
use hex::encode;
use primitives::rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Maximum number of requests allowed during the [`RATE_PERIOD`].
const MAX_REQUESTS: u64 = 60;
/// Duration for the rate limiting window.
const RATE_PERIOD: StdDuration = StdDuration::from_secs(60);

/// Allowed CORS origin for dashboard requests.
const ALLOWED_ORIGIN: &str = "https://taikoscope.xyz";

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

#[derive(Debug, Deserialize)]
struct RangeQuery {
    range: Option<String>,
}

fn range_duration(range: &Option<String>) -> ChronoDuration {
    match range.as_deref() {
        Some("24h") => ChronoDuration::hours(24),
        _ => ChronoDuration::hours(1),
    }
}

#[derive(Debug, Serialize)]
struct L2HeadResponse {
    last_l2_head_time: Option<String>,
}

#[derive(Debug, Serialize)]
struct L1HeadResponse {
    last_l1_head_time: Option<String>,
}

#[derive(Debug, Serialize)]
struct SlashingEventsResponse {
    events: Vec<clickhouse_lib::SlashingEventRow>,
}

#[derive(Debug, Serialize)]
struct ForcedInclusionEventsResponse {
    events: Vec<clickhouse_lib::ForcedInclusionProcessedRow>,
}

#[derive(Debug, Serialize)]
struct ReorgEventsResponse {
    events: Vec<clickhouse_lib::L2ReorgRow>,
}

#[derive(Debug, Serialize)]
struct ActiveGatewaysResponse {
    gateways: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AvgProveTimeResponse {
    avg_prove_time_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct AvgVerifyTimeResponse {
    avg_verify_time_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct L2BlockCadenceResponse {
    l2_block_cadence_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct BatchPostingCadenceResponse {
    batch_posting_cadence_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct AvgL2TpsResponse {
    avg_tps: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ProveTimesResponse {
    batches: Vec<clickhouse_lib::BatchProveTimeRow>,
}

#[derive(Debug, Serialize)]
struct VerifyTimesResponse {
    batches: Vec<clickhouse_lib::BatchVerifyTimeRow>,
}

#[derive(Debug, Serialize)]
struct L1BlockTimesResponse {
    blocks: Vec<clickhouse_lib::L1BlockTimeRow>,
}

#[derive(Debug, Serialize)]
struct L2BlockTimesResponse {
    blocks: Vec<clickhouse_lib::L2BlockTimeRow>,
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

async fn slashings(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<SlashingEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_slashing_events_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get slashing events: {}", e);
            Vec::new()
        }
    };
    Json(SlashingEventsResponse { events })
}

async fn forced_inclusions(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ForcedInclusionEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_forced_inclusions_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get forced inclusion events: {}", e);
            Vec::new()
        }
    };
    Json(ForcedInclusionEventsResponse { events })
}

async fn reorgs(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ReorgEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_l2_reorgs_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!("Failed to get reorg events: {}", e);
            Vec::new()
        }
    };
    Json(ReorgEventsResponse { events })
}

async fn active_gateways(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ActiveGatewaysResponse> {
    let since = Utc::now() - range_duration(&params.range);
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

async fn avg_prove_time_24h(State(state): State<ApiState>) -> Json<AvgProveTimeResponse> {
    let avg = match state.client.get_avg_prove_time_last_24_hours().await {
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

async fn avg_verify_time_24h(State(state): State<ApiState>) -> Json<AvgVerifyTimeResponse> {
    let avg = match state.client.get_avg_verify_time_last_24_hours().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg verify time: {}", e);
            None
        }
    };
    Json(AvgVerifyTimeResponse { avg_verify_time_ms: avg })
}

async fn l2_block_cadence(State(state): State<ApiState>) -> Json<L2BlockCadenceResponse> {
    let avg = match state.client.get_l2_block_cadence_last_hour().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get L2 block cadence: {}", e);
            None
        }
    };
    Json(L2BlockCadenceResponse { l2_block_cadence_ms: avg })
}

async fn l2_block_cadence_24h(State(state): State<ApiState>) -> Json<L2BlockCadenceResponse> {
    let avg = match state.client.get_l2_block_cadence_last_24_hours().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get L2 block cadence: {}", e);
            None
        }
    };
    Json(L2BlockCadenceResponse { l2_block_cadence_ms: avg })
}

async fn batch_posting_cadence(State(state): State<ApiState>) -> Json<BatchPostingCadenceResponse> {
    let avg = match state.client.get_batch_posting_cadence_last_hour().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get batch posting cadence: {}", e);
            None
        }
    };
    Json(BatchPostingCadenceResponse { batch_posting_cadence_ms: avg })
}

async fn batch_posting_cadence_24h(
    State(state): State<ApiState>,
) -> Json<BatchPostingCadenceResponse> {
    let avg = match state.client.get_batch_posting_cadence_last_24_hours().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get batch posting cadence: {}", e);
            None
        }
    };
    Json(BatchPostingCadenceResponse { batch_posting_cadence_ms: avg })
}

async fn avg_l2_tps(State(state): State<ApiState>) -> Json<AvgL2TpsResponse> {
    let avg = match state.client.get_avg_l2_tps_last_hour().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg L2 TPS: {}", e);
            None
        }
    };
    Json(AvgL2TpsResponse { avg_tps: avg })
}

async fn avg_l2_tps_24h(State(state): State<ApiState>) -> Json<AvgL2TpsResponse> {
    let avg = match state.client.get_avg_l2_tps_last_24_hours().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg L2 TPS: {}", e);
            None
        }
    };
    Json(AvgL2TpsResponse { avg_tps: avg })
}

async fn prove_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ProveTimesResponse> {
    let batches = match match params.range.as_deref() {
        Some("24h") => state.client.get_prove_times_last_24_hours().await,
        _ => state.client.get_prove_times_last_hour().await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get prove times: {}", e);
            Vec::new()
        }
    };
    Json(ProveTimesResponse { batches })
}

async fn verify_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<VerifyTimesResponse> {
    let batches = match match params.range.as_deref() {
        Some("24h") => state.client.get_verify_times_last_24_hours().await,
        _ => state.client.get_verify_times_last_hour().await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get verify times: {}", e);
            Vec::new()
        }
    };
    Json(VerifyTimesResponse { batches })
}

async fn l1_block_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L1BlockTimesResponse> {
    let blocks = match match params.range.as_deref() {
        Some("24h") => state.client.get_l1_block_times_last_24_hours().await,
        _ => state.client.get_l1_block_times_last_hour().await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L1 block times: {}", e);
            Vec::new()
        }
    };
    Json(L1BlockTimesResponse { blocks })
}

async fn l2_block_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L2BlockTimesResponse> {
    let blocks = match match params.range.as_deref() {
        Some("24h") => state.client.get_l2_block_times_last_24_hours().await,
        _ => state.client.get_l2_block_times_last_hour().await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 block times: {}", e);
            Vec::new()
        }
    };
    Json(L2BlockTimesResponse { blocks })
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

fn router(state: ApiState) -> Router {
    Router::new()
        .route("/l2-head", get(l2_head))
        .route("/l1-head", get(l1_head))
        .route("/slashings", get(slashings))
        .route("/forced-inclusions", get(forced_inclusions))
        .route("/reorgs", get(reorgs))
        .route("/active-gateways", get(active_gateways))
        .route("/avg-prove-time", get(avg_prove_time))
        .route("/avg-prove-time/24h", get(avg_prove_time_24h))
        .route("/avg-verify-time", get(avg_verify_time))
        .route("/avg-verify-time/24h", get(avg_verify_time_24h))
        .route("/l2-block-cadence", get(l2_block_cadence))
        .route("/l2-block-cadence/24h", get(l2_block_cadence_24h))
        .route("/batch-posting-cadence", get(batch_posting_cadence))
        .route("/batch-posting-cadence/24h", get(batch_posting_cadence_24h))
        .route("/avg-l2-tps", get(avg_l2_tps))
        .route("/avg-l2-tps/24h", get(avg_l2_tps_24h))
        .route("/prove-times", get(prove_times))
        .route("/verify-times", get(verify_times))
        .route("/l1-block-times", get(l1_block_times))
        .route("/l2-block-times", get(l2_block_times))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

/// Run the API server on the given address
pub async fn run(addr: SocketAddr, client: ClickhouseClient) -> Result<()> {
    let state = ApiState::new(client);
    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static(ALLOWED_ORIGIN))
        .allow_methods([Method::GET])
        .allow_headers(Any);
    let app = router(state).layer(cors);

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
        router(state)
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
        let body = send_request(app, "/slashings?range=1h").await;
        assert_eq!(body, json!({ "events": [expected] }));
    }

    #[tokio::test]
    async fn slashing_events_last_day_endpoint() {
        let mock = Mock::new();
        let event =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        mock.add(handlers::provide(vec![event]));
        let expected =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        let app = build_app(mock.url());
        let body = send_request(app, "/slashings?range=24h").await;
        assert_eq!(body, json!({ "events": [expected] }));
    }

    #[tokio::test]
    async fn forced_inclusions_endpoint() {
        let mock = Mock::new();
        let event = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        mock.add(handlers::provide(vec![event]));
        let expected = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        let app = build_app(mock.url());
        let body = send_request(app, "/forced-inclusions?range=1h").await;
        assert_eq!(body, json!({ "events": [expected] }));
    }

    #[tokio::test]
    async fn forced_inclusions_last_day_endpoint() {
        let mock = Mock::new();
        let event = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        mock.add(handlers::provide(vec![event]));
        let expected = clickhouse_lib::ForcedInclusionProcessedRow { blob_hash: [2u8; 32] };
        let app = build_app(mock.url());
        let body = send_request(app, "/forced-inclusions?range=24h").await;
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
    async fn avg_prove_time_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: Some(1500.0) }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-prove-time/24h").await;
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

    #[tokio::test]
    async fn avg_verify_time_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: Some(2500.0) }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-verify-time/24h").await;
        assert_eq!(body, json!({ "avg_verify_time_ms": 2500 }));
    }

    #[derive(Serialize, Row)]
    struct CadenceRowTest {
        min_ts: Option<u64>,
        max_ts: Option<u64>,
        cnt: u64,
    }

    #[tokio::test]
    async fn l2_block_cadence_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest {
            min_ts: Some(1000),
            max_ts: Some(4000),
            cnt: 4,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-cadence").await;
        assert_eq!(body, json!({ "l2_block_cadence_ms": 1000 }));
    }

    #[tokio::test]
    async fn l2_block_cadence_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest {
            min_ts: Some(1000),
            max_ts: Some(4000),
            cnt: 4,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-cadence/24h").await;
        assert_eq!(body, json!({ "l2_block_cadence_ms": 1000 }));
    }

    #[tokio::test]
    async fn batch_posting_cadence_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest {
            min_ts: Some(2000),
            max_ts: Some(6000),
            cnt: 3,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-cadence").await;
        assert_eq!(body, json!({ "batch_posting_cadence_ms": 2000 }));
    }

    #[tokio::test]
    async fn batch_posting_cadence_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest {
            min_ts: Some(2000),
            max_ts: Some(6000),
            cnt: 3,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-cadence/24h").await;
        assert_eq!(body, json!({ "batch_posting_cadence_ms": 2000 }));
    }

    #[derive(Serialize, Row)]
    struct ProveRowTest {
        batch_id: u64,
        seconds_to_prove: u64,
    }

    #[tokio::test]
    async fn prove_times_last_hour_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![ProveRowTest { batch_id: 1, seconds_to_prove: 10 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/prove-times?range=1h").await;
        assert_eq!(body, json!({ "batches": [ { "batch_id": 1, "seconds_to_prove": 10 } ] }));
    }

    #[tokio::test]
    async fn prove_times_last_day_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![ProveRowTest { batch_id: 1, seconds_to_prove: 10 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/prove-times?range=24h").await;
        assert_eq!(body, json!({ "batches": [ { "batch_id": 1, "seconds_to_prove": 10 } ] }));
    }

    #[derive(Serialize, Row)]
    struct VerifyRowTest {
        batch_id: u64,
        seconds_to_verify: u64,
    }

    #[tokio::test]
    async fn verify_times_last_hour_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![VerifyRowTest { batch_id: 2, seconds_to_verify: 120 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/verify-times?range=1h").await;
        assert_eq!(body, json!({ "batches": [ { "batch_id": 2, "seconds_to_verify": 120 } ] }));
    }

    #[tokio::test]
    async fn verify_times_last_day_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![VerifyRowTest { batch_id: 2, seconds_to_verify: 120 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/verify-times?range=24h").await;
        assert_eq!(body, json!({ "batches": [ { "batch_id": 2, "seconds_to_verify": 120 } ] }));
    }

    #[derive(Serialize, Row)]
    struct BlockTimeRowTest {
        minute: u64,
        block_number: u64,
    }

    #[tokio::test]
    async fn l1_block_times_last_hour_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BlockTimeRowTest { minute: 1, block_number: 2 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l1-block-times?range=1h").await;
        assert_eq!(body, json!({ "blocks": [ { "minute": 1, "block_number": 2 } ] }));
    }

    #[tokio::test]
    async fn l2_block_times_last_hour_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BlockTimeRowTest { minute: 0, block_number: 1 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=1h").await;
        assert_eq!(body, json!({ "blocks": [ { "minute": 0, "block_number": 1 } ] }));
    }

    #[tokio::test]
    async fn l2_block_times_last_day_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BlockTimeRowTest { minute: 0, block_number: 1 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=24h").await;
        assert_eq!(body, json!({ "blocks": [ { "minute": 0, "block_number": 1 } ] }));
    }

    #[derive(Serialize, Row)]
    struct TpsRowTest {
        min_ts: Option<u64>,
        max_ts: Option<u64>,
        tx_sum: Option<u64>,
    }

    #[tokio::test]
    async fn avg_l2_tps_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest {
            min_ts: Some(10),
            max_ts: Some(70),
            tx_sum: Some(180),
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-l2-tps").await;
        assert_eq!(body, json!({ "avg_tps": 3.0 }));
    }

    #[tokio::test]
    async fn avg_l2_tps_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest {
            min_ts: Some(100),
            max_ts: Some(460),
            tx_sum: Some(720),
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-l2-tps/24h").await;
        assert_eq!(body, json!({ "avg_tps": 2.0 }));
    }
}

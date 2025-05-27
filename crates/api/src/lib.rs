//! Thin HTTP API for accessing `ClickHouse` data

use std::net::SocketAddr;

use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method},
    middleware,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse_lib::ClickhouseReader;
use eyre::Result;
use futures::stream::Stream;
use hex::encode;
use primitives::rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration as StdDuration};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

/// Maximum number of requests allowed during the [`RATE_PERIOD`].
const MAX_REQUESTS: u64 = 1000;
/// Duration for the rate limiting window.
const RATE_PERIOD: StdDuration = StdDuration::from_secs(60);

/// Allowed CORS origins for dashboard requests.
const ALLOWED_ORIGINS: &[&str] = &["https://taikoscope.xyz", "https://www.taikoscope.xyz"];

#[derive(Clone, Debug)]
struct ApiState {
    client: ClickhouseReader,
    limiter: RateLimiter,
}

impl ApiState {
    fn new(client: ClickhouseReader) -> Self {
        Self { client, limiter: RateLimiter::new(MAX_REQUESTS, RATE_PERIOD) }
    }
}

#[derive(Debug, Deserialize)]
struct RangeQuery {
    range: Option<String>,
}

fn range_duration(range: &Option<String>) -> ChronoDuration {
    const MAX_RANGE_HOURS: i64 = 24 * 7; // maximum range of 7 days

    if let Some(r) = range.as_deref() {
        let r = r.trim().to_ascii_lowercase();

        if let Some(h) = r.strip_suffix('h') {
            if let Ok(hours) = h.parse::<i64>() {
                let hours = hours.clamp(0, MAX_RANGE_HOURS);
                return ChronoDuration::hours(hours);
            }
        }

        if let Some(d) = r.strip_suffix('d') {
            if let Ok(days) = d.parse::<i64>() {
                let hours = (days * 24).clamp(0, MAX_RANGE_HOURS);
                return ChronoDuration::hours(hours);
            }
        }
    }

    ChronoDuration::hours(1)
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

#[derive(Debug, Serialize)]
struct SequencerDistributionItem {
    address: String,
    blocks: u64,
}

#[derive(Debug, Serialize)]
struct SequencerDistributionResponse {
    sequencers: Vec<SequencerDistributionItem>,
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

async fn sse_l2_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l2_block_number().await.ok().flatten().unwrap_or(0);
    let stream = stream! {
        loop {
            match state.client.get_last_l2_block_number().await {
                Ok(Some(num)) if num != last => {
                    last = num;
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(_) => {}
                Err(e) => tracing::error!("Failed to fetch L2 head block: {}", e),
            }
            tokio::time::sleep(StdDuration::from_secs(1)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn sse_l1_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l1_block_number().await.ok().flatten().unwrap_or(0);
    let stream = stream! {
        loop {
            match state.client.get_last_l1_block_number().await {
                Ok(Some(num)) if num != last => {
                    last = num;
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(_) => {}
                Err(e) => tracing::error!("Failed to fetch L1 head block: {}", e),
            }
            tokio::time::sleep(StdDuration::from_secs(1)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
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

async fn avg_prove_time(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<AvgProveTimeResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_avg_prove_time_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_avg_prove_time_last_24_hours().await
    } else {
        state.client.get_avg_prove_time_last_7_days().await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg prove time: {}", e);
            None
        }
    };
    Json(AvgProveTimeResponse { avg_prove_time_ms: avg })
}

async fn avg_verify_time(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<AvgVerifyTimeResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_avg_verify_time_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_avg_verify_time_last_24_hours().await
    } else {
        state.client.get_avg_verify_time_last_7_days().await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get avg verify time: {}", e);
            None
        }
    };
    Json(AvgVerifyTimeResponse { avg_verify_time_ms: avg })
}

async fn l2_block_cadence(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L2BlockCadenceResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_l2_block_cadence_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_l2_block_cadence_last_24_hours().await
    } else {
        state.client.get_l2_block_cadence_last_7_days().await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get L2 block cadence: {}", e);
            None
        }
    };
    Json(L2BlockCadenceResponse { l2_block_cadence_ms: avg })
}

async fn batch_posting_cadence(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<BatchPostingCadenceResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_batch_posting_cadence_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_batch_posting_cadence_last_24_hours().await
    } else {
        state.client.get_batch_posting_cadence_last_7_days().await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to get batch posting cadence: {}", e);
            None
        }
    };
    Json(BatchPostingCadenceResponse { batch_posting_cadence_ms: avg })
}

async fn avg_l2_tps(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<AvgL2TpsResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_avg_l2_tps_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_avg_l2_tps_last_24_hours().await
    } else {
        state.client.get_avg_l2_tps_last_7_days().await
    } {
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
        Some("7d") => state.client.get_prove_times_last_7_days().await,
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
        Some("7d") => state.client.get_verify_times_last_7_days().await,
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
        Some("7d") => state.client.get_l1_block_times_last_7_days().await,
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
        Some("7d") => state.client.get_l2_block_times_last_7_days().await,
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

async fn sequencer_distribution(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<SequencerDistributionResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let rows = match state.client.get_sequencer_distribution_since(since).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get sequencer distribution: {}", e);
            Vec::new()
        }
    };
    let sequencers = rows
        .into_iter()
        .map(|r| SequencerDistributionItem {
            address: format!("0x{}", encode(r.sequencer)),
            blocks: r.blocks,
        })
        .collect();
    Json(SequencerDistributionResponse { sequencers })
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
        .route("/sse/l1-head", get(sse_l1_head))
        .route("/sse/l2-head", get(sse_l2_head))
        .route("/slashings", get(slashings))
        .route("/forced-inclusions", get(forced_inclusions))
        .route("/reorgs", get(reorgs))
        .route("/active-gateways", get(active_gateways))
        .route("/avg-prove-time", get(avg_prove_time))
        .route("/avg-verify-time", get(avg_verify_time))
        .route("/l2-block-cadence", get(l2_block_cadence))
        .route("/batch-posting-cadence", get(batch_posting_cadence))
        .route("/avg-l2-tps", get(avg_l2_tps))
        .route("/prove-times", get(prove_times))
        .route("/verify-times", get(verify_times))
        .route("/l1-block-times", get(l1_block_times))
        .route("/l2-block-times", get(l2_block_times))
        .route("/sequencer-distribution", get(sequencer_distribution))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

/// Run the API server on the given address
pub async fn run(addr: SocketAddr, client: ClickhouseReader) -> Result<()> {
    let state = ApiState::new(client);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| match origin.to_str() {
            Ok(origin) => ALLOWED_ORIGINS.contains(&origin) || origin.ends_with(".vercel.app"),
            Err(_) => false,
        }))
        .allow_methods([Method::GET])
        .allow_headers(Any);
    let trace = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));
    let app = router(state).layer(cors).layer(trace);

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
        avg_ms: f64,
    }

    fn build_app(mock_url: &str) -> Router {
        let url = Url::parse(mock_url).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
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
    async fn slashing_events_last_week_endpoint() {
        let mock = Mock::new();
        let event =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        mock.add(handlers::provide(vec![event]));
        let expected =
            clickhouse_lib::SlashingEventRow { l1_block_number: 1, validator_addr: [1u8; 20] };
        let app = build_app(mock.url());
        let body = send_request(app, "/slashings?range=7d").await;
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
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 1500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-prove-time").await;
        assert_eq!(body, json!({ "avg_prove_time_ms": 1500 }));
    }

    #[tokio::test]
    async fn avg_prove_time_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 1500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-prove-time?range=24h").await;
        assert_eq!(body, json!({ "avg_prove_time_ms": 1500 }));
    }

    #[tokio::test]
    async fn avg_prove_time_7d_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 1500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-prove-time?range=7d").await;
        assert_eq!(body, json!({ "avg_prove_time_ms": 1500 }));
    }

    #[tokio::test]
    async fn avg_verify_time_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 2500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-verify-time").await;
        assert_eq!(body, json!({ "avg_verify_time_ms": 2500 }));
    }

    #[tokio::test]
    async fn avg_verify_time_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 2500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-verify-time?range=24h").await;
        assert_eq!(body, json!({ "avg_verify_time_ms": 2500 }));
    }

    #[tokio::test]
    async fn avg_verify_time_7d_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![AvgRowTest { avg_ms: 2500.0 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-verify-time?range=7d").await;
        assert_eq!(body, json!({ "avg_verify_time_ms": 2500 }));
    }

    #[derive(Serialize, Row)]
    struct CadenceRowTest {
        min_ts: u64,
        max_ts: u64,
        cnt: u64,
    }

    #[tokio::test]
    async fn l2_block_cadence_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 1000, max_ts: 4000, cnt: 4 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-cadence").await;
        assert_eq!(body, json!({ "l2_block_cadence_ms": 1000 }));
    }

    #[tokio::test]
    async fn l2_block_cadence_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 1000, max_ts: 4000, cnt: 4 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-cadence?range=24h").await;
        assert_eq!(body, json!({ "l2_block_cadence_ms": 1000 }));
    }

    #[tokio::test]
    async fn l2_block_cadence_7d_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 1000, max_ts: 4000, cnt: 4 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-cadence?range=7d").await;
        assert_eq!(body, json!({ "l2_block_cadence_ms": 1000 }));
    }

    #[tokio::test]
    async fn batch_posting_cadence_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 2000, max_ts: 6000, cnt: 3 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-cadence").await;
        assert_eq!(body, json!({ "batch_posting_cadence_ms": 2000 }));
    }

    #[tokio::test]
    async fn batch_posting_cadence_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 2000, max_ts: 6000, cnt: 3 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-cadence?range=24h").await;
        assert_eq!(body, json!({ "batch_posting_cadence_ms": 2000 }));
    }

    #[tokio::test]
    async fn batch_posting_cadence_7d_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![CadenceRowTest { min_ts: 2000, max_ts: 6000, cnt: 3 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-cadence?range=7d").await;
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

    #[tokio::test]
    async fn prove_times_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![ProveRowTest { batch_id: 1, seconds_to_prove: 10 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/prove-times?range=7d").await;
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

    #[tokio::test]
    async fn verify_times_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![VerifyRowTest { batch_id: 2, seconds_to_verify: 120 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/verify-times?range=7d").await;
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
    async fn l1_block_times_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BlockTimeRowTest { minute: 1, block_number: 2 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l1-block-times?range=7d").await;
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

    #[tokio::test]
    async fn l2_block_times_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![BlockTimeRowTest { minute: 0, block_number: 1 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=7d").await;
        assert_eq!(body, json!({ "blocks": [ { "minute": 0, "block_number": 1 } ] }));
    }

    #[derive(Serialize, Row)]
    struct TpsRowTest {
        min_ts: u64,
        max_ts: u64,
        tx_sum: u64,
    }

    #[tokio::test]
    async fn avg_l2_tps_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest { min_ts: 10, max_ts: 70, tx_sum: 180 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-l2-tps").await;
        assert_eq!(body, json!({ "avg_tps": 3.0 }));
    }

    #[tokio::test]
    async fn avg_l2_tps_24h_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest { min_ts: 100, max_ts: 460, tx_sum: 720 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-l2-tps?range=24h").await;
        assert_eq!(body, json!({ "avg_tps": 2.0 }));
    }

    #[tokio::test]
    async fn avg_l2_tps_7d_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![TpsRowTest { min_ts: 100, max_ts: 460, tx_sum: 720 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-l2-tps?range=7d").await;
        assert_eq!(body, json!({ "avg_tps": 2.0 }));
    }

    #[derive(Serialize, Row)]
    struct SequencerRowTest {
        sequencer: [u8; 20],
        blocks: u64,
    }

    #[tokio::test]
    async fn sequencer_distribution_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![SequencerRowTest { sequencer: [1u8; 20], blocks: 5 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/sequencer-distribution?range=1h").await;
        assert_eq!(
            body,
            json!({ "sequencers": [ { "address": "0x0101010101010101010101010101010101010101", "blocks": 5 } ] })
        );
    }

    #[test]
    fn range_duration_clamps_negative_hours() {
        let d = range_duration(&Some("-5h".to_owned()));
        assert_eq!(d.num_hours(), 0);
    }

    #[test]
    fn range_duration_clamps_negative_days() {
        let d = range_duration(&Some("-2d".to_owned()));
        assert_eq!(d.num_hours(), 0);
    }

    #[test]
    fn range_duration_accepts_uppercase() {
        let d = range_duration(&Some("5H".to_owned()));
        assert_eq!(d.num_hours(), 5);

        let d = range_duration(&Some("2D".to_owned()));
        assert_eq!(d.num_hours(), 48);
    }
}

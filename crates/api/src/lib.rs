//! Thin HTTP API for accessing `ClickHouse` data

use api_types::*;
use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Query, State},
    middleware,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use chrono::{Duration as ChronoDuration, Utc};
use clickhouse_lib::ClickhouseReader;
use futures::stream::Stream;
use hex::encode;
use runtime::rate_limiter::RateLimiter;
use serde::Deserialize;
use std::{convert::Infallible, time::Duration as StdDuration};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// Default maximum number of requests allowed during the rate limiting period.
pub const DEFAULT_MAX_REQUESTS: u64 = 1000;
/// Default duration for the rate limiting window.
pub const DEFAULT_RATE_PERIOD: StdDuration = StdDuration::from_secs(60);

/// `OpenAPI` documentation structure
#[derive(Debug, OpenApi)]
#[openapi(
    paths(
        l2_head,
        l1_head,
        l2_head_block,
        l1_head_block,
        slashings,
        forced_inclusions,
        reorgs,
        active_gateways,
        current_operator,
        next_operator,
        avg_prove_time,
        avg_verify_time,
        l2_block_cadence,
        batch_posting_cadence,
        avg_l2_tps,
        avg_blobs_per_batch,
        blobs_per_batch,
        prove_times,
        verify_times,
        l1_block_times,
        l2_block_times,
        l2_gas_used,
        sequencer_distribution,
        sequencer_blocks,
        block_transactions
    ),
    components(
        schemas(
            RangeQuery,
            SequencerBlocksQuery,
            BlockTransactionsQuery,
            L2HeadResponse,
            L1HeadResponse,
            L2HeadBlockResponse,
            L1HeadBlockResponse,
            SlashingEventsResponse,
            ForcedInclusionEventsResponse,
            ReorgEventsResponse,
            ActiveGatewaysResponse,
            CurrentOperatorResponse,
            NextOperatorResponse,
            AvgProveTimeResponse,
            AvgVerifyTimeResponse,
            L2BlockCadenceResponse,
            BatchPostingCadenceResponse,
            AvgL2TpsResponse,
            AvgBlobsPerBatchResponse,
            BatchBlobsResponse,
            ProveTimesResponse,
            VerifyTimesResponse,
            L1BlockTimesResponse,
            L2BlockTimesResponse,
            L2GasUsedResponse,
            SequencerDistributionResponse,
            SequencerDistributionItem,
            SequencerBlocksResponse,
            SequencerBlocksItem,
            BlockTransactionsResponse,
            BlockTransactionsItem,
            clickhouse_lib::SlashingEventRow,
            clickhouse_lib::ForcedInclusionProcessedRow,
            clickhouse_lib::L2ReorgRow,
            clickhouse_lib::BatchProveTimeRow,
            clickhouse_lib::BatchVerifyTimeRow,
            clickhouse_lib::L1BlockTimeRow,
            clickhouse_lib::L2BlockTimeRow,
            clickhouse_lib::L2GasUsedRow,
            clickhouse_lib::BatchBlobCountRow
        )
    ),
    tags(
        (name = "taikoscope", description = "Taikoscope API endpoints")
    ),
    info(
        title = "Taikoscope API",
        description = "API for accessing Taiko blockchain metrics and data",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;

/// Shared state for API handlers.
#[derive(Clone, Debug)]
pub struct ApiState {
    client: ClickhouseReader,
    limiter: RateLimiter,
}

impl ApiState {
    /// Create a new [`ApiState`].
    pub fn new(client: ClickhouseReader, max_requests: u64, rate_period: StdDuration) -> Self {
        Self { client, limiter: RateLimiter::new(max_requests, rate_period) }
    }
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
struct RangeAddressQuery {
    range: Option<String>,
    address: Option<String>,
}

type RangeQuery = RangeAddressQuery;
type SequencerBlocksQuery = RangeAddressQuery;

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
struct BlockTransactionsQuery {
    range: Option<String>,
    limit: Option<u64>,
    starting_after: Option<u64>,
    ending_before: Option<u64>,
    address: Option<String>,
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

fn parse_address(addr: &Option<String>) -> Option<[u8; 20]> {
    let a = addr.as_deref()?;
    let trimmed = a.trim_start_matches("0x");
    let v = hex::decode(trimmed).ok()?;
    <[u8; 20]>::try_from(v).ok()
}

#[utoipa::path(
    get,
    path = "/l2-head",
    responses(
        (status = 200, description = "L2 head timestamp", body = L2HeadResponse)
    ),
    tag = "taikoscope"
)]
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

#[utoipa::path(
    get,
    path = "/l1-head",
    responses(
        (status = 200, description = "L1 head timestamp", body = L1HeadResponse)
    ),
    tag = "taikoscope"
)]
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

#[utoipa::path(
    get,
    path = "/l2-head-block",
    responses(
        (status = 200, description = "L2 head block number", body = L2HeadBlockResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_head_block(State(state): State<ApiState>) -> Json<L2HeadBlockResponse> {
    let num = match state.client.get_last_l2_block_number().await {
        Ok(num) => num,
        Err(e) => {
            tracing::error!("Failed to get L2 head block number: {}", e);
            None
        }
    };
    Json(L2HeadBlockResponse { l2_head_block: num })
}

#[utoipa::path(
    get,
    path = "/l1-head-block",
    responses(
        (status = 200, description = "L1 head block number", body = L1HeadBlockResponse)
    ),
    tag = "taikoscope"
)]
async fn l1_head_block(State(state): State<ApiState>) -> Json<L1HeadBlockResponse> {
    let num = match state.client.get_last_l1_block_number().await {
        Ok(num) => num,
        Err(e) => {
            tracing::error!("Failed to get L1 head block number: {}", e);
            None
        }
    };
    Json(L1HeadBlockResponse { l1_head_block: num })
}

async fn sse_l2_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l2_block_number().await.ok().flatten().unwrap_or(0);
    let stream = stream! {
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));
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
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));
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

#[utoipa::path(
    get,
    path = "/slashings",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Slashing events", body = SlashingEventsResponse)
    ),
    tag = "taikoscope"
)]
async fn slashings(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<SlashingEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_slashing_events_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get slashing events");
            Vec::new()
        }
    };
    tracing::info!(count = events.len(), "Returning slashing events");
    Json(SlashingEventsResponse { events })
}

#[utoipa::path(
    get,
    path = "/forced-inclusions",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Forced inclusion events", body = ForcedInclusionEventsResponse)
    ),
    tag = "taikoscope"
)]
async fn forced_inclusions(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ForcedInclusionEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_forced_inclusions_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get forced inclusion events");
            Vec::new()
        }
    };
    tracing::info!(count = events.len(), "Returning forced inclusion events");
    Json(ForcedInclusionEventsResponse { events })
}

#[utoipa::path(
    get,
    path = "/reorgs",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Reorg events", body = ReorgEventsResponse)
    ),
    tag = "taikoscope"
)]
async fn reorgs(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ReorgEventsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let events = match state.client.get_l2_reorgs_since(since).await {
        Ok(evts) => evts,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get reorg events");
            Vec::new()
        }
    };
    tracing::info!(count = events.len(), "Returning reorg events");
    Json(ReorgEventsResponse { events })
}

#[utoipa::path(
    get,
    path = "/active-gateways",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Active gateways", body = ActiveGatewaysResponse)
    ),
    tag = "taikoscope"
)]
async fn active_gateways(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<ActiveGatewaysResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let gateways = match state.client.get_active_gateways_since(since).await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get active gateways");
            Vec::new()
        }
    };
    let gateways: Vec<String> = gateways.into_iter().map(|a| format!("0x{}", encode(a))).collect();
    tracing::info!(count = gateways.len(), "Returning active gateways");
    Json(ActiveGatewaysResponse { gateways })
}

#[utoipa::path(
    get,
    path = "/current-operator",
    responses(
        (status = 200, description = "Current operator", body = CurrentOperatorResponse)
    ),
    tag = "taikoscope"
)]
async fn current_operator(State(state): State<ApiState>) -> Json<CurrentOperatorResponse> {
    let op = match state.client.get_last_current_operator().await {
        Ok(o) => o.map(|a| format!("0x{}", encode(a))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get current operator");
            None
        }
    };
    tracing::info!(has_value = op.is_some(), "Returning current operator");
    Json(CurrentOperatorResponse { operator: op })
}

#[utoipa::path(
    get,
    path = "/next-operator",
    responses(
        (status = 200, description = "Next operator", body = NextOperatorResponse)
    ),
    tag = "taikoscope"
)]
async fn next_operator(State(state): State<ApiState>) -> Json<NextOperatorResponse> {
    let op = match state.client.get_last_next_operator().await {
        Ok(o) => o.map(|a| format!("0x{}", encode(a))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get next operator");
            None
        }
    };
    tracing::info!(has_value = op.is_some(), "Returning next operator");
    Json(NextOperatorResponse { operator: op })
}

#[utoipa::path(
    get,
    path = "/avg-prove-time",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Average prove time", body = AvgProveTimeResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get avg prove time");
            None
        }
    };
    tracing::info!(avg_prove_time_ms = ?avg, "Returning avg prove time");
    Json(AvgProveTimeResponse { avg_prove_time_ms: avg })
}

#[utoipa::path(
    get,
    path = "/avg-verify-time",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Average verify time", body = AvgVerifyTimeResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get avg verify time");
            None
        }
    };
    tracing::info!(avg_verify_time_ms = ?avg, "Returning avg verify time");
    Json(AvgVerifyTimeResponse { avg_verify_time_ms: avg })
}

#[utoipa::path(
    get,
    path = "/l2-block-cadence",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 block cadence", body = L2BlockCadenceResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_block_cadence(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L2BlockCadenceResponse> {
    let duration = range_duration(&params.range);
    let address = parse_address(&params.address);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_l2_block_cadence_last_hour(address).await
    } else if duration.num_hours() <= 24 {
        state.client.get_l2_block_cadence_last_24_hours(address).await
    } else {
        state.client.get_l2_block_cadence_last_7_days(address).await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block cadence");
            None
        }
    };
    tracing::info!(l2_block_cadence_ms = ?avg, "Returning L2 block cadence");
    Json(L2BlockCadenceResponse { l2_block_cadence_ms: avg })
}

#[utoipa::path(
    get,
    path = "/batch-posting-cadence",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Batch posting cadence", body = BatchPostingCadenceResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get batch posting cadence");
            None
        }
    };
    tracing::info!(batch_posting_cadence_ms = ?avg, "Returning batch posting cadence");
    Json(BatchPostingCadenceResponse { batch_posting_cadence_ms: avg })
}

#[utoipa::path(
    get,
    path = "/avg-l2-tps",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Average L2 TPS", body = AvgL2TpsResponse)
    ),
    tag = "taikoscope"
)]
async fn avg_l2_tps(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<AvgL2TpsResponse> {
    let duration = range_duration(&params.range);
    let address = parse_address(&params.address);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_avg_l2_tps_last_hour(address).await
    } else if duration.num_hours() <= 24 {
        state.client.get_avg_l2_tps_last_24_hours(address).await
    } else {
        state.client.get_avg_l2_tps_last_7_days(address).await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get avg L2 TPS");
            None
        }
    };
    tracing::info!(avg_tps = ?avg, "Returning avg L2 TPS");
    Json(AvgL2TpsResponse { avg_tps: avg })
}

#[utoipa::path(
    get,
    path = "/avg-blobs-per-batch",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Average blobs per batch", body = AvgBlobsPerBatchResponse)
    ),
    tag = "taikoscope"
)]
async fn avg_blobs_per_batch(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<AvgBlobsPerBatchResponse> {
    let duration = range_duration(&params.range);
    let avg = match if duration.num_hours() <= 1 {
        state.client.get_avg_blobs_per_batch_last_hour().await
    } else if duration.num_hours() <= 24 {
        state.client.get_avg_blobs_per_batch_last_24_hours().await
    } else {
        state.client.get_avg_blobs_per_batch_last_7_days().await
    } {
        Ok(val) => val,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get avg blobs per batch");
            None
        }
    };
    tracing::info!(avg_blobs_per_batch = ?avg, "Returning avg blobs per batch");
    Json(AvgBlobsPerBatchResponse { avg_blobs: avg })
}

#[utoipa::path(
    get,
    path = "/blobs-per-batch",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Blobs per batch", body = BatchBlobsResponse)
    ),
    tag = "taikoscope"
)]
async fn blobs_per_batch(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<BatchBlobsResponse> {
    let batches = match match params.range.as_deref() {
        Some("24h") => state.client.get_blobs_per_batch_last_24_hours().await,
        Some("7d") => state.client.get_blobs_per_batch_last_7_days().await,
        _ => state.client.get_blobs_per_batch_last_hour().await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get blobs per batch");
            Vec::new()
        }
    };
    tracing::info!(count = batches.len(), "Returning blobs per batch");
    Json(BatchBlobsResponse { batches })
}

#[utoipa::path(
    get,
    path = "/prove-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Prove times", body = ProveTimesResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get prove times");
            Vec::new()
        }
    };
    tracing::info!(count = batches.len(), "Returning prove times");
    Json(ProveTimesResponse { batches })
}

#[utoipa::path(
    get,
    path = "/verify-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Verify times", body = VerifyTimesResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get verify times");
            Vec::new()
        }
    };
    tracing::info!(count = batches.len(), "Returning verify times");
    Json(VerifyTimesResponse { batches })
}

#[utoipa::path(
    get,
    path = "/l1-block-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L1 block times", body = L1BlockTimesResponse)
    ),
    tag = "taikoscope"
)]
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
            tracing::error!(error = %e, "Failed to get L1 block times");
            Vec::new()
        }
    };
    tracing::info!(count = blocks.len(), "Returning L1 block times");
    Json(L1BlockTimesResponse { blocks })
}

#[utoipa::path(
    get,
    path = "/l2-block-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 block times", body = L2BlockTimesResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_block_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L2BlockTimesResponse> {
    let address = parse_address(&params.address);
    let blocks = match match params.range.as_deref() {
        Some("24h") => state.client.get_l2_block_times_last_24_hours(address).await,
        Some("7d") => state.client.get_l2_block_times_last_7_days(address).await,
        _ => state.client.get_l2_block_times_last_hour(address).await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block times");
            Vec::new()
        }
    };
    tracing::info!(count = blocks.len(), "Returning L2 block times");
    Json(L2BlockTimesResponse { blocks })
}

#[utoipa::path(
    get,
    path = "/l2-gas-used",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 gas used", body = L2GasUsedResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_gas_used(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<L2GasUsedResponse> {
    let address = parse_address(&params.address);
    let blocks = match match params.range.as_deref() {
        Some("24h") => state.client.get_l2_gas_used_last_24_hours(address).await,
        Some("7d") => state.client.get_l2_gas_used_last_7_days(address).await,
        _ => state.client.get_l2_gas_used_last_hour(address).await,
    } {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 gas used: {}", e);
            Vec::new()
        }
    };
    Json(L2GasUsedResponse { blocks })
}

#[utoipa::path(
    get,
    path = "/sequencer-distribution",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Sequencer distribution", body = SequencerDistributionResponse)
    ),
    tag = "taikoscope"
)]
async fn sequencer_distribution(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Json<SequencerDistributionResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let rows = match state.client.get_sequencer_distribution_since(since).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get sequencer distribution");
            Vec::new()
        }
    };
    let sequencers: Vec<SequencerDistributionItem> = rows
        .into_iter()
        .map(|r| SequencerDistributionItem {
            address: format!("0x{}", encode(r.sequencer)),
            blocks: r.blocks,
        })
        .collect();
    tracing::info!(count = sequencers.len(), "Returning sequencer distribution");
    Json(SequencerDistributionResponse { sequencers })
}

#[utoipa::path(
    get,
    path = "/sequencer-blocks",
    params(
        SequencerBlocksQuery
    ),
    responses(
        (status = 200, description = "Sequencer blocks", body = SequencerBlocksResponse)
    ),
    tag = "taikoscope"
)]
async fn sequencer_blocks(
    Query(params): Query<SequencerBlocksQuery>,
    State(state): State<ApiState>,
) -> Json<SequencerBlocksResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let rows = match state.client.get_sequencer_blocks_since(since).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get sequencer blocks");
            Vec::new()
        }
    };

    let filter = if let Some(addr) = params.address.as_deref() {
        let trimmed = addr.trim_start_matches("0x");
        if let Ok(v) = hex::decode(trimmed) { <[u8; 20]>::try_from(v).ok() } else { None }
    } else {
        None
    };

    use std::collections::BTreeMap;
    let mut map: BTreeMap<[u8; 20], Vec<u64>> = BTreeMap::new();
    for r in rows {
        if let Some(addr) = filter {
            if r.sequencer != addr {
                continue;
            }
        }
        map.entry(r.sequencer).or_default().push(r.l2_block_number);
    }

    let sequencers: Vec<SequencerBlocksItem> = map
        .into_iter()
        .map(|(seq, blocks)| SequencerBlocksItem { address: format!("0x{}", encode(seq)), blocks })
        .collect();
    tracing::info!(count = sequencers.len(), "Returning sequencer blocks");
    Json(SequencerBlocksResponse { sequencers })
}

#[utoipa::path(
    get,
    path = "/block-transactions",
    params(
        BlockTransactionsQuery
    ),
    responses(
        (status = 200, description = "Block transactions", body = BlockTransactionsResponse)
    ),
    tag = "taikoscope"
)]
async fn block_transactions(
    Query(params): Query<BlockTransactionsQuery>,
    State(state): State<ApiState>,
) -> Json<BlockTransactionsResponse> {
    let since = Utc::now() - range_duration(&params.range);
    let limit = params.limit.unwrap_or(50);
    if params.starting_after.is_some() && params.ending_before.is_some() {
        tracing::warn!("starting_after and ending_before are mutually exclusive");
    }
    let rows = match state
        .client
        .get_block_transactions_paginated(
            since,
            limit,
            params.starting_after,
            params.ending_before,
            parse_address(&params.address),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get block transactions");
            Vec::new()
        }
    };

    let blocks: Vec<BlockTransactionsItem> = rows
        .into_iter()
        .map(|r| BlockTransactionsItem {
            block: r.l2_block_number,
            txs: r.sum_tx,
            sequencer: format!("0x{}", encode(r.sequencer)),
        })
        .collect();

    tracing::info!(count = blocks.len(), "Returning block transactions");
    Json(BlockTransactionsResponse { blocks })
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

/// Build the router with all API endpoints.
pub fn router(state: ApiState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/l2-head", get(l2_head))
        .route("/l1-head", get(l1_head))
        .route("/l2-head-block", get(l2_head_block))
        .route("/l1-head-block", get(l1_head_block))
        .route("/sse/l1-head", get(sse_l1_head))
        .route("/sse/l2-head", get(sse_l2_head))
        .route("/slashings", get(slashings))
        .route("/forced-inclusions", get(forced_inclusions))
        .route("/reorgs", get(reorgs))
        .route("/active-gateways", get(active_gateways))
        .route("/current-operator", get(current_operator))
        .route("/next-operator", get(next_operator))
        .route("/avg-prove-time", get(avg_prove_time))
        .route("/avg-verify-time", get(avg_verify_time))
        .route("/l2-block-cadence", get(l2_block_cadence))
        .route("/batch-posting-cadence", get(batch_posting_cadence))
        .route("/avg-l2-tps", get(avg_l2_tps))
        .route("/avg-blobs-per-batch", get(avg_blobs_per_batch))
        .route("/blobs-per-batch", get(blobs_per_batch))
        .route("/prove-times", get(prove_times))
        .route("/verify-times", get(verify_times))
        .route("/l1-block-times", get(l1_block_times))
        .route("/l2-block-times", get(l2_block_times))
        .route("/l2-gas-used", get(l2_gas_used))
        .route("/sequencer-distribution", get(sequencer_distribution))
        .route("/sequencer-blocks", get(sequencer_blocks))
        .route("/block-transactions", get(block_transactions))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
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

    #[derive(Serialize, Row)]
    struct MaxNum {
        number: u64,
    }

    fn build_app(mock_url: &str) -> Router {
        let url = Url::parse(mock_url).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, DEFAULT_MAX_REQUESTS, DEFAULT_RATE_PERIOD);
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
    async fn l2_head_block_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxNum { number: 1 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-head-block").await;
        assert_eq!(body, json!({ "l2_head_block": 1 }));
    }

    #[tokio::test]
    async fn l1_head_block_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxNum { number: 2 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l1-head-block").await;
        assert_eq!(body, json!({ "l1_head_block": 2 }));
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

    #[derive(Serialize, Row)]
    struct L2BlockTimeRowTest {
        l2_block_number: u64,
        block_time: u64,
        ms_since_prev_block: Option<u64>,
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
        mock.add(handlers::provide(vec![
            L2BlockTimeRowTest { l2_block_number: 0, block_time: 0, ms_since_prev_block: None },
            L2BlockTimeRowTest {
                l2_block_number: 1,
                block_time: 2,
                ms_since_prev_block: Some(2000),
            },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=1h").await;
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 1, "block_time": "1970-01-01T00:00:02Z", "ms_since_prev_block": 2000 } ] })
        );
    }

    #[tokio::test]
    async fn l2_block_times_last_day_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            L2BlockTimeRowTest { l2_block_number: 0, block_time: 0, ms_since_prev_block: None },
            L2BlockTimeRowTest {
                l2_block_number: 1,
                block_time: 2,
                ms_since_prev_block: Some(2000),
            },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=24h").await;
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 1, "block_time": "1970-01-01T00:00:02Z", "ms_since_prev_block": 2000 } ] })
        );
    }

    #[tokio::test]
    async fn l2_block_times_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            L2BlockTimeRowTest { l2_block_number: 0, block_time: 0, ms_since_prev_block: None },
            L2BlockTimeRowTest {
                l2_block_number: 1,
                block_time: 2,
                ms_since_prev_block: Some(2000),
            },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-block-times?range=7d").await;
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 1, "block_time": "1970-01-01T00:00:02Z", "ms_since_prev_block": 2000 } ] })
        );
    }

    #[derive(Serialize, Row)]
    struct L2GasUsedRowTest {
        l2_block_number: u64,
        gas_used: u64,
    }

    #[tokio::test]
    async fn l2_gas_used_last_hour_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            L2GasUsedRowTest { l2_block_number: 0, gas_used: 0 },
            L2GasUsedRowTest { l2_block_number: 1, gas_used: 42 },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-gas-used?range=1h").await;
        assert_eq!(body, json!({ "blocks": [ { "l2_block_number": 1, "gas_used": 42 } ] }));
    }

    #[tokio::test]
    async fn l2_gas_used_last_day_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            L2GasUsedRowTest { l2_block_number: 0, gas_used: 0 },
            L2GasUsedRowTest { l2_block_number: 1, gas_used: 42 },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-gas-used?range=24h").await;
        assert_eq!(body, json!({ "blocks": [ { "l2_block_number": 1, "gas_used": 42 } ] }));
    }

    #[tokio::test]
    async fn l2_gas_used_last_week_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            L2GasUsedRowTest { l2_block_number: 0, gas_used: 0 },
            L2GasUsedRowTest { l2_block_number: 1, gas_used: 42 },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-gas-used?range=7d").await;
        assert_eq!(body, json!({ "blocks": [ { "l2_block_number": 1, "gas_used": 42 } ] }));
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

    #[derive(Serialize, Row)]
    struct CurrentRowTest {
        current_operator: Option<[u8; 20]>,
    }

    #[derive(Serialize, Row)]
    struct NextRowTest {
        next_operator: Option<[u8; 20]>,
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

    #[tokio::test]
    async fn sequencer_blocks_endpoint() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct SeqBlockRowTest {
            sequencer: [u8; 20],
            l2_block_number: u64,
        }
        mock.add(handlers::provide(vec![SeqBlockRowTest {
            sequencer: [1u8; 20],
            l2_block_number: 42,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/sequencer-blocks?range=1h").await;
        assert_eq!(
            body,
            json!({ "sequencers": [ { "address": "0x0101010101010101010101010101010101010101", "blocks": [42] } ] })
        );
    }

    #[tokio::test]
    async fn block_transactions_endpoint() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct TxRowTest {
            sequencer: [u8; 20],
            l2_block_number: u64,
            sum_tx: u32,
        }
        mock.add(handlers::provide(vec![TxRowTest {
            sequencer: [1u8; 20],
            l2_block_number: 42,
            sum_tx: 7,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/block-transactions?range=1h").await;
        assert_eq!(
            body,
            json!({ "blocks": [ { "block": 42, "txs": 7, "sequencer": "0x0101010101010101010101010101010101010101" } ] })
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

    #[tokio::test]
    async fn current_operator_endpoint() {
        let mock = Mock::new();
        let addr = [1u8; 20];
        mock.add(handlers::provide(vec![CurrentRowTest { current_operator: Some(addr) }]));

        let app = build_app(mock.url());
        let body = send_request(app, "/current-operator").await;
        assert_eq!(body, json!({ "operator": format!("0x{}", hex::encode(addr)) }));
    }

    #[tokio::test]
    async fn next_operator_endpoint() {
        let mock = Mock::new();
        let addr = [2u8; 20];
        mock.add(handlers::provide(vec![NextRowTest { next_operator: Some(addr) }]));

        let app = build_app(mock.url());
        let body = send_request(app, "/next-operator").await;
        assert_eq!(body, json!({ "operator": format!("0x{}", hex::encode(addr)) }));
    }

    #[tokio::test]
    async fn avg_blobs_per_batch_endpoint() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct AvgRowTest {
            avg: f64,
        }
        mock.add(handlers::provide(vec![AvgRowTest { avg: 2.5 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/avg-blobs-per-batch").await;
        assert_eq!(body, json!({ "avg_blobs": 2.5 }));
    }

    #[tokio::test]
    async fn blobs_per_batch_endpoint() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct BlobRowTest {
            batch_id: u64,
            blob_count: u8,
        }
        mock.add(handlers::provide(vec![BlobRowTest { batch_id: 1, blob_count: 3 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/blobs-per-batch?range=1h").await;
        assert_eq!(body, json!({ "batches": [ { "batch_id": 1, "blob_count": 3 } ] }));
    }
}

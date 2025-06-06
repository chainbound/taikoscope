//! Thin HTTP API for accessing `ClickHouse` data

pub mod validation;

use alloy_primitives::Address;
use api_types::*;
use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
};
use chrono::{Duration as ChronoDuration, TimeZone, Utc};

use clickhouse_lib::{AddressBytes, ClickhouseReader};
use futures::stream::Stream;
use hex::encode;
use primitives::hardware::TOTAL_HARDWARE_COST_USD;
use std::{convert::Infallible, time::Duration as StdDuration};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use validation::{
    CommonQuery, PaginatedQuery, TimeRangeParams, has_time_range_params, range_duration,
    resolve_time_range_enum, resolve_time_range_since, validate_pagination,
    validate_range_exclusivity, validate_time_range,
};

/// Default maximum number of requests allowed during the rate limiting period.
pub const DEFAULT_MAX_REQUESTS: u64 = u64::MAX;
/// Default duration for the rate limiting window.
pub const DEFAULT_RATE_PERIOD: StdDuration = StdDuration::from_secs(1);
/// Maximum number of records returned by the `/block-transactions` endpoint.
pub const MAX_BLOCK_TRANSACTIONS_LIMIT: u64 = u64::MAX;

/// `OpenAPI` documentation structure
#[derive(Debug, OpenApi)]
#[openapi(
    paths(
        health,
        l2_head,
        l1_head,
        l2_head_block,
        l1_head_block,
        reorgs,
        active_gateways,
        batch_posting_times,
        avg_blobs_per_batch,
        blobs_per_batch,
        prove_times,
        verify_times,
        l1_block_times,
        l2_block_times,
        l2_gas_used,
        l2_tps,
        sequencer_distribution,
        sequencer_blocks,
        block_transactions,
        dashboard_data
    ),
    components(
        schemas(
            CommonQuery,
            PaginatedQuery,
            TimeRangeParams,
            L2HeadResponse,
            L1HeadResponse,
            L2HeadBlockResponse,
            L1HeadBlockResponse,
            ReorgEventsResponse,
            ActiveGatewaysResponse,
            BatchPostingTimesResponse,
            AvgBlobsPerBatchResponse,
            BatchBlobsResponse,
            ProveTimesResponse,
            VerifyTimesResponse,
            L1BlockTimesResponse,
            L2BlockTimesResponse,
            L2GasUsedResponse,
            L2TpsResponse,
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
            clickhouse_lib::L2TpsRow,
            clickhouse_lib::BatchBlobCountRow,
            clickhouse_lib::BatchPostingTimeRow,
            HealthResponse,
            PreconfDataResponse,
            DashboardDataResponse,
            api_types::ErrorResponse
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
#[derive(Clone)]
pub struct ApiState {
    client: ClickhouseReader,
}

impl std::fmt::Debug for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiState").finish_non_exhaustive()
    }
}

impl ApiState {
    /// Create a new [`ApiState`].
    pub const fn new(
        client: ClickhouseReader,
        _max_requests: u64,
        _rate_period: StdDuration,
    ) -> Self {
        Self { client }
    }
}

// Legacy type aliases for backward compatibility
type RangeQuery = CommonQuery;
type SequencerBlocksQuery = CommonQuery;
type BlockTransactionsQuery = PaginatedQuery;

/// Resolve the effective time range for queries, prioritizing explicit time range params (legacy
/// function)
fn _legacy_resolve_time_range(
    range: &Option<String>,
    time_params: &TimeRangeParams,
) -> chrono::DateTime<Utc> {
    let now = Utc::now();

    // If explicit time range parameters are provided, use them
    let lower_bound = time_params.created_gt.map(|v| v + 1).or(time_params.created_gte);

    if let Some(timestamp_ms) = lower_bound {
        if let Some(dt) = Utc.timestamp_millis_opt(timestamp_ms as i64).single() {
            // Clamp to reasonable range (last 7 days minimum)
            let min_time = now - ChronoDuration::days(7);
            return if dt < min_time { min_time } else { dt };
        }
    }

    // Fall back to range parameter or default
    let start = now - range_duration(range);
    let limit = now - ChronoDuration::days(7);
    if start < limit { limit } else { start }
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health", body = HealthResponse)
    ),
    tag = "taikoscope"
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok".to_owned() })
}

#[utoipa::path(
    get,
    path = "/l2-head",
    responses(
        (status = 200, description = "L2 head timestamp", body = L2HeadResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_head(State(state): State<ApiState>) -> Result<Json<L2HeadResponse>, ErrorResponse> {
    let ts = state.client.get_last_l2_head_time().await.map_err(|e| {
        tracing::error!("Failed to get L2 head time: {}", e);
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    let resp = L2HeadResponse { last_l2_head_time: ts.map(|t| t.to_rfc3339()) };
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/l1-head",
    responses(
        (status = 200, description = "L1 head timestamp", body = L1HeadResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l1_head(State(state): State<ApiState>) -> Result<Json<L1HeadResponse>, ErrorResponse> {
    let ts = state.client.get_last_l1_head_time().await.map_err(|e| {
        tracing::error!("Failed to get L1 head time: {}", e);
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    let resp = L1HeadResponse { last_l1_head_time: ts.map(|t| t.to_rfc3339()) };
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/l2-head-block",
    responses(
        (status = 200, description = "L2 head block number", body = L2HeadBlockResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_head_block(
    State(state): State<ApiState>,
) -> Result<Json<L2HeadBlockResponse>, ErrorResponse> {
    let num = state.client.get_last_l2_block_number().await.map_err(|e| {
        tracing::error!("Failed to get L2 head block number: {}", e);
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;
    Ok(Json(L2HeadBlockResponse { l2_head_block: num }))
}

#[utoipa::path(
    get,
    path = "/l1-head-block",
    responses(
        (status = 200, description = "L1 head block number", body = L1HeadBlockResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l1_head_block(
    State(state): State<ApiState>,
) -> Result<Json<L1HeadBlockResponse>, ErrorResponse> {
    let num = state.client.get_last_l1_block_number().await.map_err(|e| {
        tracing::error!("Failed to get L1 head block number: {}", e);
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;
    Ok(Json(L1HeadBlockResponse { l1_head_block: num }))
}

async fn sse_l2_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l2_block_number().await.ok().flatten().unwrap_or(0);
    let mut error_count = 0;
    let mut last_successful_fetch = std::time::Instant::now();

    let stream = stream! {
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));

        loop {
            // Add timeout to the database query to prevent long-running requests
            let fetch_result = tokio::time::timeout(
                StdDuration::from_secs(30), // 30 second timeout for database queries
                state.client.get_last_l2_block_number()
            ).await;

            match fetch_result {
                Ok(Ok(Some(num))) if num != last => {
                    last = num;
                    error_count = 0; // Reset error count on success
                    last_successful_fetch = std::time::Instant::now();
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(Ok(_)) => {
                    // No change in block number, reset error count
                    error_count = 0;
                    last_successful_fetch = std::time::Instant::now();
                }
                Ok(Err(e)) => {
                    error_count += 1;
                    tracing::error!("Failed to fetch L2 head block (attempt {}): {}", error_count, e);

                    // If we've had many consecutive errors, send the last known value
                    if error_count >= 5 && last_successful_fetch.elapsed() > StdDuration::from_secs(60) {
                        tracing::warn!("L2 head SSE: Using cached value due to persistent database errors");
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
                Err(_timeout) => {
                    error_count += 1;
                    tracing::error!("Timeout fetching L2 head block (attempt {})", error_count);

                    // On timeout, send cached value to keep connection alive
                    if error_count >= 3 {
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
            }

            // Adaptive sleep interval based on error state
            let sleep_duration = if error_count > 0 {
                // Back off when there are errors
                StdDuration::from_secs((error_count as u64).min(10))
            } else {
                StdDuration::from_secs(1)
            };

            tokio::time::sleep(sleep_duration).await;
        }
    };

    // More aggressive keep-alive settings to prevent proxy timeouts
    let keep_alive = KeepAlive::new().interval(StdDuration::from_secs(15)).text("keepalive");

    Sse::new(stream).keep_alive(keep_alive)
}

async fn sse_l1_head(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut last = state.client.get_last_l1_block_number().await.ok().flatten().unwrap_or(0);
    let mut error_count = 0;
    let mut last_successful_fetch = std::time::Instant::now();

    let stream = stream! {
        // send current head immediately
        yield Ok(Event::default().data(last.to_string()));

        loop {
            // Add timeout to the database query to prevent long-running requests
            let fetch_result = tokio::time::timeout(
                StdDuration::from_secs(30), // 30 second timeout for database queries
                state.client.get_last_l1_block_number()
            ).await;

            match fetch_result {
                Ok(Ok(Some(num))) if num != last => {
                    last = num;
                    error_count = 0; // Reset error count on success
                    last_successful_fetch = std::time::Instant::now();
                    yield Ok(Event::default().data(num.to_string()));
                }
                Ok(Ok(_)) => {
                    // No change in block number, reset error count
                    error_count = 0;
                    last_successful_fetch = std::time::Instant::now();
                }
                Ok(Err(e)) => {
                    error_count += 1;
                    tracing::error!("Failed to fetch L1 head block (attempt {}): {}", error_count, e);

                    // If we've had many consecutive errors, send the last known value
                    if error_count >= 5 && last_successful_fetch.elapsed() > StdDuration::from_secs(60) {
                        tracing::warn!("L1 head SSE: Using cached value due to persistent database errors");
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
                Err(_timeout) => {
                    error_count += 1;
                    tracing::error!("Timeout fetching L1 head block (attempt {})", error_count);

                    // On timeout, send cached value to keep connection alive
                    if error_count >= 3 {
                        yield Ok(Event::default().data(last.to_string()));
                    }
                }
            }

            // Adaptive sleep interval based on error state
            let sleep_duration = if error_count > 0 {
                // Back off when there are errors
                StdDuration::from_secs((error_count as u64).min(10))
            } else {
                StdDuration::from_secs(1)
            };

            tokio::time::sleep(sleep_duration).await;
        }
    };

    // More aggressive keep-alive settings to prevent proxy timeouts
    let keep_alive = KeepAlive::new().interval(StdDuration::from_secs(15)).text("keepalive");

    Sse::new(stream).keep_alive(keep_alive)
}

#[utoipa::path(
    get,
    path = "/reorgs",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Reorg events", body = ReorgEventsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn reorgs(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ReorgEventsResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let since = resolve_time_range_since(&params.range, &params.time_range);
    let events = state.client.get_l2_reorgs_since(since).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get reorg events");
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;
    tracing::info!(count = events.len(), "Returning reorg events");
    Ok(Json(ReorgEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/active-gateways",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Active gateways", body = ActiveGatewaysResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn active_gateways(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ActiveGatewaysResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let since = resolve_time_range_since(&params.range, &params.time_range);
    let gateways = state.client.get_active_gateways_since(since).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get active gateways");
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;
    let gateways: Vec<String> = gateways.into_iter().map(|a| format!("0x{}", encode(a))).collect();
    tracing::info!(count = gateways.len(), "Returning active gateways");
    Ok(Json(ActiveGatewaysResponse { gateways }))
}

#[utoipa::path(
    get,
    path = "/batch-posting-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Batch posting times", body = BatchPostingTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn batch_posting_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchPostingTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let rows = match state.client.get_batch_posting_times(time_range).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get batch posting times");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = rows.len(), "Returning batch posting times");
    Ok(Json(BatchPostingTimesResponse { batches: rows }))
}

#[utoipa::path(
    get,
    path = "/avg-blobs-per-batch",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Average blobs per batch", body = AvgBlobsPerBatchResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn avg_blobs_per_batch(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<AvgBlobsPerBatchResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let avg = match state.client.get_avg_blobs_per_batch(time_range).await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get avg blobs per batch");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(avg_blobs_per_batch = ?avg, "Returning avg blobs per batch");
    Ok(Json(AvgBlobsPerBatchResponse { avg_blobs: avg }))
}

#[utoipa::path(
    get,
    path = "/blobs-per-batch",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Blobs per batch", body = BatchBlobsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn blobs_per_batch(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchBlobsResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let batches = match state.client.get_blobs_per_batch(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get blobs per batch");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = batches.len(), "Returning blobs per batch");
    Ok(Json(BatchBlobsResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/prove-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Prove times", body = ProveTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn prove_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ProveTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let batches = match state.client.get_prove_times(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get prove times");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = batches.len(), "Returning prove times");
    Ok(Json(ProveTimesResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/verify-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Verify times", body = VerifyTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn verify_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<VerifyTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let batches = match state.client.get_verify_times(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get verify times");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = batches.len(), "Returning verify times");
    Ok(Json(VerifyTimesResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/l1-block-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L1 block times", body = L1BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l1_block_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L1BlockTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let blocks = match state.client.get_l1_block_times(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L1 block times");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = blocks.len(), "Returning L1 block times");
    Ok(Json(L1BlockTimesResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-block-times",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 block times", body = L2BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_block_times(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2BlockTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = params.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
        Ok(a) => Some(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse address");
            None
        }
    });
    let blocks = match state.client.get_l2_block_times(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block times");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    tracing::info!(count = blocks.len(), "Returning L2 block times");
    Ok(Json(L2BlockTimesResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-gas-used",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 gas used", body = L2GasUsedResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_gas_used(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2GasUsedResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = params.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
        Ok(a) => Some(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse address");
            None
        }
    });
    let blocks = match state.client.get_l2_gas_used(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 gas used: {}", e);
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    Ok(Json(L2GasUsedResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-tps",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L2 TPS", body = L2TpsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn l2_tps(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2TpsResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = params.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
        Ok(a) => Some(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse address");
            None
        }
    });
    let blocks = match state.client.get_l2_tps(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 TPS: {}", e);
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
        }
    };
    Ok(Json(L2TpsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/sequencer-distribution",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Sequencer distribution", body = SequencerDistributionResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn sequencer_distribution(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<SequencerDistributionResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let since = resolve_time_range_since(&params.range, &params.time_range);
    let rows = state.client.get_sequencer_distribution_since(since).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get sequencer distribution");
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;
    let sequencers: Vec<SequencerDistributionItem> = rows
        .into_iter()
        .map(|r| {
            let tps = (r.max_ts > r.min_ts && r.tx_sum > 0)
                .then(|| r.tx_sum as f64 / (r.max_ts - r.min_ts) as f64);
            SequencerDistributionItem {
                address: format!("0x{}", encode(r.sequencer)),
                blocks: r.blocks,
                tps,
            }
        })
        .collect();
    tracing::info!(count = sequencers.len(), "Returning sequencer distribution");
    Ok(Json(SequencerDistributionResponse { sequencers }))
}

#[utoipa::path(
    get,
    path = "/sequencer-blocks",
    params(
        SequencerBlocksQuery
    ),
    responses(
        (status = 200, description = "Sequencer blocks", body = SequencerBlocksResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn sequencer_blocks(
    Query(params): Query<SequencerBlocksQuery>,
    State(state): State<ApiState>,
) -> Result<Json<SequencerBlocksResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.time_range)?;

    // Check for range exclusivity
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let since = resolve_time_range_since(&params.range, &params.time_range);
    let rows = state.client.get_sequencer_blocks_since(since).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get sequencer blocks");
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    let filter = params.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
        Ok(a) => Some(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse address");
            None
        }
    });

    use std::collections::BTreeMap;
    let mut map: BTreeMap<AddressBytes, Vec<u64>> = BTreeMap::new();
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
    Ok(Json(SequencerBlocksResponse { sequencers }))
}

#[utoipa::path(
    get,
    path = "/block-transactions",
    params(
        BlockTransactionsQuery
    ),
    responses(
        (status = 200, description = "Block transactions", body = BlockTransactionsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn block_transactions(
    Query(params): Query<BlockTransactionsQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BlockTransactionsResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.common.time_range)?;

    // Validate pagination parameters
    validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_BLOCK_TRANSACTIONS_LIMIT,
    )?;

    // Check for range exclusivity between time range and slot range
    let has_time_range = has_time_range_params(&params.common.time_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_time_range, has_slot_range)?;

    let since = resolve_time_range_since(&params.common.range, &params.common.time_range);
    let limit = params.limit.unwrap_or(MAX_BLOCK_TRANSACTIONS_LIMIT);

    let rows = match state
        .client
        .get_block_transactions_paginated(
            since,
            limit,
            params.starting_after,
            params.ending_before,
            params.common.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
                Ok(a) => Some(AddressBytes::from(a)),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to parse address");
                    None
                }
            }),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get block transactions");
            return Err(ErrorResponse::new(
                "database-error",
                "Database error",
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ));
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
    Ok(Json(BlockTransactionsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/dashboard-data",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated dashboard data", body = DashboardDataResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
async fn dashboard_data(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<DashboardDataResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;

    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let since = resolve_time_range_since(&params.range, &params.time_range);
    let address = params.address.as_ref().and_then(|addr| match addr.parse::<Address>() {
        Ok(a) => Some(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse address");
            None
        }
    });

    let (
        l2_block_cadence,
        batch_posting_cadence,
        avg_prove_time,
        avg_verify_time,
        avg_tps,
        preconf,
        reorgs,
        slashings,
        forced_inclusions,
        l2_block,
        l1_block,
        l2_tx_fee,
    ) = tokio::try_join!(
        state.client.get_l2_block_cadence(address, time_range),
        state.client.get_batch_posting_cadence(time_range),
        state.client.get_avg_prove_time(time_range),
        state.client.get_avg_verify_time(time_range),
        state.client.get_avg_l2_tps(address, time_range),
        state.client.get_last_preconf_data(),
        state.client.get_l2_reorgs_since(since),
        state.client.get_slashing_events_since(since),
        state.client.get_forced_inclusions_since(since),
        state.client.get_last_l2_block_number(),
        state.client.get_last_l1_block_number(),
        state.client.get_l2_tx_fee(address, time_range)
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get dashboard data");
        ErrorResponse::new(
            "database-error",
            "Database error",
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    let preconf_data = preconf.map(|d| PreconfDataResponse {
        candidates: d.candidates.into_iter().map(|a| format!("0x{}", encode(a))).collect(),
        current_operator: d.current_operator.map(|a| format!("0x{}", encode(a))),
        next_operator: d.next_operator.map(|a| format!("0x{}", encode(a))),
    });

    let hours = time_range.seconds() as f64 / 3600.0;
    let hourly_rate = TOTAL_HARDWARE_COST_USD / (30.0 * 24.0);
    let cost = hourly_rate * hours;

    Ok(Json(DashboardDataResponse {
        l2_block_cadence_ms: l2_block_cadence,
        batch_posting_cadence_ms: batch_posting_cadence,
        avg_prove_time_ms: avg_prove_time,
        avg_verify_time_ms: avg_verify_time,
        avg_tps,
        preconf_data,
        l2_reorgs: reorgs.len(),
        slashings: slashings.len(),
        forced_inclusions: forced_inclusions.len(),
        l2_block,
        l1_block,
        l2_tx_fee,
        cloud_cost: Some(cost),
    }))
}

/// Build the router with all API endpoints.
pub fn router(state: ApiState) -> Router {
    let api_routes = Router::new()
        .route("/l2-head", get(l2_head))
        .route("/l1-head", get(l1_head))
        .route("/l2-head-block", get(l2_head_block))
        .route("/l1-head-block", get(l1_head_block))
        .route("/sse/l1-head", get(sse_l1_head))
        .route("/sse/l2-head", get(sse_l2_head))
        .route("/reorgs", get(reorgs))
        .route("/active-gateways", get(active_gateways))
        .route("/batch-posting-times", get(batch_posting_times))
        .route("/avg-blobs-per-batch", get(avg_blobs_per_batch))
        .route("/blobs-per-batch", get(blobs_per_batch))
        .route("/prove-times", get(prove_times))
        .route("/verify-times", get(verify_times))
        .route("/l1-block-times", get(l1_block_times))
        .route("/l2-block-times", get(l2_block_times))
        .route("/l2-gas-used", get(l2_gas_used))
        .route("/l2-tps", get(l2_tps))
        .route("/tps", get(l2_tps))
        .route("/sequencer-distribution", get(sequencer_distribution))
        .route("/sequencer-blocks", get(sequencer_blocks))
        .route("/block-transactions", get(block_transactions))
        .route("/dashboard-data", get(dashboard_data));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health))
        .merge(api_routes)
        .with_state(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{self, Body},
        http::{Request, StatusCode},
    };
    use chrono::{TimeZone, Utc};
    use clickhouse::{
        Row,
        test::{Mock, handlers},
    };

    use serde::Serialize;
    use serde_json::{Value, json};
    use std::time::Duration as StdDuration;
    use tower::util::ServiceExt;
    use url::Url;

    #[derive(Serialize, Row)]
    struct MaxRow {
        block_ts: u64,
    }

    #[derive(Serialize, Row)]
    struct L2BlockNumber {
        l2_block_number: u64,
    }

    #[derive(Serialize, Row)]
    struct L1BlockNumber {
        l1_block_number: u64,
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
    async fn health_endpoint() {
        let mock = Mock::new();
        let app = build_app(mock.url());
        let body = send_request(app, "/health").await;
        assert_eq!(body, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn health_not_rate_limited() {
        let mock = Mock::new();
        let url = Url::parse(mock.url()).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, 1, StdDuration::from_secs(60));
        let app = router(state);

        let resp1 = app
            .clone()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let resp2 = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
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
        mock.add(handlers::provide(vec![L2BlockNumber { l2_block_number: 1 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-head-block").await;
        assert_eq!(body, json!({ "l2_head_block": 1 }));
    }

    #[tokio::test]
    async fn l1_head_block_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![L1BlockNumber { l1_block_number: 2 }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l1-head-block").await;
        assert_eq!(body, json!({ "l1_head_block": 2 }));
    }

    #[derive(Serialize, Row)]
    struct PostingTimeRowTest {
        batch_id: u64,
        ts: u64,
        ms_since_prev_batch: Option<u64>,
    }

    #[tokio::test]
    async fn batch_posting_times_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![
            PostingTimeRowTest { batch_id: 1, ts: 1000, ms_since_prev_batch: None },
            PostingTimeRowTest { batch_id: 2, ts: 2000, ms_since_prev_batch: Some(1000) },
        ]));
        let app = build_app(mock.url());
        let body = send_request(app, "/batch-posting-times?range=1h").await;
        assert_eq!(
            body,
            json!({ "batches": [ { "batch_id": 2, "inserted_at": "1970-01-01T00:00:02Z", "ms_since_prev_batch": 1000 } ] })
        );
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
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 0, "gas_used": 0 }, { "l2_block_number": 1, "gas_used": 42 } ] })
        );
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
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 0, "gas_used": 0 }, { "l2_block_number": 1, "gas_used": 42 } ] })
        );
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
        assert_eq!(
            body,
            json!({ "blocks": [ { "l2_block_number": 0, "gas_used": 0 }, { "l2_block_number": 1, "gas_used": 42 } ] })
        );
    }

    #[derive(Serialize, Row)]
    struct L2TpsRowTest {
        l2_block_number: u64,
        sum_tx: u32,
        ms_since_prev_block: Option<u64>,
    }

    #[tokio::test]
    async fn l2_tps_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![L2TpsRowTest {
            l2_block_number: 1,
            sum_tx: 6,
            ms_since_prev_block: Some(2000),
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/l2-tps").await;
        assert_eq!(body, json!({ "blocks": [ { "l2_block_number": 1, "tps": 3.0 } ] }));
    }

    #[derive(Serialize, Row)]
    struct SequencerRowTest {
        sequencer: AddressBytes,
        blocks: u64,
        min_ts: u64,
        max_ts: u64,
        tx_sum: u64,
    }

    #[tokio::test]
    async fn sequencer_distribution_endpoint() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![SequencerRowTest {
            sequencer: AddressBytes([1u8; 20]),
            blocks: 5,
            min_ts: 100,
            max_ts: 200,
            tx_sum: 500,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/sequencer-distribution?range=1h").await;
        assert_eq!(
            body,
            json!({ "sequencers": [ { "address": "0x0101010101010101010101010101010101010101", "blocks": 5, "tps": 5.0 } ] })
        );
    }

    #[tokio::test]
    async fn sequencer_blocks_endpoint() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct SeqBlockRowTest {
            sequencer: AddressBytes,
            l2_block_number: u64,
        }
        mock.add(handlers::provide(vec![SeqBlockRowTest {
            sequencer: AddressBytes([1u8; 20]),
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
            sequencer: AddressBytes,
            l2_block_number: u64,
            sum_tx: u32,
        }
        mock.add(handlers::provide(vec![TxRowTest {
            sequencer: AddressBytes([1u8; 20]),
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
            l1_block_number: u64,
            batch_id: u64,
            blob_count: u8,
        }
        mock.add(handlers::provide(vec![BlobRowTest {
            l1_block_number: 10,
            batch_id: 1,
            blob_count: 3,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/blobs-per-batch?range=1h").await;
        assert_eq!(
            body,
            json!({
                "batches": [ { "l1_block_number": 10, "batch_id": 1, "blob_count": 3 } ]
            })
        );
    }

    #[test]
    fn openapi_spec_is_valid() {
        let openapi = ApiDoc::openapi();

        // Basic structural validation
        assert_eq!(openapi.info.title, "Taikoscope API");
        assert_eq!(openapi.info.version, "0.1.0");
        assert!(!openapi.paths.paths.is_empty(), "OpenAPI spec should have paths defined");

        // Verify all expected endpoints are documented
        let expected_paths = [
            "/health",
            "/l2-head",
            "/l1-head",
            "/l2-head-block",
            "/l1-head-block",
            "/reorgs",
            "/active-gateways",
            "/avg-blobs-per-batch",
            "/blobs-per-batch",
            "/prove-times",
            "/verify-times",
            "/l1-block-times",
            "/l2-block-times",
            "/l2-gas-used",
            "/l2-tps",
            "/sequencer-distribution",
            "/sequencer-blocks",
            "/block-transactions",
            "/dashboard-data",
        ];

        for path in expected_paths {
            assert!(openapi.paths.paths.contains_key(path), "OpenAPI spec missing path: {path}");
        }

        // Verify all paths have GET operations
        for (path, path_item) in &openapi.paths.paths {
            assert!(path_item.get.is_some(), "Path {path} should have GET operation defined");
        }

        // Verify essential components are defined
        assert!(openapi.components.is_some(), "OpenAPI spec should have components defined");

        if let Some(components) = &openapi.components {
            assert!(!components.schemas.is_empty(), "OpenAPI spec should have schemas defined");
        }
    }

    #[tokio::test]
    async fn no_rate_limiting() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        let url = Url::parse(mock.url()).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, 1, StdDuration::from_secs(60));
        let app = router(state);

        let _ = app
            .clone()
            .oneshot(Request::builder().uri("/l2-head").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let resp = app
            .oneshot(Request::builder().uri("/l2-head").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should succeed since rate limiting is disabled
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn multiple_requests_succeed_without_rate_limiting() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        let url = Url::parse(mock.url()).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, 1, StdDuration::from_secs(60));
        let app = router(state);

        let req = Request::builder()
            .uri("/l2-head")
            .header("X-Forwarded-For", "203.0.113.10")
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .uri("/l2-head")
            .header("X-Forwarded-For", "203.0.113.10")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Should succeed since rate limiting is disabled
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn forwarded_header_requests_succeed() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        let url = Url::parse(mock.url()).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, 1, StdDuration::from_secs(60));
        let app = router(state);

        let req = Request::builder()
            .uri("/l2-head")
            .header("Forwarded", "for=\"203.0.113.20\";proto=https")
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .uri("/l2-head")
            .header("Forwarded", "for=\"203.0.113.20\";proto=https")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Should succeed since rate limiting is disabled
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn x_real_ip_requests_succeed() {
        let mock = Mock::new();
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        mock.add(handlers::provide(vec![MaxRow { block_ts: 42u64 }]));
        let url = Url::parse(mock.url()).unwrap();
        let client =
            ClickhouseReader::new(url, "test-db".to_owned(), "user".into(), "pass".into()).unwrap();
        let state = ApiState::new(client, 1, StdDuration::from_secs(60));
        let app = router(state);

        let req = Request::builder()
            .uri("/l2-head")
            .header("X-Real-IP", "203.0.113.30")
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .uri("/l2-head")
            .header("X-Real-IP", "203.0.113.30")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Should succeed since rate limiting is disabled
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn range_duration_default_is_one_hour() {
        let d = range_duration(&None);
        assert_eq!(d.num_hours(), 1);
    }

    #[test]
    fn range_duration_allows_long_hours() {
        let d = range_duration(&Some("200h".to_owned()));
        assert_eq!(d.num_hours(), 200);
    }

    #[test]
    fn range_duration_allows_long_days() {
        let d = range_duration(&Some("10d".to_owned()));
        assert_eq!(d.num_hours(), 240);
    }

    #[test]
    fn range_duration_invalid_string_defaults() {
        let d = range_duration(&Some("invalid".to_owned()));
        assert_eq!(d.num_hours(), 1);
    }

    #[test]
    fn range_duration_custom_hours() {
        let d = range_duration(&Some("5h".to_owned()));
        assert_eq!(d.num_hours(), 5);
    }

    #[test]
    fn range_duration_sql_injection() {
        let d = range_duration(&Some("1h; DROP TABLE".to_owned()));
        assert_eq!(d.num_hours(), 1);
    }

    #[test]
    fn range_duration_handles_zero() {
        let d = range_duration(&Some("0h".to_owned()));
        assert_eq!(d.num_hours(), 0);

        let d = range_duration(&Some("0d".to_owned()));
        assert_eq!(d.num_hours(), 0);
    }

    #[test]
    fn range_duration_trims_whitespace() {
        let d = range_duration(&Some(" 2h ".to_owned()));
        assert_eq!(d.num_hours(), 2);

        let d = range_duration(&Some(" 1d ".to_owned()));
        assert_eq!(d.num_hours(), 24);
    }

    #[test]
    fn range_duration_empty_string_defaults() {
        let d = range_duration(&Some(String::new()));
        assert_eq!(d.num_hours(), 1);
    }

    #[test]
    fn range_duration_invalid_decimal() {
        let d = range_duration(&Some("1.5h".to_owned()));
        assert_eq!(d.num_hours(), 1);
    }

    #[test]
    fn range_duration_parses_days() {
        let d = range_duration(&Some("3d".to_owned()));
        assert_eq!(d.num_hours(), 72);
    }

    #[test]
    fn range_duration_parses_minutes() {
        let d = range_duration(&Some("15m".to_owned()));
        assert_eq!(d.num_minutes(), 15);
    }

    #[tokio::test]
    async fn sequencer_blocks_invalid_address() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct SeqBlockRowTest {
            sequencer: AddressBytes,
            l2_block_number: u64,
        }
        mock.add(handlers::provide(vec![SeqBlockRowTest {
            sequencer: AddressBytes([1u8; 20]),
            l2_block_number: 42,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/sequencer-blocks?range=1h&address=zzz").await;
        assert_eq!(
            body,
            json!({
                "sequencers": [
                    { "address": "0x0101010101010101010101010101010101010101", "blocks": [42] }
                ]
            })
        );
    }

    #[tokio::test]
    async fn block_transactions_invalid_address() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct TxRowTest {
            sequencer: AddressBytes,
            l2_block_number: u64,
            sum_tx: u32,
        }
        mock.add(handlers::provide(vec![TxRowTest {
            sequencer: AddressBytes([1u8; 20]),
            l2_block_number: 42,
            sum_tx: 7,
        }]));
        let app = build_app(mock.url());
        let body = send_request(app, "/block-transactions?range=1h&address=zzz").await;
        assert_eq!(
            body,
            json!({
                "blocks": [
                    { "block": 42, "txs": 7, "sequencer": "0x0101010101010101010101010101010101010101" }
                ]
            })
        );
    }

    #[tokio::test]
    async fn sequencer_blocks_sql_injection() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct SeqBlockRowTest {
            sequencer: AddressBytes,
            l2_block_number: u64,
        }
        mock.add(handlers::provide(vec![SeqBlockRowTest {
            sequencer: AddressBytes([1u8; 20]),
            l2_block_number: 42,
        }]));
        let app = build_app(mock.url());
        let addr = "0x0101010101010101010101010101010101010101%27;%20DROP%20TABLE%20--";
        let uri = format!("/sequencer-blocks?range=1h&address={addr}");
        let body = send_request(app, &uri).await;
        assert_eq!(
            body,
            json!({
                "sequencers": [
                    { "address": "0x0101010101010101010101010101010101010101", "blocks": [42] }
                ]
            })
        );
    }

    #[tokio::test]
    async fn block_transactions_sql_injection() {
        let mock = Mock::new();
        #[derive(Serialize, Row)]
        struct TxRowTest {
            sequencer: AddressBytes,
            l2_block_number: u64,
            sum_tx: u32,
        }
        mock.add(handlers::provide(vec![TxRowTest {
            sequencer: AddressBytes([1u8; 20]),
            l2_block_number: 42,
            sum_tx: 7,
        }]));
        let app = build_app(mock.url());
        let addr = "0x123%27;%20--";
        let uri = format!("/block-transactions?range=1h&address={addr}");
        let body = send_request(app, &uri).await;
        assert_eq!(
            body,
            json!({
                "blocks": [
                    { "block": 42, "txs": 7, "sequencer": "0x0101010101010101010101010101010101010101" }
                ]
            })
        );
    }
}

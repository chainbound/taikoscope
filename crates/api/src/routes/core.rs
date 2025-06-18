//! Core simple API endpoints

use crate::{
    state::{ApiState, MAX_TABLE_LIMIT},
    validation::{
        CommonQuery, PaginatedQuery, has_time_range_params, resolve_time_range_enum,
        resolve_time_range_since, validate_pagination, validate_range_exclusivity,
        validate_time_range,
    },
};
use alloy_primitives::Address;
use api_types::{
    ActiveGatewaysResponse, AvgBlobsPerBatchResponse, BatchBlobsResponse,
    BatchPostingTimesResponse, ErrorResponse, L1BlockTimesResponse, L1DataCostResponse,
    L1HeadBlockResponse, L1HeadResponse, L2HeadBlockResponse, L2HeadResponse, ProveTimesResponse,
    SequencerBlocksItem, SequencerBlocksResponse, SequencerDistributionItem,
    SequencerDistributionResponse, VerifyTimesResponse,
};
use axum::{
    Json,
    extract::{Query, State},
};
use clickhouse_lib::AddressBytes;
use hex::encode;

// Legacy type aliases for backward compatibility
type RangeQuery = CommonQuery;

#[utoipa::path(
    get,
    path = "/l2-head",
    responses(
        (status = 200, description = "L2 head timestamp", body = L2HeadResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get the timestamp of the latest L2 block
pub async fn l2_head(State(state): State<ApiState>) -> Result<Json<L2HeadResponse>, ErrorResponse> {
    let ts = state.client.get_last_l2_head_time().await.map_err(|e| {
        tracing::error!("Failed to get L2 head time: {}", e);
        ErrorResponse::database_error()
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
/// Get the timestamp of the latest L1 block
pub async fn l1_head(State(state): State<ApiState>) -> Result<Json<L1HeadResponse>, ErrorResponse> {
    let ts = state.client.get_last_l1_head_time().await.map_err(|e| {
        tracing::error!("Failed to get L1 head time: {}", e);
        ErrorResponse::database_error()
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
/// Get the block number of the latest L2 block
pub async fn l2_head_block(
    State(state): State<ApiState>,
) -> Result<Json<L2HeadBlockResponse>, ErrorResponse> {
    let num = state.client.get_last_l2_block_number().await.map_err(|e| {
        tracing::error!("Failed to get L2 head block number: {}", e);
        ErrorResponse::database_error()
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
/// Get the block number of the latest L1 block
pub async fn l1_head_block(
    State(state): State<ApiState>,
) -> Result<Json<L1HeadBlockResponse>, ErrorResponse> {
    let num = state.client.get_last_l1_block_number().await.map_err(|e| {
        tracing::error!("Failed to get L1 head block number: {}", e);
        ErrorResponse::database_error()
    })?;
    Ok(Json(L1HeadBlockResponse { l1_head_block: num }))
}

#[utoipa::path(
    get,
    path = "/active-gateways",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Active gateways", body = ActiveGatewaysResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get list of gateway addresses that have been active in the specified time range
pub async fn active_gateways(
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
        ErrorResponse::database_error()
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
/// Get batch posting timing metrics for the specified time range
pub async fn batch_posting_times(
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
            return Err(ErrorResponse::database_error());
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
/// Get the average number of blobs per batch for the specified time range
pub async fn avg_blobs_per_batch(
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
            return Err(ErrorResponse::database_error());
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
/// Get detailed blob count information for each batch in the specified time range
pub async fn blobs_per_batch(
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
            return Err(ErrorResponse::database_error());
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
/// Get batch proving time metrics for the specified time range
pub async fn prove_times(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ProveTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.common.time_range)?;

    // Check for range exclusivity
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_time_range = has_time_range_params(&params.common.time_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_time_range, has_slot_range)?;

    let since = resolve_time_range_since(&params.common.range, &params.common.time_range);
    let batches = match state
        .client
        .get_prove_times_paginated(since, limit, params.starting_after, params.ending_before)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get prove times");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = batches.len(), "Returning prove times");
    Ok(Json(ProveTimesResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/verify-times",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Verify times", body = VerifyTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get batch verification time metrics for the specified time range
pub async fn verify_times(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<VerifyTimesResponse>, ErrorResponse> {
    // Validate time range parameters
    validate_time_range(&params.common.time_range)?;

    // Check for range exclusivity
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_time_range = has_time_range_params(&params.common.time_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_time_range, has_slot_range)?;

    let since = resolve_time_range_since(&params.common.range, &params.common.time_range);
    let batches = match state
        .client
        .get_verify_times_paginated(since, limit, params.starting_after, params.ending_before)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get verify times");
            return Err(ErrorResponse::database_error());
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
/// Get L1 block timing information for the specified time range
pub async fn l1_block_times(
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
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = blocks.len(), "Returning L1 block times");
    Ok(Json(L1BlockTimesResponse { blocks }))
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
/// Get the distribution of blocks and TPS across different sequencers
pub async fn sequencer_distribution(
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
        ErrorResponse::database_error()
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

// Legacy type aliases for backward compatibility
type SequencerBlocksQuery = CommonQuery;

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
/// Get the list of blocks produced by each sequencer
pub async fn sequencer_blocks(
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
        ErrorResponse::database_error()
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
    path = "/l1-data-cost",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "L1 data posting cost", body = L1DataCostResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get L1 data posting cost information for the specified time range
pub async fn l1_data_cost(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L1DataCostResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let rows = match state.client.get_l1_data_costs(time_range).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get L1 data cost: {}", e);
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = rows.len(), "Returning L1 data cost");
    Ok(Json(L1DataCostResponse { blocks: rows }))
}

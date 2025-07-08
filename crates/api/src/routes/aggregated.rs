//! Aggregated data endpoints with complex processing

use crate::{
    helpers::{
        aggregate_blobs_per_batch, aggregate_block_transactions, aggregate_l2_fee_components,
        aggregate_l2_gas_used, aggregate_l2_tps, aggregate_prove_times, aggregate_verify_times,
        blobs_bucket_size, bucket_size_from_range, prove_bucket_size, verify_bucket_size,
    },
    state::{ApiState, MAX_BLOCK_TRANSACTIONS_LIMIT},
    validation::{
        CommonQuery, has_time_range_params, resolve_time_range_enum, resolve_time_range_since,
        validate_range_exclusivity, validate_time_range,
    },
};
use alloy_primitives::Address;
use api_types::*;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use clickhouse_lib::{AddressBytes, BlockFeeComponentRow};
use hex::encode;
use primitives::WEI_PER_GWEI;

// Legacy type aliases for backward compatibility
type RangeQuery = CommonQuery;

#[utoipa::path(
    get,
    path = "/l2-block-times/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated L2 block times", body = L2BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated L2 block times with automatic bucketing based on time range
pub async fn l2_block_times_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2BlockTimesResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = if let Some(addr) = params.address.as_ref() {
        match addr.parse::<Address>() {
            Ok(a) => Some(AddressBytes::from(a)),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse address");
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    e.to_string(),
                ));
            }
        }
    } else {
        None
    };
    let bucket = bucket_size_from_range(&time_range);
    let blocks = match state.client.get_l2_block_times(address, time_range, Some(bucket)).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block times");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = blocks.len(), "Returning aggregated L2 block times");
    Ok(Json(L2BlockTimesResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-gas-used/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated L2 gas used", body = L2GasUsedResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated L2 gas usage with automatic bucketing based on time range
pub async fn l2_gas_used_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2GasUsedResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = if let Some(addr) = params.address.as_ref() {
        match addr.parse::<Address>() {
            Ok(a) => Some(AddressBytes::from(a)),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse address");
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    e.to_string(),
                ));
            }
        }
    } else {
        None
    };
    let blocks = match state.client.get_l2_gas_used(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 gas used: {}", e);
            return Err(ErrorResponse::database_error());
        }
    };
    let bucket = bucket_size_from_range(&time_range);
    let blocks = aggregate_l2_gas_used(blocks, bucket);
    tracing::info!(count = blocks.len(), "Returning aggregated L2 gas used");
    Ok(Json(L2GasUsedResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-tps/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated L2 TPS", body = L2TpsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated L2 transactions per second with automatic bucketing based on time range
pub async fn l2_tps_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2TpsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = if let Some(addr) = params.address.as_ref() {
        match addr.parse::<Address>() {
            Ok(a) => Some(AddressBytes::from(a)),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse address");
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    e.to_string(),
                ));
            }
        }
    } else {
        None
    };
    let blocks = match state.client.get_l2_tps(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 TPS");
            return Err(ErrorResponse::database_error());
        }
    };
    let bucket = bucket_size_from_range(&time_range);
    let blocks = aggregate_l2_tps(blocks, bucket);
    tracing::info!(count = blocks.len(), "Returning aggregated L2 TPS");
    Ok(Json(L2TpsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/block-transactions/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated block transactions", body = BlockTransactionsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated block transaction counts with automatic bucketing based on time range.
///
/// Results are ordered by block number in descending order before aggregation.
pub async fn block_transactions_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BlockTransactionsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let since = resolve_time_range_since(&params.range, &params.time_range);
    let address = if let Some(addr) = params.address.as_ref() {
        match addr.parse::<Address>() {
            Ok(a) => Some(AddressBytes::from(a)),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse address");
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    e.to_string(),
                ));
            }
        }
    } else {
        None
    };

    let rows = match state
        .client
        .get_block_transactions_paginated(since, MAX_BLOCK_TRANSACTIONS_LIMIT, None, None, address)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get block transactions");
            return Err(ErrorResponse::database_error());
        }
    };

    let blocks: Vec<BlockTransactionsItem> = rows
        .into_iter()
        .map(|r| BlockTransactionsItem {
            block: r.l2_block_number,
            txs: r.sum_tx,
            sequencer: format!("0x{}", encode(r.sequencer)),
            block_time: r.block_time,
        })
        .collect();

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let bucket = bucket_size_from_range(&time_range);
    let blocks = aggregate_block_transactions(blocks, bucket);
    tracing::info!(count = blocks.len(), "Returning aggregated block transactions");
    Ok(Json(BlockTransactionsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/prove-times/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated prove times", body = ProveTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated batch proving time metrics with automatic bucketing based on time range
pub async fn prove_times_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ProveTimesResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let batches = match state.client.get_prove_times(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get prove times");
            return Err(ErrorResponse::database_error());
        }
    };

    let bucket = prove_bucket_size(&time_range);
    let batches = aggregate_prove_times(batches, bucket);
    tracing::info!(count = batches.len(), "Returning aggregated prove times");
    Ok(Json(ProveTimesResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/verify-times/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated verify times", body = VerifyTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated batch verification time metrics with automatic bucketing based on time range
pub async fn verify_times_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<VerifyTimesResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let batches = match state.client.get_verify_times(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get verify times");
            return Err(ErrorResponse::database_error());
        }
    };

    let bucket = verify_bucket_size(&time_range);
    let batches = aggregate_verify_times(batches, bucket);
    tracing::info!(count = batches.len(), "Returning aggregated verify times");
    Ok(Json(VerifyTimesResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/prove-costs",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated prover costs", body = ProposerCostsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated prover costs grouped by proposer
pub async fn prove_costs(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ProposerCostsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;

    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);

    let rows = state.client.get_prove_costs_by_proposer(time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get prover costs");
        ErrorResponse::database_error()
    })?;

    let proposers: Vec<ProposerCostItem> = rows
        .into_iter()
        .map(|(addr, cost)| ProposerCostItem {
            address: format!("0x{}", encode(addr)),
            cost: cost / WEI_PER_GWEI,
        })
        .collect();

    tracing::info!(count = proposers.len(), "Returning prover costs");
    Ok(Json(ProposerCostsResponse { proposers }))
}

#[utoipa::path(
    get,
    path = "/l2-fee-components/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated fee components", body = FeeComponentsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated fee components with automatic bucketing based on time range
pub async fn l2_fee_components_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<FeeComponentsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;

    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let address = if let Some(addr) = params.address.as_ref() {
        match addr.parse::<Address>() {
            Ok(a) => Some(AddressBytes::from(a)),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse address");
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    e.to_string(),
                ));
            }
        }
    } else {
        None
    };

    let blocks = state.client.get_l2_fee_components(address, time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get fee components");
        ErrorResponse::database_error()
    })?;

    let bucket = bucket_size_from_range(&time_range);
    let blocks = aggregate_l2_fee_components(blocks, bucket);
    let blocks: Vec<BlockFeeComponentRow> = blocks
        .into_iter()
        .map(|r| BlockFeeComponentRow {
            l2_block_number: r.l2_block_number,
            priority_fee: r.priority_fee / WEI_PER_GWEI,
            base_fee: r.base_fee / WEI_PER_GWEI,
            l1_data_cost: r.l1_data_cost.map(|v| v / WEI_PER_GWEI),
        })
        .collect();

    Ok(Json(FeeComponentsResponse { blocks }))
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
/// Get comprehensive dashboard data including metrics, block info, and operational statistics
pub async fn dashboard_data(
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
        l2_head_block,
        l1_head_block,
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
        state.client.get_last_l1_block_number()
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get dashboard data");
        ErrorResponse::database_error()
    })?;

    let preconf_data = preconf.map(|d| PreconfDataResponse {
        candidates: d.candidates.into_iter().map(|a| format!("0x{}", encode(a))).collect(),
        current_operator: d.current_operator.map(|a| format!("0x{}", encode(a))),
        next_operator: d.next_operator.map(|a| format!("0x{}", encode(a))),
    });

    tracing::info!(
        l2_head_block,
        l1_head_block,
        reorgs = reorgs.len(),
        slashings = slashings.len(),
        forced_inclusions = forced_inclusions.len(),
        "Returning dashboard data"
    );

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
        l2_head_block,
        l1_head_block,
    }))
}

#[utoipa::path(
    get,
    path = "/blobs-per-batch/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated blobs per batch", body = AvgBatchBlobsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated blobs per batch with automatic bucketing based on time range
pub async fn blobs_per_batch_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<AvgBatchBlobsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
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

    let bucket = blobs_bucket_size(&time_range);
    let batches = aggregate_blobs_per_batch(batches, bucket);
    tracing::info!(count = batches.len(), "Returning aggregated blobs per batch");
    Ok(Json(AvgBatchBlobsResponse { batches }))
}

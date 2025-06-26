//! Aggregated data endpoints with complex processing

use crate::{
    helpers::{
        aggregate_batch_fee_components, aggregate_block_transactions, aggregate_l2_block_times,
        aggregate_l2_fee_components, aggregate_l2_gas_used, bucket_size_from_range,
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
use clickhouse_lib::AddressBytes;
use hex::encode;
use primitives::hardware::TOTAL_HARDWARE_COST_USD;

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
    let blocks = match state.client.get_l2_block_times(address, time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block times");
            return Err(ErrorResponse::database_error());
        }
    };
    let bucket = bucket_size_from_range(&time_range);
    let blocks = aggregate_l2_block_times(blocks, bucket);
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
/// Get aggregated block transaction counts with automatic bucketing based on time range
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
    path = "/l2-fees",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Priority and base fees", body = L2FeesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get L2 fee breakdown including priority fees, base fees, and L1 data costs by sequencer
pub async fn l2_fees(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2FeesResponse>, ErrorResponse> {
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

    let (priority_fee, base_fee, l1_data_cost, rows) = tokio::try_join!(
        state.client.get_l2_priority_fee(address, time_range),
        state.client.get_l2_base_fee(address, time_range),
        state.client.get_l1_total_data_cost(address, time_range),
        state.client.get_l2_fees_by_sequencer(time_range)
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get L2 fees");
        ErrorResponse::database_error()
    })?;

    // Filter using the raw `AddressBytes` value to avoid discrepancies caused by
    // different textual representations of the same address (e.g. case, missing
    // "0x" prefix). Only after filtering do we convert addresses to their
    // canonical hex string form.
    let sequencers: Vec<SequencerFeeRow> = rows
        .into_iter()
        .filter(|r| if let Some(target) = address { r.sequencer == target } else { true })
        .map(|r| SequencerFeeRow {
            address: format!("0x{}", encode(r.sequencer)),
            priority_fee: r.priority_fee,
            base_fee: r.base_fee,
            l1_data_cost: r.l1_data_cost,
        })
        .collect();

    tracing::info!(count = sequencers.len(), "Returning L2 fees and breakdown");
    Ok(Json(L2FeesResponse { priority_fee, base_fee, l1_data_cost, sequencers }))
}

#[utoipa::path(
    get,
    path = "/batch-fees",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Priority and base fees per batch", body = L2FeesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get batch fee breakdown including priority fees, base fees, and L1 data costs by proposer
pub async fn batch_fees(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2FeesResponse>, ErrorResponse> {
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

    let (priority_fee, base_fee, l1_data_cost, rows) = tokio::try_join!(
        state.client.get_batch_priority_fee(address, time_range),
        state.client.get_batch_base_fee(address, time_range),
        state.client.get_batch_total_data_cost(address, time_range),
        state.client.get_batch_fees_by_proposer(time_range)
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get batch fees");
        ErrorResponse::database_error()
    })?;

    let sequencers: Vec<SequencerFeeRow> = rows
        .into_iter()
        .filter(|r| if let Some(target) = address { r.sequencer == target } else { true })
        .map(|r| SequencerFeeRow {
            address: format!("0x{}", encode(r.sequencer)),
            priority_fee: r.priority_fee,
            base_fee: r.base_fee,
            l1_data_cost: r.l1_data_cost,
        })
        .collect();

    tracing::info!(count = sequencers.len(), "Returning batch fees and breakdown");
    Ok(Json(L2FeesResponse { priority_fee, base_fee, l1_data_cost, sequencers }))
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
        .map(|(addr, cost)| ProposerCostItem { address: format!("0x{}", encode(addr)), cost })
        .collect();

    tracing::info!(count = proposers.len(), "Returning prover costs");
    Ok(Json(ProposerCostsResponse { proposers }))
}

#[utoipa::path(
    get,
    path = "/verify-costs",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated verifier costs", body = ProposerCostsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated verifier costs grouped by proposer
pub async fn verify_costs(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ProposerCostsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;

    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);

    let rows = state.client.get_verify_costs_by_proposer(time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get verifier costs");
        ErrorResponse::database_error()
    })?;

    let proposers: Vec<ProposerCostItem> = rows
        .into_iter()
        .map(|(addr, cost)| ProposerCostItem { address: format!("0x{}", encode(addr)), cost })
        .collect();

    tracing::info!(count = proposers.len(), "Returning verifier costs");
    Ok(Json(ProposerCostsResponse { proposers }))
}

#[utoipa::path(
    get,
    path = "/l2-fee-components",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Fee components per block", body = FeeComponentsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get detailed fee components per block showing priority fee, base fee, and L1 data cost
pub async fn l2_fee_components(
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

    Ok(Json(FeeComponentsResponse { blocks }))
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

    Ok(Json(FeeComponentsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/batch-fee-components",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Fee components per batch", body = BatchFeeComponentsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get detailed fee components per batch showing priority fee, base fee, and L1 data cost
pub async fn batch_fee_components(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchFeeComponentsResponse>, ErrorResponse> {
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

    let rows = state.client.get_batch_fee_components(address, time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get batch fee components");
        ErrorResponse::database_error()
    })?;

    let prove_total =
        state.client.get_total_prove_cost(address, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get prove cost");
            ErrorResponse::database_error()
        })?;
    let verify_total =
        state.client.get_total_verify_cost(address, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get verify cost");
            ErrorResponse::database_error()
        })?;
    let count = rows.len() as u128;
    let amortized_prove = prove_total.and_then(|c| (count > 0).then_some(c / count));
    let amortized_verify = verify_total.and_then(|c| (count > 0).then_some(c / count));

    let batches: Vec<BatchFeeComponentRow> = rows
        .into_iter()
        .map(|r| BatchFeeComponentRow {
            batch_id: r.batch_id,
            l1_block_number: r.l1_block_number,
            sequencer: format!("0x{}", encode(r.sequencer)),
            priority_fee: r.priority_fee,
            base_fee: r.base_fee,
            l1_data_cost: r.l1_data_cost,
            amortized_prove_cost: amortized_prove,
            amortized_verify_cost: amortized_verify,
        })
        .collect();

    Ok(Json(BatchFeeComponentsResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/batch-fee-components/aggregated",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Aggregated batch fee components", body = BatchFeeComponentsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated fee components per batch with automatic bucketing based on time range
pub async fn batch_fee_components_aggregated(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchFeeComponentsResponse>, ErrorResponse> {
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

    let rows = state.client.get_batch_fee_components(address, time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get batch fee components");
        ErrorResponse::database_error()
    })?;

    let prove_total =
        state.client.get_total_prove_cost(address, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get prove cost");
            ErrorResponse::database_error()
        })?;
    let verify_total =
        state.client.get_total_verify_cost(address, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get verify cost");
            ErrorResponse::database_error()
        })?;
    let count = rows.len() as u128;
    let amortized_prove = prove_total.and_then(|c| (count > 0).then_some(c / count));
    let amortized_verify = verify_total.and_then(|c| (count > 0).then_some(c / count));

    let batches: Vec<BatchFeeComponentRow> = rows
        .into_iter()
        .map(|r| BatchFeeComponentRow {
            batch_id: r.batch_id,
            l1_block_number: r.l1_block_number,
            sequencer: format!("0x{}", encode(r.sequencer)),
            priority_fee: r.priority_fee,
            base_fee: r.base_fee,
            l1_data_cost: r.l1_data_cost,
            amortized_prove_cost: amortized_prove,
            amortized_verify_cost: amortized_verify,
        })
        .collect();

    let bucket = bucket_size_from_range(&time_range);
    let batches = aggregate_batch_fee_components(batches, bucket);

    Ok(Json(BatchFeeComponentsResponse { batches }))
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
        priority_fee,
        base_fee,
        prove_cost,
        verify_cost,
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
        state.client.get_l2_priority_fee(address, time_range),
        state.client.get_l2_base_fee(address, time_range),
        state.client.get_total_prove_cost(address, time_range),
        state.client.get_total_verify_cost(address, time_range)
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

    let hours = time_range.seconds() as f64 / 3600.0;
    let hourly_rate = TOTAL_HARDWARE_COST_USD / (30.0 * 24.0);
    let cost = hourly_rate * hours;

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
        priority_fee,
        base_fee,
        prove_cost,
        verify_cost,
        cloud_cost: Some(cost),
    }))
}

//! Paginated table endpoints

use crate::{
    state::{ApiState, MAX_TABLE_LIMIT},
    validation::{
        PaginatedQuery, has_time_range_params, resolve_time_range_since, validate_pagination,
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

// Legacy type aliases for backward compatibility
type BlockTransactionsQuery = PaginatedQuery;

#[utoipa::path(
    get,
    path = "/reorgs",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Reorg events", body = ReorgEventsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated list of L2 blockchain reorganization events
pub async fn reorgs(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ReorgEventsResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
    let events = match state
        .client
        .get_l2_reorgs_paginated(since, limit, params.starting_after, params.ending_before)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get reorg events");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = events.len(), "Returning reorg events");
    Ok(Json(ReorgEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/l2-tps",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "L2 TPS", body = L2TpsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 transactions per second data
pub async fn l2_tps(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2TpsResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
    let address = if let Some(addr) = params.common.address.as_ref() {
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
    let blocks = match state
        .client
        .get_l2_tps_paginated(since, limit, params.starting_after, params.ending_before, address)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to get L2 TPS: {}", e);
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = blocks.len(), "Returning L2 TPS");
    Ok(Json(L2TpsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/l2-block-times",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Paginated L2 block times", body = L2BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 block timing information
pub async fn l2_block_times(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2BlockTimesResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
    let address = if let Some(addr) = params.common.address.as_ref() {
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
        .get_l2_block_times_paginated(
            since,
            limit,
            params.starting_after,
            params.ending_before,
            address,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get L2 block times");
            return Err(ErrorResponse::database_error());
        }
    };

    tracing::info!(count = rows.len(), "Returning table L2 block times");
    Ok(Json(L2BlockTimesResponse { blocks: rows }))
}

#[utoipa::path(
    get,
    path = "/l2-gas-used",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Paginated L2 gas used", body = L2GasUsedResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 gas usage information per block
pub async fn l2_gas_used(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2GasUsedResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
    let address = if let Some(addr) = params.common.address.as_ref() {
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
        .get_l2_gas_used_paginated(
            since,
            limit,
            params.starting_after,
            params.ending_before,
            address,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get L2 gas used: {}", e);
            return Err(ErrorResponse::database_error());
        }
    };

    tracing::info!(count = rows.len(), "Returning table L2 gas used");
    Ok(Json(L2GasUsedResponse { blocks: rows }))
}

#[utoipa::path(
    get,
    path = "/block-transactions",
    params(
        BlockTransactionsQuery
    ),
    responses(
        (status = 200, description = "Paginated block transactions", body = BlockTransactionsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated transaction count information per block with sequencer details
pub async fn block_transactions(
    Query(params): Query<BlockTransactionsQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BlockTransactionsResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
    let address = if let Some(addr) = params.common.address.as_ref() {
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
        .get_block_transactions_paginated(
            since,
            limit,
            params.starting_after,
            params.ending_before,
            address,
        )
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

    tracing::info!(count = blocks.len(), "Returning table block transactions");
    Ok(Json(BlockTransactionsResponse { blocks }))
}

#[utoipa::path(
    get,
    path = "/blobs-per-batch",
    params(
        PaginatedQuery
    ),
    responses(
        (status = 200, description = "Blobs per batch", body = BatchBlobsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated blob count information for each batch
pub async fn blobs_per_batch(
    Query(params): Query<PaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchBlobsResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
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
        .get_blobs_per_batch_paginated(since, limit, params.starting_after, params.ending_before)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get blobs per batch");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = batches.len(), "Returning blobs per batch");
    Ok(Json(BatchBlobsResponse { batches }))
}

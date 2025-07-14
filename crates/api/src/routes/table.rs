//! Paginated table endpoints

use crate::{
    state::{ApiState, MAX_TABLE_LIMIT},
    validation::{
        BlockPaginatedQuery, PaginatedQuery, has_block_range_params, has_time_range_params,
        resolve_time_range_bounds, resolve_time_range_since, validate_block_range,
        validate_pagination, validate_range_exclusivity, validate_time_range,
    },
};
use api_types::*;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use hex::encode;

// Legacy type aliases for backward compatibility
type BlockTransactionsQuery = BlockPaginatedQuery;

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
/// Get paginated list of L2 blockchain reorganization events.
///
/// Results are ordered by insertion time in descending order.
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

    let (since, until) = resolve_time_range_bounds(&params.common.range, &params.common.time_range);
    let rows = match state
        .client
        .get_l2_reorgs_paginated(since, until, limit, params.starting_after, params.ending_before)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get reorg events");
            return Err(ErrorResponse::database_error());
        }
    };
    let events: Vec<L2ReorgEvent> = rows
        .into_iter()
        .map(|e| L2ReorgEvent {
            l2_block_number: e.l2_block_number,
            depth: e.depth,
            old_sequencer: format!("0x{}", encode(e.old_sequencer)),
            new_sequencer: format!("0x{}", encode(e.new_sequencer)),
            inserted_at: e.inserted_at,
        })
        .collect();
    tracing::info!(count = events.len(), "Returning reorg events");
    Ok(Json(ReorgEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/l2-tps",
    params(
        BlockPaginatedQuery
    ),
    responses(
        (status = 200, description = "L2 TPS", body = L2TpsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 transactions per second data.
///
/// Results are ordered by block number in descending order.
pub async fn l2_tps(
    Query(params): Query<BlockPaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2TpsResponse>, ErrorResponse> {
    validate_block_range(&params.block_range)?;
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_block_range = has_block_range_params(&params.block_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_block_range, has_slot_range)?;

    let start_block = if let Some(gt) = params.block_range.block_gt {
        Some(gt.checked_add(1).ok_or_else(|| {
            ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "block[gt] value is too large",
            )
        })?)
    } else {
        params.block_range.block_gte
    };
    let end_block = params.block_range.block_lt.or(params.block_range.block_lte);

    let blocks = match state
        .client
        .get_l2_tps_block_range(
            None,
            start_block,
            end_block,
            limit,
            params.starting_after,
            params.ending_before,
        )
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
        BlockPaginatedQuery
    ),
    responses(
        (status = 200, description = "Paginated L2 block times", body = L2BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 block timing information.
///
/// Results are ordered by block number in descending order.
pub async fn l2_block_times(
    Query(params): Query<BlockPaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2BlockTimesResponse>, ErrorResponse> {
    validate_block_range(&params.block_range)?;
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_block_range = has_block_range_params(&params.block_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_block_range, has_slot_range)?;

    let start_block = if let Some(gt) = params.block_range.block_gt {
        Some(gt.checked_add(1).ok_or_else(|| {
            ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "block[gt] value is too large",
            )
        })?)
    } else {
        params.block_range.block_gte
    };
    let end_block = params.block_range.block_lt.or(params.block_range.block_lte);

    let rows = match state
        .client
        .get_l2_block_times_block_range(
            None,
            start_block,
            end_block,
            limit,
            params.starting_after,
            params.ending_before,
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
        BlockPaginatedQuery
    ),
    responses(
        (status = 200, description = "Paginated L2 gas used", body = L2GasUsedResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get paginated L2 gas usage information per block.
///
/// Results are ordered by block number in descending order.
pub async fn l2_gas_used(
    Query(params): Query<BlockPaginatedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2GasUsedResponse>, ErrorResponse> {
    validate_block_range(&params.block_range)?;
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_block_range = has_block_range_params(&params.block_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_block_range, has_slot_range)?;

    let start_block = if let Some(gt) = params.block_range.block_gt {
        Some(gt.checked_add(1).ok_or_else(|| {
            ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "block[gt] value is too large",
            )
        })?)
    } else {
        params.block_range.block_gte
    };
    let end_block = params.block_range.block_lt.or(params.block_range.block_lte);

    let rows = match state
        .client
        .get_l2_gas_used_block_range(
            None,
            start_block,
            end_block,
            limit,
            params.starting_after,
            params.ending_before,
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
/// Get paginated transaction count information per block with sequencer details.
///
/// Results are ordered by block number in descending order.
pub async fn block_transactions(
    Query(params): Query<BlockTransactionsQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BlockTransactionsResponse>, ErrorResponse> {
    validate_block_range(&params.block_range)?;
    let limit = validate_pagination(
        params.starting_after.as_ref(),
        params.ending_before.as_ref(),
        params.limit.as_ref(),
        MAX_TABLE_LIMIT,
    )?;
    let has_block_range = has_block_range_params(&params.block_range);
    let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
    validate_range_exclusivity(has_block_range, has_slot_range)?;

    let start_block = if let Some(gt) = params.block_range.block_gt {
        Some(gt.checked_add(1).ok_or_else(|| {
            ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "block[gt] value is too large",
            )
        })?)
    } else {
        params.block_range.block_gte
    };
    let end_block = params.block_range.block_lt.or(params.block_range.block_lte);

    let rows = match state
        .client
        .get_block_transactions_block_range(
            start_block,
            end_block,
            None,
            limit,
            params.starting_after,
            params.ending_before,
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
            block_number: r.l2_block_number,
            txs: r.sum_tx,
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
/// Get paginated blob count information for each batch.
///
/// Results are ordered by batch id in descending order.
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

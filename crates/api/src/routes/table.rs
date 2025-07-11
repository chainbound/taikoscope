//! Paginated table endpoints

use crate::{
    helpers::bucket_size_from_range,
    state::{ApiState, MAX_TABLE_LIMIT},
    validation::{
        CommonQuery, PaginatedQuery, QueryMode, UnifiedQuery, has_time_range_params,
        resolve_time_range_bounds, resolve_time_range_enum, resolve_time_range_since,
        validate_pagination, validate_range_exclusivity, validate_time_range,
        validate_unified_query,
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
// type BlockTransactionsQuery = BlockPaginatedQuery; // Removed - not used anymore
type RangeQuery = CommonQuery;

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

    let (since, until) = resolve_time_range_bounds(&params.common.time_range);
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
        .map(|e| {
            let from_block_number = e.l2_block_number + u64::from(e.depth);
            L2ReorgEvent {
                from_block_number,
                to_block_number: e.l2_block_number,
                depth: e.depth,
                old_sequencer: format!("0x{}", encode(e.old_sequencer)),
                new_sequencer: format!("0x{}", encode(e.new_sequencer)),
                inserted_at: e.inserted_at,
            }
        })
        .collect();
    tracing::info!(count = events.len(), "Returning reorg events");
    Ok(Json(ReorgEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/slashings",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Slashing events", body = SlashingEventsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get validator slashing events within the requested time range.
pub async fn slashings(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<SlashingEventsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let (since, until) = resolve_time_range_bounds(&params.time_range);
    let events = match state.client.get_slashing_events_range(since, until).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get slashing events");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = events.len(), "Returning slashing events");
    Ok(Json(SlashingEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/forced-inclusions",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Forced inclusion events", body = ForcedInclusionEventsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get forced inclusion events within the requested time range.
pub async fn forced_inclusions(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ForcedInclusionEventsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let (since, until) = resolve_time_range_bounds(&params.time_range);
    let events = match state.client.get_forced_inclusions_range(since, until).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get forced inclusion events");
            return Err(ErrorResponse::database_error());
        }
    };
    tracing::info!(count = events.len(), "Returning forced inclusion events");
    Ok(Json(ForcedInclusionEventsResponse { events }))
}

#[utoipa::path(
    get,
    path = "/l2-tps",
    params(
        UnifiedQuery
    ),
    responses(
        (status = 200, description = "L2 TPS (regular or aggregated)", body = L2TpsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get L2 transactions per second data.
///
/// Use ?aggregated for aggregated data with automatic bucketing based on time range.
/// Without ?aggregated, returns paginated results ordered by block number in descending order.
#[allow(clippy::cognitive_complexity)]
pub async fn l2_tps(
    Query(params): Query<UnifiedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2TpsResponse>, ErrorResponse> {
    let query_mode = validate_unified_query(&params, MAX_TABLE_LIMIT)?;

    match query_mode {
        QueryMode::Aggregated => {
            // Aggregated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            validate_range_exclusivity(has_time_range, false)?;

            let time_range = resolve_time_range_enum(&params.common.time_range);
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
            let bucket = bucket_size_from_range(&time_range);
            let blocks = match state.client.get_l2_tps(address, time_range, Some(bucket)).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get L2 TPS");
                    return Err(ErrorResponse::database_error());
                }
            };
            tracing::info!(count = blocks.len(), "Returning aggregated L2 TPS");
            Ok(Json(L2TpsResponse { blocks }))
        }
        QueryMode::Regular { limit } => {
            // Regular paginated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
            validate_range_exclusivity(has_time_range, has_slot_range)?;

            let since = resolve_time_range_since(&params.common.time_range);
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
                .get_l2_tps_paginated(
                    since,
                    limit,
                    params.starting_after,
                    params.ending_before,
                    address,
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("Failed to get L2 TPS: {}", e);
                    return Err(ErrorResponse::database_error());
                }
            };

            tracing::info!(count = blocks.len(), "Returning paginated L2 TPS");
            Ok(Json(L2TpsResponse { blocks }))
        }
    }
}

#[utoipa::path(
    get,
    path = "/l2-block-times",
    params(
        UnifiedQuery
    ),
    responses(
        (status = 200, description = "L2 block times (regular or aggregated)", body = L2BlockTimesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get L2 block timing information.
///
/// Use ?aggregated for aggregated data with automatic bucketing based on time range.
/// Without ?aggregated, returns paginated results ordered by block number in descending order.
#[allow(clippy::cognitive_complexity)]
pub async fn l2_block_times(
    Query(params): Query<UnifiedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2BlockTimesResponse>, ErrorResponse> {
    let query_mode = validate_unified_query(&params, MAX_TABLE_LIMIT)?;

    match query_mode {
        QueryMode::Aggregated => {
            // Aggregated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            validate_range_exclusivity(has_time_range, false)?;

            let time_range = resolve_time_range_enum(&params.common.time_range);
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
            let bucket = bucket_size_from_range(&time_range);
            let blocks =
                match state.client.get_l2_block_times(address, time_range, Some(bucket)).await {
                    Ok(rows) => rows,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to get L2 block times");
                        return Err(ErrorResponse::database_error());
                    }
                };
            tracing::info!(count = blocks.len(), "Returning aggregated L2 block times");
            Ok(Json(L2BlockTimesResponse { blocks }))
        }
        QueryMode::Regular { limit } => {
            // Regular paginated mode - use block range parameters
            // For regular mode, we need to support both time-based and block-based queries
            // For now, we'll use time-based queries (like the original table endpoint)
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
            validate_range_exclusivity(has_time_range, has_slot_range)?;

            let (since, _until) = resolve_time_range_bounds(&params.common.time_range);
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

            tracing::info!(count = rows.len(), "Returning paginated L2 block times");
            Ok(Json(L2BlockTimesResponse { blocks: rows }))
        }
    }
}

#[utoipa::path(
    get,
    path = "/l2-gas-used",
    params(
        UnifiedQuery
    ),
    responses(
        (status = 200, description = "L2 gas used (regular or aggregated)", body = L2GasUsedResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get L2 gas usage information per block.
///
/// Use ?aggregated for aggregated data with automatic bucketing based on time range.
/// Without ?aggregated, returns paginated results ordered by block number in descending order.
#[allow(clippy::cognitive_complexity)]
pub async fn l2_gas_used(
    Query(params): Query<UnifiedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<L2GasUsedResponse>, ErrorResponse> {
    let query_mode = validate_unified_query(&params, MAX_TABLE_LIMIT)?;

    match query_mode {
        QueryMode::Aggregated => {
            // Aggregated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            validate_range_exclusivity(has_time_range, false)?;

            let time_range = resolve_time_range_enum(&params.common.time_range);
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
            let bucket = bucket_size_from_range(&time_range);
            let blocks = match state.client.get_l2_gas_used(address, time_range, Some(bucket)).await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("Failed to get L2 gas used: {}", e);
                    return Err(ErrorResponse::database_error());
                }
            };
            tracing::info!(count = blocks.len(), "Returning aggregated L2 gas used");
            Ok(Json(L2GasUsedResponse { blocks }))
        }
        QueryMode::Regular { limit } => {
            // Regular paginated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
            validate_range_exclusivity(has_time_range, has_slot_range)?;

            let since = resolve_time_range_since(&params.common.time_range);
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

            tracing::info!(count = rows.len(), "Returning paginated L2 gas used");
            Ok(Json(L2GasUsedResponse { blocks: rows }))
        }
    }
}

#[utoipa::path(
    get,
    path = "/block-transactions",
    params(
        UnifiedQuery
    ),
    responses(
        (status = 200, description = "Block transactions (regular or aggregated)", body = BlockTransactionsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get transaction count information per block with sequencer details.
///
/// Use ?aggregated for aggregated data with automatic bucketing based on time range.
/// Without ?aggregated, returns paginated results ordered by block number in descending order.
#[allow(clippy::cognitive_complexity)]
pub async fn block_transactions(
    Query(params): Query<UnifiedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BlockTransactionsResponse>, ErrorResponse> {
    let query_mode = validate_unified_query(&params, MAX_TABLE_LIMIT)?;

    match query_mode {
        QueryMode::Aggregated => {
            // Aggregated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            validate_range_exclusivity(has_time_range, false)?;

            let time_range = resolve_time_range_enum(&params.common.time_range);
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
            let bucket = bucket_size_from_range(&time_range);
            let rows = match state
                .client
                .get_block_transactions(address, time_range, Some(bucket))
                .await
            {
                Ok(rows) => rows,
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

            tracing::info!(count = blocks.len(), "Returning aggregated block transactions");
            Ok(Json(BlockTransactionsResponse { blocks }))
        }
        QueryMode::Regular { limit } => {
            // Regular paginated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
            validate_range_exclusivity(has_time_range, has_slot_range)?;

            let since = resolve_time_range_since(&params.common.time_range);
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
                    None, // No bucketing for regular mode
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

            tracing::info!(count = blocks.len(), "Returning paginated block transactions");
            Ok(Json(BlockTransactionsResponse { blocks }))
        }
    }
}

#[utoipa::path(
    get,
    path = "/blobs-per-batch",
    params(
        UnifiedQuery
    ),
    responses(
        (status = 200, description = "Blobs per batch (regular or aggregated)", body = BatchBlobsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get blob count information for each batch.
///
/// Use ?aggregated for aggregated data based on time range.
/// Without ?aggregated, returns paginated results ordered by batch id in descending order.
#[allow(clippy::cognitive_complexity)]
pub async fn blobs_per_batch(
    Query(params): Query<UnifiedQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchBlobsResponse>, ErrorResponse> {
    let query_mode = validate_unified_query(&params, MAX_TABLE_LIMIT)?;

    match query_mode {
        QueryMode::Aggregated => {
            // Aggregated mode - use time range parameters
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            validate_range_exclusivity(has_time_range, false)?;

            let time_range = resolve_time_range_enum(&params.common.time_range);
            let batches = match state.client.get_blobs_per_batch(time_range).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get blobs per batch");
                    return Err(ErrorResponse::database_error());
                }
            };
            tracing::info!(count = batches.len(), "Returning aggregated blobs per batch");
            Ok(Json(BatchBlobsResponse { batches }))
        }
        QueryMode::Regular { limit } => {
            // Regular paginated mode
            validate_time_range(&params.common.time_range)?;
            let has_time_range = has_time_range_params(&params.common.time_range);
            let has_slot_range = params.starting_after.is_some() || params.ending_before.is_some();
            validate_range_exclusivity(has_time_range, has_slot_range)?;

            let since = resolve_time_range_since(&params.common.time_range);
            let batches = match state
                .client
                .get_blobs_per_batch_paginated(
                    since,
                    limit,
                    params.starting_after,
                    params.ending_before,
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get blobs per batch");
                    return Err(ErrorResponse::database_error());
                }
            };
            tracing::info!(count = batches.len(), "Returning paginated blobs per batch");
            Ok(Json(BatchBlobsResponse { batches }))
        }
    }
}

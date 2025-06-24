use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use tracing;

use crate::{
    state::{ApiState, MAX_TABLE_LIMIT},
    validation::{
        CommonQuery, ProfitQuery, has_time_range_params, resolve_time_range_enum,
        validate_range_exclusivity, validate_time_range,
    },
};
use alloy_primitives::Address;
use api_types::{
    BatchDashboardDataResponse, BatchEconomicsResponse, BatchL2FeesResponse,
    BatchProfitRankingResponse, ErrorResponse,
};
use clickhouse_lib::AddressBytes;
use hex::encode;

type RangeQuery = CommonQuery;

#[utoipa::path(
    get,
    path = "/batch-economics",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Batch economics data", body = BatchEconomicsResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get economics data aggregated per batch for the specified time range
pub async fn get_batch_economics(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchEconomicsResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);
    let rows = match state.client.get_batch_economics(time_range).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get batch economics");
            return Err(ErrorResponse::database_error());
        }
    };

    // Convert clickhouse types to api types
    let batches: Vec<api_types::BatchEconomicsItem> = rows
        .into_iter()
        .map(|r| api_types::BatchEconomicsItem {
            batch_id: r.batch_id,
            l1_block_number: r.l1_block_number,
            batch_size: r.batch_size,
            last_l2_block_number: r.last_l2_block_number,
            first_l2_block_number: r.first_l2_block_number,
            proposer_addr: r.proposer_addr,
            total_priority_fee: r.total_priority_fee,
            total_base_fee: r.total_base_fee,
            total_l1_data_cost: r.total_l1_data_cost,
            net_profit: r.net_profit,
            total_transactions: r.total_transactions,
            total_gas_used: r.total_gas_used,
            proposed_at: r.proposed_at,
        })
        .collect();

    tracing::info!(count = batches.len(), "Returning batch economics");
    Ok(Json(BatchEconomicsResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/batch-profit-ranking",
    params(
        ProfitQuery
    ),
    responses(
        (status = 200, description = "Batch profit ranking", body = BatchProfitRankingResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get the most or least profitable batches in the specified range
pub async fn get_batch_profit_ranking(
    Query(params): Query<ProfitQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchProfitRankingResponse>, ErrorResponse> {
    validate_time_range(&params.common.time_range)?;
    let limit = params.limit.unwrap_or(5).min(MAX_TABLE_LIMIT);
    let order_desc =
        params.order.as_deref().map(|o| o.eq_ignore_ascii_case("desc")).unwrap_or(true);
    let has_time_range = has_time_range_params(&params.common.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.common.range, &params.common.time_range);
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

    let rows = state
        .client
        .get_batch_profit_ranking(time_range, limit, order_desc, address)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get batch profit ranking");
            ErrorResponse::database_error()
        })?;

    // Convert clickhouse types to api types
    let batches: Vec<api_types::BatchProfitItem> = rows
        .into_iter()
        .map(|r| api_types::BatchProfitItem {
            batch_id: r.batch_id,
            net_profit: r.net_profit,
            l1_block_number: r.l1_block_number,
            first_l2_block_number: r.first_l2_block_number,
            last_l2_block_number: r.last_l2_block_number,
            proposer_addr: r.proposer_addr,
        })
        .collect();

    tracing::info!(count = batches.len(), "Returning batch profit ranking");
    Ok(Json(BatchProfitRankingResponse { batches }))
}

#[utoipa::path(
    get,
    path = "/batch-l2-fees",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Batch-level L2 fees", body = BatchL2FeesResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated L2 fees per batch for the specified range
pub async fn get_batch_l2_fees(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchL2FeesResponse>, ErrorResponse> {
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

    let (priority_fee, base_fee, l1_data_cost) =
        state.client.get_batch_l2_fees_totals(address, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get batch L2 fees totals");
            ErrorResponse::database_error()
        })?;

    let seq_rows = state.client.get_batch_l2_fees_by_sequencer(time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get batch L2 fees by sequencer");
        ErrorResponse::database_error()
    })?;

    // Convert clickhouse types to api types
    let sequencers: Vec<api_types::BatchSequencerFeeRow> = seq_rows
        .into_iter()
        .map(|r| api_types::BatchSequencerFeeRow {
            address: r.address,
            priority_fee: r.priority_fee,
            base_fee: r.base_fee,
            l1_data_cost: r.l1_data_cost,
            batch_count: r.batch_count,
        })
        .collect();

    tracing::info!(
        priority_fee = ?priority_fee,
        base_fee = ?base_fee,
        l1_data_cost = ?l1_data_cost,
        sequencers_count = sequencers.len(),
        "Returning batch L2 fees"
    );
    Ok(Json(BatchL2FeesResponse { priority_fee, base_fee, l1_data_cost, sequencers }))
}

#[utoipa::path(
    get,
    path = "/batch-dashboard-data",
    params(
        RangeQuery
    ),
    responses(
        (status = 200, description = "Batch-level dashboard data", body = BatchDashboardDataResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "taikoscope"
)]
/// Get aggregated dashboard data with batch-level metrics
pub async fn get_batch_dashboard_data(
    Query(params): Query<RangeQuery>,
    State(state): State<ApiState>,
) -> Result<Json<BatchDashboardDataResponse>, ErrorResponse> {
    validate_time_range(&params.time_range)?;
    let has_time_range = has_time_range_params(&params.time_range);
    validate_range_exclusivity(has_time_range, false)?;

    let time_range = resolve_time_range_enum(&params.range, &params.time_range);

    // Get existing dashboard metrics (reusing existing endpoints)
    let l2_block_cadence_ms =
        state.client.get_l2_block_cadence(None, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get L2 block cadence");
            ErrorResponse::database_error()
        })?;

    let batch_posting_cadence_ms =
        state.client.get_batch_posting_cadence(time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get batch posting cadence");
            ErrorResponse::database_error()
        })?;

    let avg_prove_time_ms = state.client.get_avg_prove_time(time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get avg prove time");
        ErrorResponse::database_error()
    })?;

    let avg_verify_time_ms = state.client.get_avg_verify_time(time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get avg verify time");
        ErrorResponse::database_error()
    })?;

    let avg_tps = state.client.get_avg_l2_tps(None, time_range).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get avg TPS");
        ErrorResponse::database_error()
    })?;

    let preconf_data_raw = state.client.get_last_preconf_data().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get preconf data");
        ErrorResponse::database_error()
    })?;

    // Convert to PreconfDataResponse
    let preconf_data = preconf_data_raw.map(|data| api_types::PreconfDataResponse {
        candidates: data.candidates.into_iter().map(|addr| format!("0x{}", encode(addr))).collect(),
        current_operator: data.current_operator.map(|addr| format!("0x{}", encode(addr))),
        next_operator: data.next_operator.map(|addr| format!("0x{}", encode(addr))),
    });

    let since = chrono::Utc::now() - chrono::Duration::seconds(time_range.seconds() as i64);
    let l2_reorgs = state
        .client
        .get_l2_reorgs_since(since)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get L2 reorgs");
            ErrorResponse::database_error()
        })?
        .len();

    let slashings = state
        .client
        .get_slashing_events_since(since)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get slashing events");
            ErrorResponse::database_error()
        })?
        .len();

    let forced_inclusions = state
        .client
        .get_forced_inclusions_since(since)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get forced inclusions");
            ErrorResponse::database_error()
        })?
        .len();

    let l2_block = state.client.get_last_l2_block_number().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get last L2 block");
        ErrorResponse::database_error()
    })?;

    let l1_block = state.client.get_last_l1_block_number().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get last L1 block");
        ErrorResponse::database_error()
    })?;

    // Get batch-level fees
    let (priority_fee, base_fee, _l1_data_cost) =
        state.client.get_batch_l2_fees_totals(None, time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get batch L2 fees");
            ErrorResponse::database_error()
        })?;

    // Get batch-specific metrics
    let (total_batches, avg_blocks_per_batch) =
        state.client.get_batch_dashboard_metrics(time_range).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get batch dashboard metrics");
            ErrorResponse::database_error()
        })?;

    // Cloud cost estimation (placeholder - would need to be implemented based on requirements)
    let cloud_cost = None;

    tracing::info!(
        total_batches,
        avg_blocks_per_batch = ?avg_blocks_per_batch,
        "Returning batch dashboard data"
    );

    Ok(Json(BatchDashboardDataResponse {
        l2_block_cadence_ms,
        batch_posting_cadence_ms,
        avg_prove_time_ms,
        avg_verify_time_ms,
        avg_tps,
        preconf_data,
        l2_reorgs,
        slashings,
        forced_inclusions,
        l2_block,
        l1_block,
        priority_fee,
        base_fee,
        cloud_cost,
        total_batches,
        avg_blocks_per_batch,
    }))
}

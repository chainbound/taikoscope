//! API route definitions

pub mod aggregated;
pub mod core;
pub mod sse;
pub mod table;

use crate::{ApiDoc, state::ApiState};
use axum::{Router, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use aggregated::*;
use core::*;
use sse::*;
use table::*;

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
        .route("/l1-block-times", get(l1_block_times))
        .route("/l2-block-times", get(l2_block_times))
        .route("/l2-block-times/aggregated", get(l2_block_times_aggregated))
        .route("/l2-gas-used", get(l2_gas_used))
        .route("/l2-gas-used/aggregated", get(l2_gas_used_aggregated))
        .route("/l2-tps", get(l2_tps))
        .route("/l2-tps/aggregated", get(l2_tps_aggregated))
        .route("/sequencer-distribution", get(sequencer_distribution))
        .route("/sequencer-blocks", get(sequencer_blocks))
        .route("/block-transactions", get(block_transactions))
        .route("/block-transactions/aggregated", get(block_transactions_aggregated))
        .route("/l2-fees", get(l2_fees))
        .route("/batch-fees", get(batch_fees))
        .route("/l2-fee-components", get(l2_fee_components))
        .route("/l2-fee-components/aggregated", get(l2_fee_components_aggregated))
        .route("/batch-fee-components", get(batch_fee_components))
        .route("/batch-fee-components/aggregated", get(batch_fee_components_aggregated))
        .route("/dashboard-data", get(dashboard_data))
        .route("/l1-data-cost", get(l1_data_cost))
        .route("/prove-costs", get(prove_costs))
        .route("/prove-cost", get(prove_cost))
        .route("/block-profits", get(block_profits))
        .route("/eth-price", get(eth_price));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(api_routes)
        .with_state(state)
}

//! Data types for the Taikoscope API.
//!
//! These structs define the JSON responses returned by the API server. They
//! are provided in a separate crate so that consumers such as the dashboard can
//! depend on them without pulling in the rest of the server implementation.

#![allow(missing_docs)]

use clickhouse_lib::{
    BatchBlobCountRow, BatchProveTimeRow, BatchVerifyTimeRow, ForcedInclusionProcessedRow,
    L1BlockTimeRow, L2BlockTimeRow, L2GasUsedRow, L2ReorgRow, SlashingEventRow,
};

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct L2HeadResponse {
    pub last_l2_head_time: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L1HeadResponse {
    pub last_l1_head_time: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SlashingEventsResponse {
    pub events: Vec<SlashingEventRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ForcedInclusionEventsResponse {
    pub events: Vec<ForcedInclusionProcessedRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReorgEventsResponse {
    pub events: Vec<L2ReorgRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveGatewaysResponse {
    pub gateways: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentOperatorResponse {
    pub operator: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NextOperatorResponse {
    pub operator: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AvgProveTimeResponse {
    pub avg_prove_time_ms: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AvgVerifyTimeResponse {
    pub avg_verify_time_ms: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L2BlockCadenceResponse {
    pub l2_block_cadence_ms: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchPostingCadenceResponse {
    pub batch_posting_cadence_ms: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AvgL2TpsResponse {
    pub avg_tps: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProveTimesResponse {
    pub batches: Vec<BatchProveTimeRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyTimesResponse {
    pub batches: Vec<BatchVerifyTimeRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L1BlockTimesResponse {
    pub blocks: Vec<L1BlockTimeRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L2BlockTimesResponse {
    pub blocks: Vec<L2BlockTimeRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L2GasUsedResponse {
    pub blocks: Vec<L2GasUsedRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerDistributionItem {
    pub address: String,
    pub blocks: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerDistributionResponse {
    pub sequencers: Vec<SequencerDistributionItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerBlocksItem {
    pub address: String,
    pub blocks: Vec<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerBlocksResponse {
    pub sequencers: Vec<SequencerBlocksItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BlockTransactionsItem {
    pub block: u64,
    pub txs: u32,
    pub sequencer: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BlockTransactionsResponse {
    pub blocks: Vec<BlockTransactionsItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchBlobsResponse {
    pub batches: Vec<BatchBlobCountRow>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AvgBlobsPerBatchResponse {
    pub avg_blobs: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L2HeadBlockResponse {
    pub l2_head_block: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct L1HeadBlockResponse {
    pub l1_head_block: Option<u64>,
}

//! Data types for the Taikoscope API.
//!
//! These structs define the JSON responses returned by the API server. They
//! are provided in a separate crate so that consumers such as the dashboard can
//! depend on them without pulling in the rest of the server implementation.

use clickhouse_lib::{
    BatchBlobCountRow, BatchProveTimeRow, BatchVerifyTimeRow, ForcedInclusionProcessedRow,
    L1BlockTimeRow, L2BlockTimeRow, L2GasUsedRow, L2ReorgRow, SlashingEventRow,
};

use serde::Serialize;
use utoipa::ToSchema;

/// Timestamp of the most recent L2 block.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L2HeadResponse {
    pub last_l2_head_time: Option<String>,
}

/// Timestamp of the most recent L1 block.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L1HeadResponse {
    pub last_l1_head_time: Option<String>,
}

/// List of validator slashing events.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct SlashingEventsResponse {
    pub events: Vec<SlashingEventRow>,
}

/// Forced inclusion events that were processed.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct ForcedInclusionEventsResponse {
    pub events: Vec<ForcedInclusionProcessedRow>,
}

/// Detected L2 reorg events.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct ReorgEventsResponse {
    pub events: Vec<L2ReorgRow>,
}

/// Gateways that submitted batches in the requested range.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveGatewaysResponse {
    pub gateways: Vec<String>,
}

/// Current operator address.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentOperatorResponse {
    pub operator: Option<String>,
}

/// Address of the next operator.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct NextOperatorResponse {
    pub operator: Option<String>,
}

/// Average time in milliseconds to prove a batch.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct AvgProveTimeResponse {
    pub avg_prove_time_ms: Option<u64>,
}

/// Average time in milliseconds to verify a batch.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct AvgVerifyTimeResponse {
    pub avg_verify_time_ms: Option<u64>,
}

/// Average delay between L2 blocks in milliseconds.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L2BlockCadenceResponse {
    pub l2_block_cadence_ms: Option<u64>,
}

/// Average delay between batch submissions.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchPostingCadenceResponse {
    pub batch_posting_cadence_ms: Option<u64>,
}

/// Average L2 transactions per second.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct AvgL2TpsResponse {
    pub avg_tps: Option<f64>,
}

/// Time to prove individual batches.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct ProveTimesResponse {
    pub batches: Vec<BatchProveTimeRow>,
}

/// Time to verify individual batches.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyTimesResponse {
    pub batches: Vec<BatchVerifyTimeRow>,
}

/// L1 block numbers grouped by minute.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L1BlockTimesResponse {
    pub blocks: Vec<L1BlockTimeRow>,
}

/// Timestamp data for L2 blocks.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L2BlockTimesResponse {
    pub blocks: Vec<L2BlockTimeRow>,
}

/// Gas usage for each L2 block.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L2GasUsedResponse {
    pub blocks: Vec<L2GasUsedRow>,
}

/// Number of blocks produced by a sequencer.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerDistributionItem {
    pub address: String,
    pub blocks: u64,
}

/// Distribution of blocks across sequencers.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerDistributionResponse {
    pub sequencers: Vec<SequencerDistributionItem>,
}

/// Blocks proposed by a sequencer.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerBlocksItem {
    pub address: String,
    pub blocks: Vec<u64>,
}

/// Mapping of sequencers to their blocks.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct SequencerBlocksResponse {
    pub sequencers: Vec<SequencerBlocksItem>,
}

/// Transaction count for a block and its sequencer.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BlockTransactionsItem {
    pub block: u64,
    pub txs: u32,
    pub sequencer: String,
}

/// Collection of block transaction counts.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BlockTransactionsResponse {
    pub blocks: Vec<BlockTransactionsItem>,
}

/// Blob count per batch.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchBlobsResponse {
    pub batches: Vec<BatchBlobCountRow>,
}

/// Average number of blobs per batch.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct AvgBlobsPerBatchResponse {
    pub avg_blobs: Option<f64>,
}

/// Number of the most recent L2 block.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L2HeadBlockResponse {
    pub l2_head_block: Option<u64>,
}

/// Number of the most recent L1 block.
#[allow(missing_docs)]
#[derive(Debug, Serialize, ToSchema)]
pub struct L1HeadBlockResponse {
    pub l1_head_block: Option<u64>,
}

use chrono::{DateTime, Utc};
use clickhouse::Row;
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::types::{AddressBytes, HashBytes};

/// L1 head event
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L1HeadEvent {
    /// L1 block number
    pub l1_block_number: u64,
    /// Block hash
    pub block_hash: HashBytes,
    /// Slot
    pub slot: u64,
    /// Block timestamp
    pub block_ts: u64,
}

/// Preconf data
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreconfData {
    /// Slot
    pub slot: u64,
    /// Candidates
    pub candidates: Vec<AddressBytes>,
    /// Current operator
    pub current_operator: Option<AddressBytes>,
    /// Next operator
    pub next_operator: Option<AddressBytes>,
}

/// L2 head event
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2HeadEvent {
    /// L2 block number
    pub l2_block_number: u64,
    /// Block hash
    pub block_hash: HashBytes,
    /// Block timestamp
    pub block_ts: u64,
    /// Sum of gas used in the block
    pub sum_gas_used: u128,
    /// Number of transactions
    pub sum_tx: u32,
    /// Sum of priority fees paid
    pub sum_priority_fee: u128,
    /// Sequencer sequencing the block
    pub sequencer: AddressBytes,
}

/// Batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Batch size
    pub batch_size: u16,
    /// Proposer address
    pub proposer_addr: AddressBytes,
    /// Blob count
    pub blob_count: u8,
    /// Blob total bytes
    pub blob_total_bytes: u32,
}

/// Proved batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvedBatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Verifier address
    pub verifier_addr: AddressBytes,
    /// Parent hash
    pub parent_hash: HashBytes,
    /// Block hash
    pub block_hash: HashBytes,
    /// State root
    pub state_root: HashBytes,
}

/// L2 reorg row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct L2ReorgRow {
    /// Block number
    pub l2_block_number: u64,
    /// Depth
    pub depth: u16,
}

/// Forced inclusion processed row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ForcedInclusionProcessedRow {
    /// Blob hash
    pub blob_hash: HashBytes,
}

/// Verified batch row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedBatchRow {
    /// L1 block number
    pub l1_block_number: u64,
    /// Batch ID
    pub batch_id: u64,
    /// Block hash
    pub block_hash: HashBytes,
}

/// Slashing event row
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SlashingEventRow {
    /// L1 block number where slashing occurred
    pub l1_block_number: u64,
    /// Address of the validator that was slashed
    pub validator_addr: AddressBytes,
}

/// Row representing the number of blocks produced by a sequencer
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct SequencerDistributionRow {
    /// Sequencer address
    pub sequencer: AddressBytes,
    /// Number of blocks produced by the sequencer
    pub blocks: u64,
}

/// Row representing a single block proposed by a sequencer
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct SequencerBlockRow {
    /// Sequencer address
    pub sequencer: AddressBytes,
    /// L2 block number proposed by the sequencer
    pub l2_block_number: u64,
}

/// Row representing the transaction count of a block and its sequencer
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockTransactionRow {
    /// Sequencer address
    pub sequencer: AddressBytes,
    /// L2 block number
    pub l2_block_number: u64,
    /// Number of transactions in the block
    pub sum_tx: u32,
}

/// Row representing the time it took for a batch to be proven
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct BatchProveTimeRow {
    /// Batch ID
    pub batch_id: u64,
    /// Seconds between proposal and proof
    pub seconds_to_prove: u64,
}

/// Row representing the time it took for a batch to be verified
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct BatchVerifyTimeRow {
    /// Batch ID
    pub batch_id: u64,
    /// Seconds between proof and verification
    pub seconds_to_verify: u64,
}

/// Row representing the block number seen at a given minute
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct L1BlockTimeRow {
    /// Minute timestamp (unix seconds)
    pub minute: u64,
    /// Highest L1 block number within that minute
    pub block_number: u64,
}

/// Row representing the time between consecutive L2 blocks
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct L2BlockTimeRow {
    /// L2 block number
    pub l2_block_number: u64,
    /// Timestamp of the L2 block
    pub block_time: DateTime<Utc>,
    /// Milliseconds since the previous block
    pub ms_since_prev_block: Option<u64>,
}

/// Row representing the gas used in each L2 block
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct L2GasUsedRow {
    /// L2 block number
    pub l2_block_number: u64,
    /// Total gas used in the block
    pub gas_used: u64,
}

/// Row representing the blob count for each batch
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct BatchBlobCountRow {
    /// Batch ID
    pub batch_id: u64,
    /// Number of blobs in the batch
    pub blob_count: u8,
}

/// Row representing the interval between consecutive batch proposals
#[derive(Debug, Row, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct BatchPostingTimeRow {
    /// Batch ID
    pub batch_id: u64,
    /// Time the batch was inserted
    pub inserted_at: DateTime<Utc>,
    /// Milliseconds since the previous batch
    pub ms_since_prev_batch: Option<u64>,
}

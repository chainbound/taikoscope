#![allow(missing_docs)]
#![allow(clippy::large_enum_variant)]
use alloy_primitives::B256;
use chainio::BatchesVerified;
use primitives::headers::{L1Header, L2Header};
use serde::{Deserialize, Serialize};

// Updated wrappers to preserve L1 transaction hash and block number metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchProposedWrapper {
    pub batch: chainio::ITaikoInbox::BatchProposed,
    pub l1_tx_hash: B256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchesProvedWrapper {
    pub proved: chainio::ITaikoInbox::BatchesProved,
    pub l1_block_number: u64,
    pub l1_tx_hash: B256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchesVerifiedWrapper {
    pub verified: BatchesVerified,
    pub l1_block_number: u64,
    pub l1_tx_hash: B256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForcedInclusionProcessedWrapper {
    pub event: chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
}

// Updated From implementations to preserve all metadata
impl From<(chainio::ITaikoInbox::BatchProposed, B256)> for BatchProposedWrapper {
    fn from(data: (chainio::ITaikoInbox::BatchProposed, B256)) -> Self {
        Self { batch: data.0, l1_tx_hash: data.1 }
    }
}

impl From<(chainio::ITaikoInbox::BatchesProved, u64, B256)> for BatchesProvedWrapper {
    fn from(data: (chainio::ITaikoInbox::BatchesProved, u64, B256)) -> Self {
        Self { proved: data.0, l1_block_number: data.1, l1_tx_hash: data.2 }
    }
}

impl From<(BatchesVerified, u64, B256)> for BatchesVerifiedWrapper {
    fn from(data: (BatchesVerified, u64, B256)) -> Self {
        Self { verified: data.0, l1_block_number: data.1, l1_tx_hash: data.2 }
    }
}

impl From<chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed>
    for ForcedInclusionProcessedWrapper
{
    fn from(data: chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed) -> Self {
        Self { event: data }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaikoEvent {
    L1Header(L1Header),
    L2Header(L2Header),
    BatchProposed(BatchProposedWrapper),
    BatchesProved(BatchesProvedWrapper),
    BatchesVerified(BatchesVerifiedWrapper),
    ForcedInclusionProcessed(ForcedInclusionProcessedWrapper),
}

impl TaikoEvent {
    pub fn dedup_id(&self) -> String {
        match self {
            Self::L1Header(h) => format!("{}:{}-l1_header", h.number, h.hash),
            Self::L2Header(h) => format!("{}:{}-l2_header", h.number, h.hash),
            Self::BatchProposed(b) => {
                let inner = &b.batch;
                format!("{}:{}-batch_proposed", inner.info.lastBlockId, b.l1_tx_hash)
            }
            Self::BatchesProved(p) => {
                let inner = &p.proved;
                let batch_id = inner.batchIds.first().copied().unwrap_or_default();
                format!("{}:{}-batches_proved", batch_id, p.l1_tx_hash)
            }
            Self::BatchesVerified(v) => {
                format!("{}:{}-batches_verified", v.verified.batch_id, v.l1_tx_hash)
            }
            Self::ForcedInclusionProcessed(f) => {
                format!("{}-forced_inclusion_processed", f.event.blobHash)
            }
        }
    }
}

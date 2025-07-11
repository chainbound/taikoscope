#![allow(missing_docs)]
#![allow(clippy::large_enum_variant)]
use chainio::BatchesVerified;
use primitives::headers::{L1Header, L2Header};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchProposedWrapper(pub chainio::ITaikoInbox::BatchProposed);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchesProvedWrapper(pub chainio::ITaikoInbox::BatchesProved);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForcedInclusionProcessedWrapper(
    pub chainio::taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaikoEvent {
    L1Header(L1Header),
    L2Header(L2Header),
    BatchProposed(BatchProposedWrapper),
    BatchesProved(BatchesProvedWrapper),
    BatchesVerified(BatchesVerified),
    ForcedInclusionProcessed(ForcedInclusionProcessedWrapper),
}

impl TaikoEvent {
    pub fn dedup_id(&self) -> String {
        match self {
            Self::L1Header(h) => format!("{}:{}-l1_header", h.number, h.hash),
            Self::L2Header(h) => format!("{}:{}-l2_header", h.number, h.hash),
            Self::BatchProposed(b) => {
                let inner = &b.0;
                format!("{}:{}-batch_proposed", inner.info.lastBlockId, inner.info.anchorBlockHash)
            }
            Self::BatchesProved(p) => {
                let inner = &p.0;
                let batch_id = inner.batchIds.first().copied().unwrap_or_default();
                let block_hash = inner
                    .transitions
                    .first()
                    .map(|t| format!("{:?}", t.blockHash))
                    .unwrap_or_default();
                format!("{}:{}-batches_proved", batch_id, block_hash)
            }
            Self::BatchesVerified(v) => {
                format!("{}:{:?}-batches_verified", v.batch_id, v.block_hash)
            }
            Self::ForcedInclusionProcessed(f) => {
                format!("{}-forced_inclusion_processed", f.0.blobHash)
            }
        }
    }
}

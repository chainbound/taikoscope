use chainio::{
    ITaikoInbox::{BatchProposed, BatchesProved},
    taiko::wrapper::ITaikoWrapper::ForcedInclusionProcessed,
};
use extractor::{L1Header, L2Header};

/// Events handled by the [`Driver`] event loop.
#[derive(Debug)]
pub enum DriverEvent {
    /// A new L1 header was received.
    L1Header(L1Header),
    /// A new L2 header was received.
    L2Header(L2Header),
    /// A new batch was proposed on L1.
    BatchProposed(BatchProposed),
    /// A forced inclusion was processed.
    ForcedInclusion(ForcedInclusionProcessed),
    /// Batches were proved on L1 together with the block number.
    BatchesProved((BatchesProved, u64)),
    /// Batches were verified on L1 together with the block number.
    BatchesVerified((chainio::BatchesVerified, u64)),
}
